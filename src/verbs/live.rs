use std::path::{Path, PathBuf};

use serde_json::json;

use crate::cli::{Globals, LiveArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::spawn::Argv;

pub fn run(args: LiveArgs, g: &Globals) -> Result<Contract, Error> {
    let scheme = args.to.split("://").next().unwrap_or("").to_lowercase();
    if !matches!(scheme.as_str(), "rtmp" | "rtmps" | "tcp" | "udp") {
        return Err(Error::input(
            "live --to needs an rtmp://, rtmps://, tcp://, or udp:// URL",
        ));
    }
    if args.test && args.list {
        return Err(Error::input("live --test and --list are exclusive"));
    }
    if args.test && args.input.is_some() {
        return Err(Error::input(
            "live --test streams a generated card — drop the input file",
        ));
    }
    if !args.test && args.input.is_none() {
        return Err(Error::input(
            "live needs an input file (or --test for the built-in card)",
        ));
    }
    if args.list && args.input.is_none() {
        return Err(Error::input("live --list needs a manifest file"));
    }

    // --list: probe the first manifest entry for stream shape (the manifest
    // itself is a text file ffprobe can't read)
    let mut list_files: Vec<PathBuf> = Vec::new();
    let (has_video, has_audio) = if args.test {
        (true, true)
    } else {
        let input = args.input.as_deref().unwrap_or_else(|| Path::new(""));
        let probe_src = if args.list {
            list_files = manifest_files(input)?;
            list_files[0].clone()
        } else {
            input.to_path_buf()
        };
        let probe = engine::probe_or_err(&probe_src, g)?;
        if !probe.has_video && !probe.has_audio {
            return Err(Error::input("live: input has no media streams"));
        }
        (probe.has_video, probe.has_audio)
    };

    let vbitrate = args.vbitrate.as_deref().unwrap_or("2500k");
    let abitrate = args.abitrate.as_deref().unwrap_or("128k");
    // FLV for RTMP/plain-TCP ingest; MPEG-TS is the container UDP expects
    let fmt = if scheme == "udp" { "mpegts" } else { "flv" };

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-re");
    if args.loop_ && !args.test {
        argv.extend(["-stream_loop", "-1"]);
    }
    if args.test {
        argv.extend([
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=1280x720:rate=30",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=1000:sample_rate=48000",
        ]);
    } else if args.list {
        argv.extend(["-f", "concat", "-safe", "0", "-i"]);
        argv.push(args.input.as_deref().unwrap_or_else(|| Path::new("")));
    } else {
        argv.push("-i");
        argv.push(args.input.as_deref().unwrap_or_else(|| Path::new("")));
    }
    if has_video {
        if let Some(sc) = &args.scale {
            let mut p = sc.split('x');
            let (w, h) = (
                p.next().and_then(|v| v.parse::<u32>().ok()),
                p.next().and_then(|v| v.parse::<u32>().ok()),
            );
            let (w, h) = match (w, h) {
                (Some(w), Some(h)) if w > 0 && h > 0 => (w, h),
                _ => return Err(Error::input("live --scale needs WxH like 1280x720")),
            };
            argv.extend(["-vf".into(), format!("scale={w}:{h}")]);
        }
        argv.extend([
            "-c:v".into(),
            "libx264".into(),
            "-preset".into(),
            "veryfast".into(),
            "-tune".into(),
            "zerolatency".into(),
            "-b:v".into(),
            vbitrate.to_string(),
            "-pix_fmt".into(),
            "yuv420p".into(),
        ]);
        if let Some(fps) = args.fps {
            argv.extend(["-r".into(), fps.to_string()]);
        }
    } else {
        argv.push("-vn");
    }
    if has_audio {
        argv.extend([
            "-c:a".into(),
            "aac".into(),
            "-b:a".into(),
            abitrate.to_string(),
        ]);
    } else {
        argv.push("-an");
    }
    if let Some(until) = args.until {
        if !until.is_finite() || until <= 0.0 {
            return Err(Error::input(
                "live --until needs a positive duration in seconds",
            ));
        }
        argv.extend(["-t".into(), until.to_string()]);
    }
    // test mode maps its two lavfi inputs explicitly (video from input 0,
    // tone from input 1); file inputs use the normal stream indexes
    let (vmap, amap) = if args.test {
        ("0:v", "1:a")
    } else {
        ("0:v", "0:a")
    };
    if let Some(rec) = &args.record {
        let input_refs: Vec<&Path> = args.input.iter().map(|p| p.as_path()).collect();
        crate::paths::ensure_output_allowed(rec, &input_refs, g.overwrite)?;
        let rec_ext = rec
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        let rec_fmt = match rec_ext.as_str() {
            "mp4" | "mov" | "m4v" => "mp4",
            "mkv" => "matroska",
            "ts" | "mts" | "m2ts" => "mpegts",
            "flv" => "flv",
            _ => {
                return Err(Error::input(
                    "live --record needs a media extension (mp4/mov/mkv/ts/flv)",
                ));
            }
        };
        // tee muxer doesn't do default stream selection — map explicitly,
        // then encode once and mux to ingest + local archive together
        if has_video {
            argv.extend(["-map", vmap]);
        }
        if has_audio {
            argv.extend(["-map", amap]);
        }
        argv.extend([
            "-f".to_string(),
            "tee".to_string(),
            format!("[f={fmt}]{}|[f={rec_fmt}]{}", args.to, rec.display()),
        ]);
    } else {
        if args.test {
            if has_video {
                argv.extend(["-map", vmap]);
            }
            if has_audio {
                argv.extend(["-map", amap]);
            }
        }
        argv.extend(["-f".into(), fmt.into(), args.to.clone()]);
    }

    let contract = stream_out("live", args.input.as_deref(), &args.to, vec![argv], g)?;
    Ok(contract.with_extra(json!({
        "to": args.to,
        "loop": args.loop_,
        "vbitrate": vbitrate,
        "abitrate": abitrate,
        "format": fmt,
        "record": args.record,
        "until": args.until,
        "list": args.list,
        "files": list_files.len(),
        "test": args.test,
    })))
}

/// Parse an ffconcat manifest into resolved file paths (validated to exist).
/// Recognizes `file 'x'` / `file "x"` / `file x` lines; `#` comments skipped.
fn manifest_files(manifest: &Path) -> Result<Vec<PathBuf>, Error> {
    let text = std::fs::read_to_string(manifest)
        .map_err(|e| Error::input(format!("live --list: {manifest:?}: {e}")))?;
    let dir = manifest.parent().unwrap_or_else(|| Path::new("."));
    let mut files = Vec::new();
    for line in text.lines() {
        let l = line.trim();
        if l.is_empty() || l.starts_with('#') {
            continue;
        }
        if let Some(rest) = l.strip_prefix("file") {
            let name = rest.trim().trim_matches(|c| c == '\'' || c == '"');
            if !name.is_empty() {
                let p = PathBuf::from(name);
                files.push(if p.is_absolute() { p } else { dir.join(p) });
            }
        }
    }
    if files.is_empty() {
        return Err(Error::input(
            "live --list: no `file …` entries in the manifest",
        ));
    }
    for f in &files {
        crate::paths::ensure_input(f)?;
    }
    Ok(files)
}

/// Stream-output shared path for verbs that push a URL instead of writing a
/// file — write_job's exists/probe verification is file-only, so a stream is
/// verified by ffmpeg's exit status alone. Honors --dry-run.
pub(crate) fn stream_out(
    tool: &str,
    input: Option<&Path>,
    to: &str,
    argvs: Vec<Argv>,
    g: &Globals,
) -> Result<Contract, Error> {
    if let Some(i) = input {
        crate::paths::ensure_input(i)?;
    }
    let inputs: Vec<&Path> = input.into_iter().collect();
    crate::paths::ensure_output_allowed(Path::new(to), &inputs, g.overwrite)?;
    let commands = engine::commands_of(&argvs);
    if g.dry_run {
        let p = input.and_then(|i| crate::probe::probe(i, std::time::Duration::from_secs(60)).ok());
        return Ok(Contract::dry_run(tool, Some(to.to_string()), p).with_commands(commands));
    }
    if let Err(e) = engine::run_argvs(&argvs, g) {
        return Ok(Contract::failed(tool, &e).with_commands(commands));
    }
    Ok(Contract::ok(tool, Some(to.to_string()), None).with_commands(commands))
}

use std::path::Path;

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
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_video && !probe.has_audio {
        return Err(Error::input("live: input has no media streams"));
    }

    let vbitrate = args.vbitrate.as_deref().unwrap_or("2500k");
    let abitrate = args.abitrate.as_deref().unwrap_or("128k");
    // FLV for RTMP/plain-TCP ingest; MPEG-TS is the container UDP expects
    let fmt = if scheme == "udp" { "mpegts" } else { "flv" };

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-re");
    if args.loop_ {
        argv.extend(["-stream_loop", "-1"]);
    }
    argv.extend(["-i".into(), args.input.display().to_string()]);
    if probe.has_video {
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
    if probe.has_audio {
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
    if let Some(rec) = &args.record {
        crate::paths::ensure_output_allowed(rec, &[&args.input], g.overwrite)?;
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
        if probe.has_video {
            argv.extend(["-map", "0:v"]);
        }
        if probe.has_audio {
            argv.extend(["-map", "0:a"]);
        }
        argv.extend([
            "-f".to_string(),
            "tee".to_string(),
            format!("[f={fmt}]{}|[f={rec_fmt}]{}", args.to, rec.display()),
        ]);
    } else {
        argv.extend(["-f".into(), fmt.into(), args.to.clone()]);
    }

    let contract = stream_out("live", &args.input, &args.to, vec![argv], g)?;
    Ok(contract.with_extra(json!({
        "to": args.to,
        "loop": args.loop_,
        "vbitrate": vbitrate,
        "abitrate": abitrate,
        "format": fmt,
        "record": args.record,
        "until": args.until,
    })))
}

/// Stream-output shared path for verbs that push a URL instead of writing a
/// file — write_job's exists/probe verification is file-only, so a stream is
/// verified by ffmpeg's exit status alone. Honors --dry-run.
pub(crate) fn stream_out(
    tool: &str,
    input: &Path,
    to: &str,
    argvs: Vec<Argv>,
    g: &Globals,
) -> Result<Contract, Error> {
    crate::paths::ensure_input(input)?;
    crate::paths::ensure_output_allowed(Path::new(to), &[input], g.overwrite)?;
    let commands = engine::commands_of(&argvs);
    if g.dry_run {
        let p = crate::probe::probe(input, std::time::Duration::from_secs(60)).ok();
        return Ok(Contract::dry_run(tool, Some(to.to_string()), p).with_commands(commands));
    }
    if let Err(e) = engine::run_argvs(&argvs, g) {
        return Ok(Contract::failed(tool, &e).with_commands(commands));
    }
    Ok(Contract::ok(tool, Some(to.to_string()), None).with_commands(commands))
}

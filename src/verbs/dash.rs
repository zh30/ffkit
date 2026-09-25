use std::path::{Path, PathBuf};

use serde_json::json;

use crate::cli::{DashArgs, Globals};
use crate::contract::Contract;
use crate::engine;
use crate::error::Error;
use crate::paths;

pub fn run(args: DashArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_video && !probe.has_audio {
        return Err(Error::input("input has no media streams"));
    }
    if !(0.5..=30.0).contains(&args.seg) {
        return Err(Error::input("--seg must be 0.5..=30 seconds"));
    }
    if args.streaming && args.sidx {
        return Err(Error::input(
            "--streaming and --sidx are exclusive (fragments vs one-file index)",
        ));
    }
    if args.sidx && !args.single {
        return Err(Error::input(
            "--sidx indexes the single byte-range file — pass --single too",
        ));
    }
    if args.sidx && args.webm {
        return Err(Error::input("--sidx needs the mp4 package (drop --webm)"));
    }
    paths::ensure_input(&args.input)?;
    if args.webm && args.copy {
        let v = probe.vcodec.as_deref().unwrap_or("");
        if probe.has_video && !matches!(v, "vp8" | "vp9") {
            return Err(Error::input(format!(
                "dash --webm --copy needs a vp8/vp9 input (found {v})"
            )));
        }
        let a = probe.acodec.as_deref().unwrap_or("");
        if probe.has_audio && !args.audio_only && !matches!(a, "opus" | "vorbis") {
            return Err(Error::input(format!(
                "dash --webm --copy needs an opus/vorbis input (found {a})"
            )));
        }
    }
    let mut hs = args.ladder.clone();
    if !hs.is_empty() {
        if args.copy {
            return Err(Error::input("--ladder doesn't combine with --copy"));
        }
        if args.audio_only {
            return Err(Error::input("--ladder doesn't combine with --audio-only"));
        }
        if args.video_only {
            return Err(Error::input("--ladder doesn't combine with --video-only"));
        }
        if !probe.has_video {
            return Err(Error::input("--ladder needs a video stream"));
        }
        hs.sort_unstable_by(|a, b| b.cmp(a));
        hs.dedup();
        if hs.len() < 2 || hs.len() > 6 {
            return Err(Error::input("--ladder needs 2..=6 heights"));
        }
    }

    // -o is the manifest name (or a directory → <dir>/manifest.mpd)
    let (dir, manifest): (PathBuf, PathBuf) = if args.output.extension().is_some() {
        let dir = args
            .output
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();
        (dir, args.output.clone())
    } else {
        (args.output.clone(), args.output.join("manifest.mpd"))
    };
    std::fs::create_dir_all(&dir)
        .map_err(|e| Error::output(format!("create {}: {e}", paths::display(&dir))))?;
    let seg_ext = if args.webm { "webm" } else { "m4s" };

    let mut argv = engine::ffmpeg_base(g.progress);
    argv.extend(["-i".to_string(), args.input.display().to_string()]);
    if args.video_only {
        if args.audio_only {
            return Err(Error::input("--video-only conflicts with --audio-only"));
        }
        if !probe.has_video {
            return Err(Error::input("--video-only needs a video stream"));
        }
        argv.extend(["-an".to_string()]);
    }
    if args.copy {
        if probe.has_video && !args.audio_only {
            argv.extend(["-c:v".to_string(), "copy".to_string()]);
        }
        if probe.has_audio && !args.video_only {
            argv.extend(["-c:a".to_string(), "copy".to_string()]);
        }
    } else if hs.is_empty() {
        if probe.has_video && !args.audio_only {
            argv.extend([
                "-vf".to_string(),
                "scale=trunc(iw/2)*2:trunc(ih/2)*2".to_string(),
            ]);
            if args.webm {
                argv.extend([
                    "-c:v".to_string(),
                    "libvpx-vp9".to_string(),
                    "-b:v".to_string(),
                    "1M".to_string(),
                    "-row-mt".to_string(),
                    "1".to_string(),
                ]);
            } else {
                argv.extend([
                    "-c:v".to_string(),
                    "libx264".to_string(),
                    "-preset".to_string(),
                    "veryfast".to_string(),
                    "-crf".to_string(),
                    "20".to_string(),
                    "-pix_fmt".to_string(),
                    "yuv420p".to_string(),
                ]);
            }
        }
        if probe.has_audio && !args.video_only {
            if args.webm {
                argv.extend([
                    "-c:a".to_string(),
                    "libopus".to_string(),
                    "-b:a".to_string(),
                    "96k".to_string(),
                ]);
            } else {
                argv.extend([
                    "-c:a".to_string(),
                    "aac".to_string(),
                    "-b:a".to_string(),
                    "128k".to_string(),
                ]);
            }
        }
        if args.audio_only {
            argv.extend(["-vn".to_string()]);
        }
    } else {
        // ABR ladder: one encode per rung → N Representations in one
        // AdaptationSet; audio rides a second AdaptationSet
        let n = hs.len();
        let bitrate = |h: u32| -> u32 {
            if h >= 1080 {
                4500
            } else if h >= 720 {
                2800
            } else if h >= 480 {
                1400
            } else if h >= 360 {
                800
            } else {
                500
            }
        };
        let mut fc = String::new();
        fc.push_str(&format!(
            "[0:v]split={n}{}",
            (0..n).map(|i| format!("[sp{i}]")).collect::<String>()
        ));
        for (i, h) in hs.iter().enumerate() {
            fc.push_str(&format!(";[sp{i}]scale=-2:{h}[lv{i}]"));
        }
        argv.extend(["-filter_complex".to_string(), fc]);
        for (i, h) in hs.iter().enumerate() {
            let r = bitrate(*h);
            argv.extend([
                "-map".to_string(),
                format!("[lv{i}]"),
                format!("-c:v:{i}"),
                if args.webm {
                    "libvpx-vp9".to_string()
                } else {
                    "libx264".to_string()
                },
                format!("-b:v:{i}"),
                format!("{r}k"),
                format!("-maxrate:v:{i}"),
                format!("{r}k"),
                format!("-bufsize:v:{i}"),
                format!("{}k", r * 2),
            ]);
            if args.webm {
                argv.extend([format!("-row-mt:v:{i}"), "1".to_string()]);
            } else {
                argv.extend([
                    format!("-pix_fmt:v:{i}"),
                    "yuv420p".to_string(),
                    format!("-preset:v:{i}"),
                    "veryfast".to_string(),
                ]);
            }
        }
        let n_streams = n;
        if probe.has_audio && !args.video_only {
            argv.extend([
                "-map".to_string(),
                "0:a".to_string(),
                "-c:a".to_string(),
                if args.webm {
                    "libopus".to_string()
                } else {
                    "aac".to_string()
                },
                "-b:a".to_string(),
                "96k".to_string(),
            ]);
        }
        let vids: Vec<String> = (0..n_streams).map(|i| i.to_string()).collect();
        let adapt = if probe.has_audio && !args.video_only {
            format!("id=0,streams={} id=1,streams={n_streams}", vids.join(","))
        } else {
            format!("id=0,streams={}", vids.join(","))
        };
        argv.extend(["-adaptation_sets".to_string(), adapt]);
    }
    argv.extend([
        "-f".to_string(),
        "dash".to_string(),
        "-seg_duration".to_string(),
        format!("{:.3}", args.seg),
        // segment names are resolved against the manifest's dir by the
        // muxer — bare filenames, no path prefix
        "-init_seg_name".to_string(),
        format!("init-$RepresentationID$.{seg_ext}"),
        "-media_seg_name".to_string(),
        format!("seg-$RepresentationID$-$Number%05d$.{seg_ext}"),
    ]);
    if args.single {
        argv.extend([
            "-single_file".to_string(),
            "1".to_string(),
            "-single_file_name".to_string(),
            format!("stream-$RepresentationID$.{seg_ext}"),
        ]);
    }
    if let Some(n) = args.window {
        argv.extend(["-window_size".to_string(), n.to_string()]);
    }
    if args.webm {
        argv.extend(["-dash_segment_type".to_string(), "webm".to_string()]);
    }
    if args.streaming {
        argv.extend(["-streaming".to_string(), "1".to_string()]);
    }
    if args.sidx {
        argv.extend(["-global_sidx".to_string(), "1".to_string()]);
    }
    argv.push(manifest.display().to_string());

    let argvs = vec![argv];
    let commands = engine::commands_of(&argvs);
    if g.dry_run {
        return Ok(
            Contract::dry_run("dash", Some(paths::display(&manifest)), Some(probe))
                .with_commands(commands),
        );
    }
    if let Err(e) = engine::run_argvs(&argvs, g) {
        return Ok(Contract::failed("dash", &e).with_commands(commands));
    }
    let nseg = std::fs::read_dir(&dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .filter(|e| {
                    let n = e.file_name().to_string_lossy().into_owned();
                    (n.starts_with("seg-") || n.starts_with("stream-"))
                        && e.path()
                            .extension()
                            .is_some_and(|x| x == "m4s" || x == "webm" || x == "mp4")
                })
                .count()
        })
        .unwrap_or(0);
    let mpd_ok = manifest.is_file()
        && std::fs::read_to_string(&manifest)
            .map(|m| m.contains("<MPD") && m.contains("<Period"))
            .unwrap_or(false);
    if nseg == 0 || !mpd_ok {
        return Err(Error::verification(
            "dash finished but no segments/manifest were written",
        ));
    }
    let c = Contract::ok(
        "dash",
        Some(paths::display(&manifest)),
        crate::probe::probe(&manifest, std::time::Duration::from_secs(60)).ok(),
    )
    .with_commands(commands);
    Ok(c.with_extra(json!({
        "manifest": paths::display(&manifest),
        "segments": nseg,
        "segment_seconds": args.seg,
        "single": args.single,
        "webm": args.webm,
        "window": args.window.unwrap_or(0),
        "ladder": hs
            .iter()
            .map(|h| format!("{h}p"))
            .collect::<Vec<String>>()
            .join(","),
    })))
}

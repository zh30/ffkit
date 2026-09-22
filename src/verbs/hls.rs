use std::path::{Path, PathBuf};

use serde_json::json;

use crate::cli::{Globals, HlsArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::paths;

pub fn run(args: HlsArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_video && !probe.has_audio {
        return Err(Error::input("input has no media streams"));
    }
    if !(0.5..=30.0).contains(&args.seg) {
        return Err(Error::input("--seg must be 0.5..=30 seconds"));
    }
    paths::ensure_input(&args.input)?;

    // -o is the playlist name (or a directory → <dir>/index.m3u8).
    let (dir, playlist): (PathBuf, PathBuf) = if args.output.extension().is_some() {
        let dir = args
            .output
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();
        (dir, args.output.clone())
    } else {
        (args.output.clone(), args.output.join("index.m3u8"))
    };
    std::fs::create_dir_all(&dir)
        .map_err(|e| Error::output(format!("create {}: {e}", paths::display(&dir))))?;
    let seg_tpl = if args.single {
        dir.join("seg.ts")
    } else {
        dir.join("seg_%03d.ts")
    };

    let mut argv = ffmpeg_base(g.progress);
    argv.extend(["-i".to_string(), args.input.display().to_string()]);
    if args.audio_only {
        if args.copy || !args.ladder.is_empty() {
            return Err(Error::input(
                "--audio-only doesn't combine with --copy/--ladder",
            ));
        }
        if !probe.has_audio {
            return Err(Error::input("--audio-only needs an audio stream"));
        }
    }
    if !args.ladder.is_empty() {
        // ABR ladder: N variants at tiered bitrates, one audio, master.m3u8.
        if args.copy || args.single {
            return Err(Error::input(
                "--ladder doesn't combine with --copy/--single",
            ));
        }
        if !probe.has_video {
            return Err(Error::input("--ladder needs a video stream"));
        }
        if args.output.extension().is_some() {
            return Err(Error::input("--ladder needs a directory -o"));
        }
        let mut hs = args.ladder.clone();
        hs.sort_unstable_by(|a, b| b.cmp(a));
        hs.dedup();
        if hs.len() < 2 || hs.len() > 6 {
            return Err(Error::input("--ladder needs 2..=6 heights"));
        }
        let n = hs.len();
        let bitrate = |h: u32| -> u32 {
            if h >= 1080 {
                4500
            } else if h >= 720 {
                2800
            } else if h >= 480 {
                1400
            } else {
                800
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
                "libx264".to_string(),
                format!("-preset:v:{i}"),
                "veryfast".to_string(),
                format!("-b:v:{i}"),
                format!("{r}k"),
                format!("-maxrate:v:{i}"),
                format!("{r}k"),
                format!("-bufsize:v:{i}"),
                format!("{}k", r * 2),
                format!("-pix_fmt:v:{i}"),
                "yuv420p".to_string(),
            ]);
        }
        if probe.has_audio {
            // ffmpeg <7 hls: an elementary stream may appear in only one
            // variant group — so each variant gets its own aac encode.
            for i in 0..n {
                argv.extend([
                    "-map".to_string(),
                    "0:a".to_string(),
                    format!("-c:a:{i}"),
                    "aac".to_string(),
                    format!("-b:a:{i}"),
                    "128k".to_string(),
                ]);
            }
        }
        let varmap = if probe.has_audio {
            (0..n)
                .map(|i| format!("v:{i},a:{i}"))
                .collect::<Vec<_>>()
                .join(" ")
        } else {
            (0..n)
                .map(|i| format!("v:{i}"))
                .collect::<Vec<_>>()
                .join(" ")
        };
        argv.extend([
            "-f".to_string(),
            "hls".to_string(),
            "-hls_time".to_string(),
            format!("{:.3}", args.seg),
            "-hls_playlist_type".to_string(),
            "vod".to_string(),
            "-hls_segment_filename".to_string(),
            dir.join("seg_%v_%03d.ts").display().to_string(),
            "-master_pl_name".to_string(),
            "master.m3u8".to_string(),
            "-var_stream_map".to_string(),
            varmap,
        ]);
        argv.push(dir.join("v%v.m3u8").display().to_string());

        let commands = engine::commands_of(std::slice::from_ref(&argv));
        if g.dry_run {
            return Ok(Contract::dry_run(
                "hls",
                Some(paths::display(&dir.join("master.m3u8"))),
                Some(probe),
            )
            .with_commands(commands));
        }
        if let Err(e) = engine::run_argvs(&[argv], g) {
            return Ok(Contract::failed("hls", &e).with_commands(commands));
        }
        let master = dir.join("master.m3u8");
        if !master.is_file() {
            return Err(Error::verification(
                "hls --ladder finished but master.m3u8 is missing",
            ));
        }
        let variants = hs
            .iter()
            .enumerate()
            .map(|(i, h)| format!("v{i}.m3u8 ({h}p)"))
            .collect::<Vec<_>>();
        let mut c = Contract::ok(
            "hls",
            Some(paths::display(&master)),
            crate::probe::probe(&master, std::time::Duration::from_secs(60)).ok(),
        )
        .with_commands(commands);
        c = c.with_extra(json!({
            "playlist": paths::display(&master),
            "variants": variants,
            "segment_seconds": args.seg,
        }));
        return Ok(c);
    }
    if args.copy {
        argv.extend([
            "-c:v".to_string(),
            "copy".to_string(),
            "-bsf:v".to_string(),
            "h264_mp4toannexb".to_string(),
        ]);
        if probe.has_audio {
            argv.extend(["-c:a".to_string(), "copy".to_string()]);
        }
    } else {
        if probe.has_video && !args.audio_only {
            argv.extend([
                "-vf".to_string(),
                "scale=trunc(iw/2)*2:trunc(ih/2)*2".to_string(),
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
        if probe.has_audio {
            argv.extend([
                "-c:a".to_string(),
                "aac".to_string(),
                "-b:a".to_string(),
                "128k".to_string(),
            ]);
        }
        if args.audio_only {
            argv.extend(["-vn".to_string()]);
        }
    }
    argv.extend([
        "-f".to_string(),
        "hls".to_string(),
        "-hls_time".to_string(),
        format!("{:.3}", args.seg),
        "-hls_playlist_type".to_string(),
        "vod".to_string(),
        "-hls_segment_filename".to_string(),
        seg_tpl.display().to_string(),
    ]);
    if args.single {
        argv.extend(["-hls_flags".to_string(), "single_file".to_string()]);
    }
    argv.push(playlist.display().to_string());

    // Outputs are a dir of segments + a playlist — manual contract like split.
    let commands = engine::commands_of(std::slice::from_ref(&argv));
    if g.dry_run {
        return Ok(
            Contract::dry_run("hls", Some(paths::display(&playlist)), Some(probe))
                .with_commands(commands),
        );
    }
    if let Err(e) = engine::run_argvs(&[argv], g) {
        return Ok(Contract::failed("hls", &e).with_commands(commands));
    }
    let nseg = std::fs::read_dir(&dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .filter(|e| {
                    let n = e.file_name().to_string_lossy().into_owned();
                    (n.starts_with("seg_") || n == "seg.ts")
                        && e.path().extension().is_some_and(|x| x == "ts")
                })
                .count()
        })
        .unwrap_or(0);
    if nseg == 0 || !playlist.is_file() {
        return Err(Error::verification(
            "hls finished but no segments/playlist were written",
        ));
    }
    let mut c = Contract::ok(
        "hls",
        Some(paths::display(&playlist)),
        crate::probe::probe(&playlist, std::time::Duration::from_secs(60)).ok(),
    )
    .with_commands(commands);
    c = c.with_extra(json!({
        "playlist": paths::display(&playlist),
        "segments": nseg,
        "segment_seconds": args.seg,
    }));
    Ok(c)
}

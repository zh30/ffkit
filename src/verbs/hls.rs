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
    let seg_tpl = dir.join("seg_%03d.ts");

    let mut argv = ffmpeg_base(g.progress);
    argv.extend(["-i".to_string(), args.input.display().to_string()]);
    if probe.has_video {
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
                    e.file_name().to_string_lossy().starts_with("seg_")
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

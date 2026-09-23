use std::path::{Path, PathBuf};

use serde_json::json;

use crate::cli::{Globals, SpriteArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::paths;

fn vtt_ts(t: f64) -> String {
    let ms = (t.max(0.0) * 1000.0).round() as u64;
    format!(
        "{:02}:{:02}:{:02}.{:03}",
        ms / 3_600_000,
        (ms / 60_000) % 60,
        (ms / 1_000) % 60,
        ms % 1_000
    )
}

fn sheet_path(output: &Path, i: u64) -> PathBuf {
    let stem = output
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "sprite".to_string());
    output.with_file_name(format!("{stem}-{i}.jpg"))
}

/// Seek-preview sprite: `name-1.jpg..name-N.jpg` tile sheets plus a WebVTT
/// cue file pointing each thumbnail at its `#xywh` cell. Players (Video.js,
/// Plyr, JW) show the right frame on the progress-bar hover.
pub fn run(args: SpriteArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "sprite")?;
    if !(args.every.is_finite() && args.every > 0.0) {
        return Err(Error::input("--every must be positive seconds"));
    }
    if args.cols == 0 || args.rows == 0 || args.cols * args.rows > 100 {
        return Err(Error::input("--cols x --rows must be 1..=100 tiles"));
    }
    let w = probe.width.unwrap_or(1280);
    let h = probe.height.unwrap_or(720);
    let tw = crate::paths::even(args.width.max(16));
    let th = crate::paths::even((tw as u64 * h as u64 / w as u64).max(2) as u32);
    let per = (args.cols * args.rows) as u64;
    let n = (probe.duration / args.every).ceil().max(1.0) as u64;
    let sheets = n.div_ceil(per);

    let first = sheet_path(&args.output, 1);
    paths::ensure_input(&args.input)?;
    paths::ensure_output_allowed(&first, &[&args.input], g.overwrite)?;
    let vtt_path = args
        .vtt
        .clone()
        .unwrap_or_else(|| args.output.with_extension("vtt"));
    paths::ensure_output_allowed(&vtt_path, &[&args.input], g.overwrite)?;

    let pattern = args
        .output
        .with_file_name(format!(
            "{}-%d.jpg",
            args.output
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "sprite".to_string())
        ))
        .to_string_lossy()
        .into_owned();
    let vf = format!(
        "fps={fps:.6},scale={tw}:{th},tile={c}x{r}:margin=0:padding=0",
        fps = 1.0 / args.every,
        c = args.cols,
        r = args.rows,
    );
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend([
        "-vf",
        &vf,
        "-frames:v",
        &sheets.to_string(),
        "-q:v",
        "3",
        &pattern,
    ]);
    let commands = engine::commands_of(&[argv.clone()]);
    if g.dry_run {
        return Ok(
            Contract::dry_run("sprite", Some(paths::display(&first)), Some(probe))
                .with_commands(commands),
        );
    }
    if let Err(e) = engine::run_argvs(&[argv], g) {
        return Ok(Contract::failed("sprite", &e).with_commands(commands));
    }

    let mut files = Vec::new();
    for i in 1..=sheets {
        let p = sheet_path(&args.output, i);
        if p.exists() {
            files.push(p);
        }
    }
    if files.is_empty() {
        return Err(Error::verification(format!(
            "sprite wrote no sheets matching {pattern}"
        )));
    }

    // WebVTT: one cue per thumbnail → its sheet file + #xywh cell.
    let mut vtt = String::from("WEBVTT\n\n");
    for k in 0..n {
        let s = k / per;
        let idx = k % per;
        let x = (idx % args.cols as u64) * tw as u64;
        let y = (idx / args.cols as u64) * th as u64;
        let name = sheet_path(&args.output, s + 1)
            .file_name()
            .map(|f| f.to_string_lossy().into_owned())
            .unwrap_or_default();
        vtt.push_str(&format!(
            "{} --> {}\n{}#xywh={},{},{},{}\n\n",
            vtt_ts(k as f64 * args.every),
            vtt_ts(((k + 1) as f64 * args.every).min(probe.duration)),
            name,
            x,
            y,
            tw,
            th
        ));
    }
    std::fs::write(&vtt_path, vtt).map_err(|e| Error::output(e.to_string()))?;

    let names: Vec<String> = files.iter().map(|p| paths::display(p)).collect();
    let c = Contract::ok("sprite", Some(paths::display(&vtt_path)), Some(probe))
        .with_commands(commands)
        .with_extra(json!({
            "every": args.every,
            "thumbs": n,
            "sheets": names,
            "tile": [tw, th],
        }));
    Ok(c)
}

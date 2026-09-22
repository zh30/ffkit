use std::path::Path;

use crate::cli::{Globals, OverlayArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: OverlayArgs, g: &Globals) -> Result<Contract, Error> {
    let overlay = match (&args.image, &args.video) {
        (Some(p), None) | (None, Some(p)) => p,
        (Some(_), Some(_)) => {
            return Err(Error::input("pass only one of --image or --video"));
        }
        (None, None) => {
            return Err(Error::input("overlay needs --image or --video"));
        }
    };
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "overlay")?;

    let (x, y) = if args.x.is_some() || args.y.is_some() {
        (
            args.x.clone().unwrap_or_else(|| "0".into()),
            args.y.clone().unwrap_or_else(|| "0".into()),
        )
    } else {
        overlay_xy(&args.position, args.margin)?
    };
    if args.tile > 0 {
        let overlay_path = overlay.to_path_buf();
        return tiled(args, overlay_path, &probe, g);
    }

    if args.dur.is_some() && args.at.is_none() {
        return Err(Error::input("--dur needs --at"));
    }
    let (at_secs, end_secs) = match &args.at {
        Some(s) => {
            let at = crate::time::parse_time(s)?;
            if at < 0.0 || at >= probe.duration {
                return Err(Error::input("--at must land inside the input"));
            }
            let end = match args.dur {
                Some(d) => (at + d).min(probe.duration),
                None => probe.duration,
            };
            (at, end)
        }
        None => (0.0, probe.duration),
    };
    let enable = if args.at.is_some() {
        format!(":enable='between(t,{at_secs:.3},{end_secs:.3})'")
    } else {
        String::new()
    };
    let fade = args.fade.clamp(0.0, ((end_secs - at_secs) / 2.0).max(0.0));
    if fade > 0.0 && args.tile > 0 {
        return Err(Error::input("--fade is not supported with --tile"));
    }
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    if fade > 0.0 && args.image.is_some() {
        // Looping the still gives it advancing pts so alpha fades animate.
        argv.extend(["-loop", "1", "-framerate", "30"]);
    }
    argv.push("-i");
    argv.push(overlay);
    let (pre, ovl) = if fade > 0.0 {
        (
            format!(
                "[1:v]format=rgba,fade=t=in:st={at_secs:.3}:d={fade:.3}:alpha=1,fade=t=out:st={:.3}:d={fade:.3}:alpha=1[ovl];",
                end_secs - fade
            ),
            "ovl",
        )
    } else if args.image.is_some() && args.opacity < 1.0 && args.mode.is_none() {
        // subtle watermark: scale + alpha on the still itself
        let op = args.opacity.clamp(0.0, 1.0);
        (
            format!("[1:v]format=rgba,colorchannelmixer=aa={op:.3}[ovl];"),
            "ovl",
        )
    } else {
        (String::new(), "1:v")
    };
    // Infinite looped still secondary: end each composite on the main stream.
    let shortest = if fade > 0.0 && args.image.is_some() {
        ":shortest=1"
    } else {
        ""
    };

    let fc = if let Some(mode) = &args.mode {
        const MODES: &[&str] = &[
            "screen",
            "addition",
            "multiply",
            "lighten",
            "darken",
            "overlay",
            "difference",
        ];
        if !MODES.contains(&mode.as_str()) {
            return Err(Error::input(format!(
                "unknown --mode {mode}; use {MODES:?}"
            )));
        }
        if args.scale.is_some() {
            return Err(Error::input("--mode covers the whole frame; drop --scale"));
        }
        if args.x.is_some() || args.y.is_some() {
            return Err(Error::input("--mode covers the whole frame; drop --x/--y"));
        }
        let (w, h) = (probe.width.unwrap_or(1280), probe.height.unwrap_or(720));
        format!(
            "[{ovl}]scale={w}:{h},format=yuv420p[b];             [0:v][b]blend=all_mode={mode}:all_opacity={op}{en}:shortest=1[vout]",
            op = args.opacity.clamp(0.0, 1.0),
            en = enable,
        )
    } else if let Some(scale) = args.scale {
        format!("[{ovl}]scale={scale}:-1[ov];[0:v][ov]overlay=x={x}:y={y}{enable}{shortest}[vout]")
    } else {
        format!("[0:v][{ovl}]overlay=x={x}:y={y}{enable}{shortest}[vout]")
    };
    let fc = format!("{pre}{fc}");
    let _ = x;
    let _ = y;
    argv.extend(["-filter_complex", &fc, "-map", "[vout]"]);
    if probe.has_audio {
        argv.extend(["-map", "0:a?", "-c:a", "copy"]);
    }
    argv.extend(["-c:v", "libx264", "-preset", "fast", "-crf", "18"]);
    argv.push(&args.output);

    let inputs: Vec<&Path> = vec![&args.input, &overlay];
    engine::write_job("overlay", &inputs, &args.output, vec![argv], g)
}

fn overlay_xy(pos: &str, margin: i32) -> Result<(String, String), Error> {
    let m = margin;
    let (x, y) = match pos {
        "top-left" => (format!("{m}"), format!("{m}")),
        "top" => ("(W-w)/2".into(), format!("{m}")),
        "top-right" => (format!("W-w-{m}"), format!("{m}")),
        "left" => (format!("{m}"), "(H-h)/2".into()),
        "center" => ("(W-w)/2".into(), "(H-h)/2".into()),
        "right" => (format!("W-w-{m}"), "(H-h)/2".into()),
        "bottom-left" => (format!("{m}"), format!("H-h-{m}")),
        "bottom" => ("(W-w)/2".into(), format!("H-h-{m}")),
        "bottom-right" => (format!("W-w-{m}"), format!("H-h-{m}")),
        other => {
            return Err(Error::input(format!(
                "unknown --position {other}; use top-left, top, top-right, left, center, right, bottom-left, bottom, bottom-right"
            )));
        }
    };
    Ok((x, y))
}

/// Tiled draft watermark: N scaled copies chained as overlay passes in a
/// diagonal/stepped pattern — one logo per pass (cheap up to ~6).
fn tiled(
    args: OverlayArgs,
    overlay: std::path::PathBuf,
    probe: &crate::probe::Probe,
    g: &Globals,
) -> Result<Contract, Error> {
    let n = args.tile.clamp(2, 6);
    let w = probe.width.unwrap_or(1280) as i64;
    let h = probe.height.unwrap_or(720) as i64;
    let scale = args.scale.unwrap_or(320);
    // ffmpeg 4.4 consumes a pad label on first use — split into N copies.
    if args.dur.is_some() && args.at.is_none() {
        return Err(Error::input("--dur needs --at"));
    }
    let enable = match &args.at {
        Some(s) => {
            let at = crate::time::parse_time(s)?;
            if at < 0.0 || at >= probe.duration {
                return Err(Error::input("--at must land inside the input"));
            }
            let end = match args.dur {
                Some(d) => (at + d).min(probe.duration),
                None => probe.duration,
            };
            format!(":enable='between(t,{at:.3},{end:.3})'")
        }
        None => String::new(),
    };
    let ovs: String = (0..n).map(|i| format!("[ov{i}]")).collect();
    let mut seg: Vec<String> = vec![format!(
        "[1:v]scale={scale}:-1,format=rgba,colorchannelmixer=aa=0.5,split={n}{ovs}"
    )];
    let mut prev = "[0:v]".to_string();
    for i in 0..n {
        // Diagonal cascade: each copy offset by its index
        let fx = i as f64 / n as f64;
        let fy = (i as f64 + 0.5) / n as f64;
        let x = format!("{}-overlay_w/2", (fx * w as f64) as i64);
        let y = format!("{}-overlay_h/2", (fy * h as f64) as i64);
        let lab = if i + 1 == n {
            "vout".to_string()
        } else {
            format!("t{i}")
        };
        let en = if i + 1 == n { enable.as_str() } else { "" };
        seg.push(format!("{prev}[ov{i}]overlay=x={x}:y={y}{en}[{lab}]"));
        prev = format!("[{lab}]");
    }
    let fc = seg.join(";");

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.push("-i");
    argv.push(&overlay);
    argv.extend(["-filter_complex", &fc, "-map", "[vout]"]);
    if probe.has_audio {
        argv.extend(["-map", "0:a?", "-c:a", "copy"]);
    }
    argv.extend([
        "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
    ]);
    argv.push(&args.output);

    let inputs: Vec<&Path> = vec![&args.input, &overlay];
    let c = engine::write_job("overlay", &inputs, &args.output, vec![argv], g)?;
    Ok(c.with_extra(serde_json::json!({ "tile": n })))
}

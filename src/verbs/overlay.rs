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

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.push("-i");
    argv.push(overlay);

    let fc = if let Some(scale) = args.scale {
        format!("[1:v]scale={scale}:-1[ov];[0:v][ov]overlay=x={x}:y={y}[vout]")
    } else {
        format!("[0:v][1:v]overlay=x={x}:y={y}[vout]")
    };
    argv.extend(["-filter_complex", &fc, "-map", "[vout]"]);
    if probe.has_audio {
        argv.extend(["-map", "0:a?", "-c:a", "copy"]);
    }
    argv.extend(["-c:v", "libx264", "-preset", "fast", "-crf", "18"]);
    argv.push(&args.output);

    let inputs: Vec<&Path> = vec![&args.input, overlay];
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

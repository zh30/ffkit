use serde_json::json;

use crate::cli::{Globals, ShearArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: ShearArgs, g: &Globals) -> Result<Contract, Error> {
    for (name, v) in [("--x", args.x), ("--y", args.y)] {
        if !(-2.0..=2.0).contains(&v) {
            return Err(Error::input(format!("{name} must be -2..=2")));
        }
    }
    let interp = match args.interp.as_str() {
        "nearest" => "nearest",
        "bilinear" => "bilinear",
        other => {
            return Err(Error::input(format!(
                "--interp must be nearest|bilinear, got {other}"
            )))
        }
    };
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "shear")?;

    // shear transform: italic-style slant — x skews rows, y skews columns;
    // fillcolor fills the vacated edge
    crate::color::rgb(&args.fill)?;
    let fill = crate::color::lavfi(&args.fill);
    let chain = format!(
        "shear=shx={:.4}:shy={:.4}:fillcolor={}:interp={}",
        args.x, args.y, fill, interp
    );
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    match &args.at {
        Some(s) => {
            let en = crate::time::enable_expr(s, args.dur, probe.duration)?.replace("(t,", "(T,");
            let fc =
                format!("[0:v]split[m][f];[f]{chain}[p];[m][p]blend=all_expr='if({en},B,A)'[v]");
            argv.extend(["-filter_complex", &fc, "-map", "[v]", "-map", "0:a?"]);
        }
        None => {
            if args.dur.is_some() {
                return Err(Error::input("--dur needs --at"));
            }
            argv.extend(["-vf", &chain]);
        }
    }
    argv.extend([
        "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
    ]);
    if probe.has_audio {
        argv.extend(["-c:a", "copy"]);
    }
    argv.push(&args.output);

    let c = engine::write_job("shear", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "x": args.x,
        "y": args.y,
        "interp": interp,
        "fill": fill,
    })))
}

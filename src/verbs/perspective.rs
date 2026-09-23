use serde_json::json;

use crate::cli::{Globals, PerspectiveArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

/// Keystone/deskew fix: the 4 corners of a skewed quad (a filmed screen, a
/// tilted whiteboard) in the source get stretched onto the output rectangle.
/// --points order is top-left, top-right, bottom-left, bottom-right.
pub fn run(args: PerspectiveArgs, g: &Globals) -> Result<Contract, Error> {
    let pts: Vec<f64> = args
        .points
        .split(',')
        .map(|p| p.trim().parse::<f64>())
        .collect::<Result<_, _>>()
        .map_err(|_| Error::input("--points needs 8 numbers: x0,y0,x1,y1,x2,y2,x3,y3 (px)"))?;
    if pts.len() != 8 || pts.iter().any(|v| *v < 0.0) {
        return Err(Error::input(
            "--points needs 8 numbers: x0,y0,x1,y1,x2,y2,x3,y3 (px)",
        ));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "perspective")?;
    for (i, v) in pts.iter().enumerate() {
        let lim = if i % 2 == 0 {
            probe.width.unwrap_or(0)
        } else {
            probe.height.unwrap_or(0)
        } as f64;
        if *v > lim {
            return Err(Error::input(format!(
                "--points {i} ({v}) exceeds the {} frame",
                if i % 2 == 0 { "width" } else { "height" }
            )));
        }
    }
    let interp = match args.interp.as_str() {
        "linear" | "cubic" => args.interp.as_str(),
        _ => return Err(Error::input("--interp: linear | cubic")),
    };
    let vf = format!(
        "perspective=x0={:.1}:y0={:.1}:x1={:.1}:y1={:.1}:x2={:.1}:y2={:.1}:x3={:.1}:y3={:.1}:sense=0:interpolation={}",
        pts[0], pts[1], pts[2], pts[3], pts[4], pts[5], pts[6], pts[7], interp
    );
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-vf", &vf]);
    if probe.has_audio {
        argv.extend(["-c:a", "copy"]);
    }
    argv.extend(["-c:v", "libx264", "-preset", "fast", "-crf", "18"]);
    argv.push(&args.output);

    let c = engine::write_job("perspective", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({ "points": args.points, "interp": interp })))
}

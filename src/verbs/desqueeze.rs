use serde_json::json;

use crate::cli::{DesqueezeArgs, Globals};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: DesqueezeArgs, g: &Globals) -> Result<Contract, Error> {
    if !(1.01..=2.0).contains(&args.factor) {
        return Err(Error::input(
            "--factor must be 1.01..=2.0 (anamorphic ratio)",
        ));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "desqueeze")?;
    let (w, h) = (
        probe.width.unwrap_or(0) as f64,
        probe.height.unwrap_or(0) as f64,
    );

    // anamorphic restore: stretch one axis by the lens factor
    let (ow, oh) = match args.axis.as_str() {
        "x" => (((w * args.factor) as u32) & !1, h as u32 & !1),
        _ => (w as u32 & !1, ((h * args.factor) as u32) & !1),
    };

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    let vf = format!("scale={ow}:{oh}:flags=lanczos");
    argv.extend(["-vf", &vf]);
    argv.extend([
        "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
    ]);
    if probe.has_audio {
        argv.extend(["-c:a", "copy"]);
    }
    argv.push(&args.output);

    let c2 = engine::write_job("desqueeze", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c2.with_extra(json!({
        "factor": args.factor,
        "axis": args.axis,
        "out": format!("{ow}x{oh}"),
    })))
}

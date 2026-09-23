use serde_json::json;

use crate::cli::{Globals, UpscaleArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

/// Up-res old or phone footage: zscale's spline36 kernel reconstructs detail
/// better than bilinear/lanczos, then a light unsharp restores perceived edge
/// acuity. --factor is the linear multiplier (2 = double width AND height).
pub fn run(args: UpscaleArgs, g: &Globals) -> Result<Contract, Error> {
    if !(1.05..=4.0).contains(&args.factor) {
        return Err(Error::input("--factor must be 1.05..=4"));
    }
    if !(0.0..=1.0).contains(&args.strength) {
        return Err(Error::input("--strength must be 0..=1"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "upscale")?;
    let mut vf = format!(
        "zscale=w=trunc(iw*{:.4}/2)*2:h=trunc(ih*{:.4}/2)*2:filter=spline36",
        args.factor, args.factor
    );
    if args.strength > 0.0 {
        vf.push_str(&format!(",unsharp=5:5:{:.2}", args.strength));
    }
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-vf", &vf]);
    if probe.has_audio {
        argv.extend(["-c:a", "copy"]);
    }
    argv.extend([
        "-c:v", "libx264", "-preset", "slow", "-crf", "18", "-pix_fmt", "yuv420p",
    ]);
    argv.push(&args.output);

    let c = engine::write_job("upscale", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "factor": args.factor,
        "strength": args.strength,
        "filter": vf,
    })))
}

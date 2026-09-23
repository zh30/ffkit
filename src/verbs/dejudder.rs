use crate::cli::DejudderArgs;
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use serde_json::json;

/// Remove pullup judder from frame-rate-converted footage (3:2 pulldown
/// wobble, PAL→NTSC speedups): dejudder evens out repeated-field timing.
pub fn run(args: DejudderArgs, g: &crate::cli::Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "dejudder")?;
    let cycle = args.cycle.clamp(2, 240);
    let vf = format!("dejudder=cycle={cycle}");
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend([
        "-vf", &vf, "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
    ]);
    if probe.has_audio {
        argv.extend(["-c:a", "copy"]);
    }
    argv.push(&args.output);
    let c = engine::write_job("dejudder", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({ "cycle": cycle })))
}

use serde_json::json;

use crate::cli::{Globals, ZoomArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::paths;

pub fn run(args: ZoomArgs, g: &Globals) -> Result<Contract, Error> {
    if !(1.05..=3.0).contains(&args.factor) {
        return Err(Error::input(
            "--factor must be 1.05..=3 (1.25 = mild punch-in)",
        ));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "zoom")?;
    let w = paths::even(probe.width.unwrap_or(1280)).max(2);
    let h = paths::even(probe.height.unwrap_or(720)).max(2);
    let sw = paths::even(((w as f64) * args.factor).round() as u32).max(w + 2);
    let sh = paths::even(((h as f64) * args.factor).round() as u32).max(h + 2);
    let vf = format!("scale={sw}:{sh},crop={w}:{h},setsar=1");

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

    let c = engine::write_job("zoom", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "factor": args.factor,
        "frame": format!("{w}x{h}"),
    })))
}

use serde_json::json;

use crate::cli::{GlitchArgs, Globals};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: GlitchArgs, g: &Globals) -> Result<Contract, Error> {
    if !(0.5..=20.0).contains(&args.strength) {
        return Err(Error::input("--strength must be 0.5..=20"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "glitch")?;

    // rgb channel offset + temporal noise = datamosh-style glitch
    let s = args.strength.round() as i32;
    let nz = (args.strength * 4.0).round() as i32;
    let vf =
        format!("format=rgba,rgbashift=rh={s}:bh=-{s},noise=alls={nz}:allf=t+u,format=yuv420p");

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

    let c = engine::write_job("glitch", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "strength": args.strength,
        "filter": "rgbashift+noise",
    })))
}

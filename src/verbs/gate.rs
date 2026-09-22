use serde_json::json;

use crate::cli::{GateArgs, Globals};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: GateArgs, g: &Globals) -> Result<Contract, Error> {
    if !(-80.0..=0.0).contains(&args.threshold) {
        return Err(Error::input("--threshold must be -80..=0 dB"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_audio {
        return Err(Error::input("gate: input has no audio stream"));
    }

    // agate threshold is linear amplitude, creators think in dB.
    let lin = 10f64.powf(args.threshold / 20.0);
    let af = format!(
        "agate=threshold={lin:.6}:ratio={r}:attack={a}:release={rel}:makeup=2",
        r = args.ratio,
        a = args.attack,
        rel = args.release,
    );
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-af", &af]);
    if probe.has_video {
        argv.extend(["-c:v", "copy"]);
    }
    argv.extend(["-c:a", "aac"]);
    argv.push(&args.output);

    let c = engine::write_job("gate", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "threshold_db": args.threshold,
        "ratio": args.ratio,
    })))
}

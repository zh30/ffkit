use serde_json::json;

use crate::cli::{Globals, TempoArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: TempoArgs, g: &Globals) -> Result<Contract, Error> {
    if !(0.5..=8.0).contains(&args.factor) {
        return Err(Error::input("--factor must be 0.5..=8"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_audio {
        return Err(Error::input("tempo: input has no audio stream"));
    }
    if probe.has_video {
        return Err(Error::input(
            "tempo retimes audio only — use `speed` to retime video",
        ));
    }

    // atempo accepts at most 2x per instance on old ffmpeg — chain segments.
    let mut af = String::new();
    let mut f = args.factor;
    while f > 2.0 {
        af.push_str("atempo=2.0,");
        f /= 2.0;
    }
    while f < 0.5 {
        af.push_str("atempo=0.5,");
        f /= 0.5;
    }
    af.push_str(&format!("atempo={f:.4}"));

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-af", &af]);
    if probe.has_video {
        argv.extend(["-c:v", "copy"]);
    }
    argv.extend(["-c:a", "aac"]);
    argv.push(&args.output);

    let c = engine::write_job("tempo", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "factor": args.factor,
        "duration": probe.duration / args.factor,
    })))
}

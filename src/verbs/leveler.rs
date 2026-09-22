use serde_json::json;

use crate::cli::{Globals, LevelerArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: LevelerArgs, g: &Globals) -> Result<Contract, Error> {
    if !(-60.0..=0.0).contains(&args.threshold) {
        return Err(Error::input("--threshold must be -60..=0 dB"));
    }
    if !(1.0..=20.0).contains(&args.ratio) {
        return Err(Error::input("--ratio must be 1..=20"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_audio {
        return Err(Error::input("leveler: input has no audio stream"));
    }

    let af = format!(
        "acompressor=threshold={t}dB:ratio={r}:attack={a}:release={rel}:makeup={m}dB",
        t = args.threshold,
        r = args.ratio,
        a = args.attack,
        rel = args.release,
        m = args.makeup,
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

    let c = engine::write_job("leveler", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "threshold_db": args.threshold,
        "ratio": args.ratio,
        "makeup_db": args.makeup,
    })))
}

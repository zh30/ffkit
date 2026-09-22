use serde_json::json;

use crate::cli::{CrossfadeArgs, Globals};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::paths;

pub fn run(args: CrossfadeArgs, g: &Globals) -> Result<Contract, Error> {
    if args.dur <= 0.0 {
        return Err(Error::input("--dur must be > 0"));
    }
    paths::ensure_input(&args.second)?;
    let a = engine::probe_or_err(&args.input, g)?;
    let b = engine::probe_or_err(&args.second, g)?;
    if !a.has_audio || !b.has_audio {
        return Err(Error::input("crossfade needs audio in both inputs"));
    }
    if args.dur * 2.0 > a.duration + b.duration {
        return Err(Error::input("--dur is longer than the inputs"));
    }

    // acrossfade overlaps the last d seconds of A with the first d of B.
    let dur = args.dur;
    let fc = format!("[0:a][1:a]acrossfade=d={dur:.3}[aout]");
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.push("-i");
    argv.push(&args.second);
    argv.extend(["-filter_complex", &fc, "-map", "[aout]", "-c:a", "aac"]);
    argv.push(&args.output);

    let c = engine::write_job(
        "crossfade",
        &[&args.input, &args.second],
        &args.output,
        vec![argv],
        g,
    )?;
    Ok(c.with_extra(json!({
        "dur": dur,
        "total": a.duration + b.duration - dur,
    })))
}

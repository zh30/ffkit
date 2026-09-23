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

    if args.dur.is_some() && args.at.is_none() {
        return Err(Error::input("--dur requires --at"));
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
    match &args.at {
        Some(raw) => {
            let at = crate::time::resolve_at(raw, args.dur, probe.duration)?;
            if at >= probe.duration - 0.05 {
                return Err(Error::input("--at is past the end of the input"));
            }
            let end = (at + args.dur.unwrap_or(probe.duration - at)).min(probe.duration);
            // head + retempoed mid + tail
            let fc = format!(
                "[0:a]atrim=0:{at:.3},asetpts=PTS-STARTPTS[h];\
                 [0:a]atrim={at:.3}:{end:.3},asetpts=PTS-STARTPTS,{af}[m];\
                 [0:a]atrim={end:.3}:,asetpts=PTS-STARTPTS[tl];\
                 [h][m][tl]concat=n=3:v=0:a=1[aout]"
            );
            argv.extend(["-filter_complex", &fc, "-map", "[aout]"]);
        }
        None => {
            argv.extend(["-af", &af]);
        }
    }
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

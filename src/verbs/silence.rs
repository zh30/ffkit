use serde_json::json;

use crate::cli::{Globals, SilenceArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: SilenceArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_audio {
        return Err(Error::input("silence: input has no audio stream"));
    }
    if args.detect {
        let threshold = args.threshold.unwrap_or(-35.0);
        let min = args.min.unwrap_or(0.4);
        let ranges = crate::silence::detect(&args.input, threshold, min, g.timeout, false)?;
        return Ok(
            Contract::ok("silence", None, Some(probe)).with_extra(json!({
                "detect": true,
                "threshold_db": threshold,
                "min_duration": min,
                "ranges": ranges
                    .iter()
                    .map(|(s, e)| json!({ "start": s, "end": e, "duration": e - s }))
                    .collect::<Vec<_>>(),
            })),
        );
    }
    let dur = args
        .dur
        .ok_or_else(|| Error::input("silence needs --dur unless --detect"))?;
    let output = args
        .output
        .as_ref()
        .ok_or_else(|| Error::input("silence needs -o unless --detect"))?;
    if probe.has_video {
        return Err(Error::input(
            "silence pads audio only — use `freeze` to hold video frames",
        ));
    }
    let at = if args.end {
        probe.duration
    } else {
        args.at.unwrap_or(0.0)
    };
    if dur <= 0.0 {
        return Err(Error::input("--dur must be > 0"));
    }
    if at < 0.0 {
        return Err(Error::input("--at must be >= 0"));
    }

    // anullsrc must match the input layout for concat to chain.
    let cl = if probe.channels.unwrap_or(2) == 1 {
        "mono"
    } else {
        "stereo"
    };
    let sr = probe.sample_rate.unwrap_or(48000);
    let fc = format!(
        "[0:a]atrim=0:{at:.3},asetpts=PTS-STARTPTS[a0];\
         anullsrc=r={sr}:cl={cl},atrim=0:{dur:.3},asetpts=PTS-STARTPTS[a1];\
         [0:a]atrim={at:.3},asetpts=PTS-STARTPTS[a2];\
         [a0][a1][a2]concat=n=3:v=0:a=1[aout]"
    );
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-filter_complex", &fc, "-map", "[aout]", "-c:a", "aac"]);
    argv.push(output);

    let c = engine::write_job("silence", &[&args.input], output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "at": at,
        "dur": dur,
    })))
}

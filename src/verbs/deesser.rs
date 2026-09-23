use serde_json::json;

use crate::cli::{DeesserArgs, Globals};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

/// De-ess a voice track: tame the 4-8kHz sibilance band (podcast/voiceover cleanup).
pub fn run(args: DeesserArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_audio {
        return Err(Error::input("deesser: input has no audio stream"));
    }
    if !(0.05..=1.0).contains(&args.amount) {
        return Err(Error::input("--amount must be 0.05..=1.0"));
    }
    if !(0.2..=0.9).contains(&args.freq) {
        return Err(Error::input("--freq must be 0.2..=0.9 (band ratio)"));
    }
    let af = format!(
        "deesser=i={a:.3}:m=0.5:f={f:.3}",
        a = args.amount,
        f = args.freq
    );
    let fc = match &args.at {
        Some(raw) => Some(engine::audio_window_for(
            &af,
            raw,
            args.dur,
            probe.duration,
        )?),
        None => {
            if args.dur.is_some() {
                return Err(Error::input("--dur needs --at"));
            }
            None
        }
    };
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    match &fc {
        Some(fc) => {
            argv.extend(["-filter_complex", fc, "-map", "0:v?", "-map", "[aout]"]);
        }
        None => argv.extend(["-af", &af, "-map", "0:v?", "-map", "0:a"]),
    }
    if probe.has_video {
        argv.extend(["-c:v", "copy"]);
    }
    argv.extend(["-c:a", "aac"]);
    argv.push(&args.output);

    let c = engine::write_job("deesser", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "amount": args.amount,
        "freq": args.freq,
        "filter": "deesser",
    })))
}

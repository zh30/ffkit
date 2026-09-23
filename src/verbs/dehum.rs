use serde_json::json;

use crate::cli::{DehumArgs, Globals, MainsFreq};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: DehumArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_audio {
        return Err(Error::input("dehum: input has no audio stream"));
    }
    if !(1..=8).contains(&args.harmonics) {
        return Err(Error::input("--harmonics must be 1..=8"));
    }
    let hz: u32 = match (args.freq, args.mains) {
        (Some(f), _) => {
            if !(20..=500).contains(&f) {
                return Err(Error::input("--freq must be 20..=500 Hz"));
            }
            f
        }
        (None, MainsFreq::F50) => 50,
        (None, MainsFreq::F60) => 60,
    };

    // Narrow Q=12 notches at the mains fundamental and its harmonics —
    // low enough to leave voice/music intact, deep enough for buzz.
    let mut af = String::from("highpass=f=35");
    for k in 1..=args.harmonics {
        af.push_str(&format!(",equalizer=f={}:t=q:w=12:g=-20", hz * k));
    }

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

    let c = engine::write_job("dehum", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "mains_hz": hz,
        "harmonics": args.harmonics,
    })))
}

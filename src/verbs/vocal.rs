use serde_json::json;

use crate::cli::{Globals, VocalArgs, VocalMode};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: VocalArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_audio {
        return Err(Error::input("vocal: input has no audio stream"));
    }
    if probe.channels.unwrap_or(2) < 2 {
        return Err(Error::input(
            "vocal needs a stereo source — it works on the L/R difference",
        ));
    }
    // Center content (typically the vocal) lives equally in L and R.
    let a = args.amount.unwrap_or(1.0).clamp(0.0, 1.0);
    let af = match args.mode {
        // partial cancel keeps a fraction of the other channel (backing bleed)
        VocalMode::Karaoke => {
            format!("pan=stereo|c0=c0-{a:.3}*c1|c1=c1-{a:.3}*c0")
        }
        // amount blends toward a pure-center mix
        VocalMode::Isolate => {
            let s = 0.5 * a;
            let m = 1.0 - s;
            format!("pan=stereo|c0={m:.3}*c0+{s:.3}*c1|c1={s:.3}*c0+{m:.3}*c1")
        }
    };
    let fc = match &args.at {
        Some(raw) => {
            let at = crate::time::resolve_at(raw, args.dur, probe.duration)?;
            if !(0.0..probe.duration).contains(&at) {
                return Err(Error::input("--at is outside the input"));
            }
            Some(engine::audio_window(&af, at, args.dur))
        }
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
            argv.extend(["-filter_complex", fc]);
            if probe.has_video {
                argv.extend(["-map", "0:v", "-map", "[aout]", "-c:v", "copy"]);
            } else {
                argv.extend(["-map", "[aout]"]);
            }
        }
        None => {
            if probe.has_video {
                argv.extend(["-filter_complex", &format!("[0:a]{af}[aout]")]);
                argv.extend(["-map", "0:v", "-map", "[aout]", "-c:v", "copy"]);
            } else {
                argv.extend(["-af", &af]);
            }
        }
    }
    argv.extend(["-c:a", "aac"]);
    argv.push(&args.output);

    let mode = match args.mode {
        VocalMode::Karaoke => "karaoke",
        VocalMode::Isolate => "isolate",
    };
    let c = engine::write_job("vocal", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({ "mode": mode })))
}

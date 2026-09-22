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
    let af = match args.mode {
        VocalMode::Karaoke => "pan=stereo|c0=c0-c1|c1=c1-c0".to_string(),
        VocalMode::Isolate => "pan=mono|c0=0.5*c0+0.5*c1".to_string(),
    };
    let fc = match &args.at {
        Some(raw) => {
            let at = crate::time::parse_time(raw)?;
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

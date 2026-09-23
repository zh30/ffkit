use std::path::Path;

use serde_json::json;

use crate::cli::{Globals, MuteArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::paths;

pub fn run(args: MuteArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_audio {
        return Err(Error::input("input has no audio stream — nothing to mute"));
    }
    paths::ensure_input(&args.input)?;

    if args.dur.is_some() && args.at.is_none() {
        return Err(Error::input("--dur requires --at"));
    }

    let mut argv = ffmpeg_base(g.progress);
    argv.extend(["-i".to_string(), args.input.display().to_string()]);
    match &args.at {
        // Silence inside the window, keep the rest of the track.
        Some(raw) => {
            let at = crate::time::resolve_at(raw, args.dur, probe.duration)?;
            if at >= probe.duration {
                return Err(Error::input("--at is past the end of the input"));
            }
            let end = (at + args.dur.unwrap_or(probe.duration - at)).min(probe.duration);
            argv.extend(["-map".to_string(), "0".to_string()]);
            argv.extend([
                "-af".to_string(),
                format!("volume=0:enable='between(t,{at:.3},{end:.3})':eval=frame"),
            ]);
            argv.extend(["-c:v".to_string(), "copy".to_string()]);
            argv.extend(["-c:a".to_string(), "aac".to_string()]);
        }
        // Keep every stream except audio, stream-copied.
        None => {
            argv.extend(["-map".to_string(), "0".to_string()]);
            argv.extend(["-map".to_string(), "-0:a".to_string()]);
            argv.extend(["-c".to_string(), "copy".to_string()]);
        }
    }
    argv.push(args.output.display().to_string());

    let inputs: Vec<&Path> = vec![&args.input];
    let mut c = engine::write_job("mute", &inputs, &args.output, vec![argv], g)?;
    c = c.with_extra(json!({ "dropped": "audio" }));
    Ok(c)
}

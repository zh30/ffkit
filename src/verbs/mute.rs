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

    let mut argv = ffmpeg_base(g.progress);
    argv.extend(["-i".to_string(), args.input.display().to_string()]);
    // Keep every stream except audio, stream-copied.
    argv.extend(["-map".to_string(), "0".to_string()]);
    argv.extend(["-map".to_string(), "-0:a".to_string()]);
    argv.extend(["-c".to_string(), "copy".to_string()]);
    argv.push(args.output.display().to_string());

    let inputs: Vec<&Path> = vec![&args.input];
    let mut c = engine::write_job("mute", &inputs, &args.output, vec![argv], g)?;
    c = c.with_extra(json!({ "dropped": "audio" }));
    Ok(c)
}

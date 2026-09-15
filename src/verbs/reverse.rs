use serde_json::json;

use crate::cli::{Globals, ReverseArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: ReverseArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_video && !probe.has_audio {
        return Err(Error::input("reverse: input has no streams"));
    }
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    if probe.has_video {
        argv.extend([
            "-vf", "reverse", "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt",
            "yuv420p",
        ]);
    } else {
        argv.push("-vn");
    }
    if probe.has_audio {
        argv.extend(["-af", "areverse", "-c:a", "aac"]);
    } else {
        argv.push("-an");
    }
    argv.push(&args.output);
    let c = engine::write_job("reverse", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({ "reversed": true })))
}

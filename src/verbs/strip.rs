use serde_json::json;

use crate::cli::{Globals, StripArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: StripArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_video && !probe.has_audio {
        return Err(Error::input("strip: input has no media streams"));
    }

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend([
        "-map",
        "0",
        "-map_metadata",
        "-1",
        "-map_chapters",
        "-1",
        "-c",
        "copy",
    ]);
    argv.push(&args.output);

    let c = engine::write_job("strip", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({})))
}

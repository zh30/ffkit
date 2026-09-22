use serde_json::json;

use crate::cli::{Globals, SubsArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: SubsArgs, g: &Globals) -> Result<Contract, Error> {
    let _probe = engine::probe_or_err(&args.input, g)?;
    let codec = match args
        .output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
    {
        "srt" => "srt",
        "vtt" => "webvtt",
        "ass" | "ssa" => "ass",
        ext => {
            return Err(Error::input(format!(
                "subs output must be .srt/.vtt/.ass, got .{ext}"
            )))
        }
    };
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-map", &format!("0:s:{}", args.stream)]);
    argv.extend(["-c:s", codec]);
    argv.push(&args.output);

    let c = engine::write_job("subs", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "stream": args.stream,
        "codec": codec,
    })))
}

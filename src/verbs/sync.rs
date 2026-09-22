use std::path::Path;

use serde_json::json;

use crate::cli::{Globals, SyncArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: SyncArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_audio {
        return Err(Error::input(
            "sync shifts audio — input has no audio stream",
        ));
    }
    if args.ms.abs() > 60_000.0 {
        return Err(Error::input("--ms must be within ±60000"));
    }

    // Positive: audio starts later (pad). Negative: audio starts earlier (trim).
    let af = if args.ms > 0.0 {
        format!("adelay={:.0}:all=1", args.ms)
    } else {
        format!("atrim=start={:.3},asetpts=PTS-STARTPTS", -args.ms / 1000.0)
    };

    let mut argv = ffmpeg_base(g.progress);
    argv.extend(["-i".to_string(), args.input.display().to_string()]);
    if probe.has_video {
        argv.extend([
            "-filter_complex".to_string(),
            format!("[0:a]{af}[aout]"),
            "-map".to_string(),
            "0:v".to_string(),
            "-map".to_string(),
            "[aout]".to_string(),
            "-c:v".to_string(),
            "copy".to_string(),
        ]);
    } else {
        argv.extend(["-af".to_string(), af]);
    }
    argv.extend(["-c:a".to_string(), "aac".to_string()]);
    argv.push(args.output.display().to_string());

    let inputs: Vec<&Path> = vec![&args.input];
    let mut c = engine::write_job("sync", &inputs, &args.output, vec![argv], g)?;
    c = c.with_extra(json!({ "shift_ms": args.ms }));
    Ok(c)
}

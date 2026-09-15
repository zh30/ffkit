use serde_json::json;

use crate::cli::{Globals, StabilizeArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: StabilizeArgs, g: &Globals) -> Result<Contract, Error> {
    if !(4..=64).contains(&args.rx) || !(4..=64).contains(&args.ry) {
        return Err(Error::input("--rx and --ry must be 4..=64"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "stabilize")?;

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend([
        "-vf",
        &format!("deshake=rx={}:ry={}", args.rx, args.ry),
        "-c:v",
        "libx264",
        "-preset",
        "fast",
        "-crf",
        "18",
        "-pix_fmt",
        "yuv420p",
    ]);
    if probe.has_audio {
        argv.extend(["-c:a", "copy"]);
    }
    argv.push(&args.output);

    let c = engine::write_job("stabilize", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "filter": "deshake",
        "rx": args.rx,
        "ry": args.ry,
    })))
}

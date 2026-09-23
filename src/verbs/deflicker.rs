use serde_json::json;

use crate::cli::{DeflickerArgs, Globals};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: DeflickerArgs, g: &Globals) -> Result<Contract, Error> {
    if !(3..=129).contains(&args.size) {
        return Err(Error::input(
            "--size must be 3..=129 (temporal window frames)",
        ));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "deflicker")?;

    let vf = match args.engine.unwrap_or(crate::cli::DeflickerEngine::Am) {
        crate::cli::DeflickerEngine::Am => format!("deflicker=size={}:mode=am", args.size),
        // tmidequalizer: radius counts frames either side → size/2, capped at 60
        crate::cli::DeflickerEngine::Tmide => {
            format!("tmidequalizer=radius={}", (args.size / 2).clamp(1, 60))
        }
    };
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-vf", &vf]);
    argv.extend([
        "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
    ]);
    if probe.has_audio {
        argv.extend(["-c:a", "copy"]);
    }
    argv.push(&args.output);

    let c2 = engine::write_job("deflicker", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c2.with_extra(json!({
        "size": args.size,
        "engine": match args.engine.unwrap_or(crate::cli::DeflickerEngine::Am) {
            crate::cli::DeflickerEngine::Am => "deflicker",
            crate::cli::DeflickerEngine::Tmide => "tmidequalizer",
        },
    })))
}

use crate::cli::{PremultArgs, PremultMode};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use serde_json::json;

/// Straight ↔ premultiplied alpha conversion, in place on an alpha-carrying
/// clip (yuva420p/prores4444/alpha webm). Motion/AE handoffs where the
/// receiving tool expects the other convention.
pub fn run(args: PremultArgs, g: &crate::cli::Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "premult")?;
    let vf = match args.mode {
        PremultMode::Premultiply => "premultiply=inplace=1",
        PremultMode::Unpremultiply => "unpremultiply=inplace=1",
    };
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    // alpha survives through prores_ks 4444 — keeps the channel the filter edits
    argv.extend([
        "-vf",
        vf,
        "-c:v",
        "prores_ks",
        "-profile:v",
        "4444",
        "-pix_fmt",
        "yuva444p10le",
    ]);
    if probe.has_audio {
        argv.extend(["-c:a", "copy"]);
    }
    argv.push(&args.output);
    let c = engine::write_job("premult", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({ "mode": vf })))
}

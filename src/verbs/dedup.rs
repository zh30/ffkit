use serde_json::json;

use crate::cli::{DedupArgs, Globals};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

/// Drop near-duplicate frames (screen recordings, slide decks, static b-roll).
pub fn run(args: DedupArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "dedup")?;
    if !(0.01..=1.0).contains(&args.frac) {
        return Err(Error::input(
            "--frac must be 0.01..=1.0 (changed-pixel fraction to keep a frame)",
        ));
    }

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    // mpdecimate drops frames where fewer than `frac` of pixels differ. Output
    // is VFR — timestamps are preserved so audio sync and duration hold.
    argv.extend([
        "-vf",
        &format!("mpdecimate=frac={f:.3}", f = args.frac),
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

    let c = engine::write_job("dedup", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "frac": args.frac,
        "filter": "mpdecimate",
    })))
}

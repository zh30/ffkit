use serde_json::json;

use crate::cli::{DeclipArgs, DeclipEngine, Globals};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

/// Repair clipped (blown-out) audio with `adeclip`: it interpolates the
/// flattened peaks back into waveforms. `--threshold` is the clip fraction —
/// lower rescues harsher clipping; `--window` is the analysis slice in ms.
pub fn run(args: DeclipArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_audio {
        return Err(Error::input("declip: input has no audio stream"));
    }
    if !(10.0..=100.0).contains(&args.window) {
        return Err(Error::input("--window must be 10..=100 ms"));
    }
    if !(1.0..=100.0).contains(&args.threshold) {
        return Err(Error::input("--threshold must be 1..=100"));
    }
    let method = if args.overlap_save { "save" } else { "add" };
    let af = match args.engine.unwrap_or(DeclipEngine::Clip) {
        DeclipEngine::Clip => format!(
            "adeclip=w={:.0}:t={:.0}:m={method}",
            args.window, args.threshold
        ),
        DeclipEngine::Click => format!(
            "adeclick=w={:.0}:o=75:arorder=2:t={:.0}:b=2:m={method}",
            args.window, args.threshold
        ),
    };
    let fc = match &args.at {
        Some(raw) => Some(engine::audio_window_for(
            &af,
            raw,
            args.dur,
            probe.duration,
        )?),
        None => {
            if args.dur.is_some() {
                return Err(Error::input("--dur needs --at"));
            }
            None
        }
    };
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    match &fc {
        Some(fc) => {
            argv.extend(["-filter_complex", fc, "-map", "0:v?", "-map", "[aout]"]);
        }
        None => argv.extend(["-af", &af, "-map", "0:v?", "-map", "0:a"]),
    }
    if probe.has_video {
        argv.extend(["-c:v", "copy"]);
    }
    // AAC-in-.wav fails to decode on ffmpeg 4.x — lossless pcm there instead.
    let acodec = if args.output.extension().and_then(|e| e.to_str()) == Some("wav") {
        "pcm_s16le"
    } else {
        "aac"
    };
    argv.extend(["-c:a", acodec]);
    argv.push(&args.output);

    let c = engine::write_job("declip", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "window_ms": args.window,
        "threshold": args.threshold,
        "overlap": if args.overlap_save { "save" } else { "add" },
        "filter": if matches!(args.engine, Some(DeclipEngine::Click)) { "adeclick" } else { "adeclip" },
    })))
}

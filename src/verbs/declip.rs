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
    let mut af = match args.engine.unwrap_or(DeclipEngine::Clip) {
        DeclipEngine::Clip => format!(
            "{}adeclip=w={:.0}:t={:.0}:m={method}",
            symmetric_plateau(),
            args.window,
            args.threshold
        ),
        DeclipEngine::Click => format!(
            "adeclick=w={:.0}:o=75:arorder=2:t={:.0}:b=2:m={method}",
            args.window, args.threshold
        ),
    };
    // adeclip rebuilds the flattened peaks above full scale (a 31%-clipped
    // sine comes back at +1 dB); written to s16/AAC they are clipped flat
    // again. Measure the repaired peak once and trim only what overshoots.
    // Whole-file repairs only: a windowed repair would step in level.
    let mut headroom_db = 0.0;
    let clip_engine = matches!(
        args.engine.unwrap_or(DeclipEngine::Clip),
        DeclipEngine::Clip
    );
    if clip_engine && args.at.is_none() && !g.dry_run {
        if let Some(peak) = repaired_peak_db(&args.input, &af, g)? {
            if peak > -0.1 {
                headroom_db = -(peak + 0.1);
                af = format!("{af},volume={headroom_db:.2}dB");
            }
        }
    }
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
        "headroom_db": (headroom_db * 100.0_f64).round() / 100.0,
    })))
}

/// Peak level (dBFS) of the repaired audio, measured in float before any
/// integer output can clip it.
fn repaired_peak_db(input: &std::path::Path, af: &str, g: &Globals) -> Result<Option<f64>, Error> {
    let mut argv = crate::spawn::Argv::ffmpeg();
    argv.extend(["-nostats", "-i"]);
    argv.push(input);
    argv.extend([
        "-af".to_string(),
        format!("{af},astats=measure_overall=Peak_level:measure_perchannel=none"),
        "-f".to_string(),
        "null".to_string(),
        "-".to_string(),
    ]);
    let spawned = crate::spawn::run(&argv, g.timeout, false)?;
    let stderr = String::from_utf8_lossy(&spawned.stderr);
    Ok(stderr.lines().rev().find_map(|l| {
        l.split("Peak level dB:")
            .nth(1)
            .and_then(|v| v.trim().parse().ok())
    }))
}

/// Integer PCM clips at +32767/32768 but −1.0, and ffmpeg 9's adeclip
/// repairs nothing on that lopsided plateau (peak stays 0.0 dB; the same
/// wave clipped symmetrically comes back at +1.03 dB). Shave the negative
/// rail to match the positive one first — a 4e-5 change. Builds without
/// asoftclip's hard type (4.4) never needed it.
fn symmetric_plateau() -> &'static str {
    static HARD: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    if *HARD.get_or_init(|| engine::filter_help("asoftclip").contains("hard")) {
        "aformat=sample_fmts=dbl,asoftclip=type=hard:threshold=0.99996,"
    } else {
        ""
    }
}

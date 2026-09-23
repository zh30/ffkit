use serde_json::json;

use crate::cli::{Globals, ScanArgs};
use crate::contract::Contract;
use crate::engine;
use crate::error::Error;
use crate::spawn::{self, Argv};

/// QC pass: report black stretches, frozen frames and per-black-frame hits.
/// Report-only — writes no media, findings land in `extras`.
pub fn run(args: ScanArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "scan")?;
    let freeze_min = args.freeze_min.unwrap_or(1.0);
    let black_min = args.black_min.unwrap_or(0.3);
    let thresh = args.thresh.unwrap_or(32.0).clamp(0.0, 255.0);

    // photosensitivity rides the same pass: bypass=1 keeps frames intact and
    // emits lavfi.photosensitivity.* metadata; metadata=print mirrors it to the
    // log where we count flash-flagged frames (badness > 0)
    let vf = format!(
        "blackdetect=d={black_min}:pic_th=0.98,blackframe=thresh={thresh:.0}:amount=98,freezedetect=d={freeze_min},photosensitivity=bypass=1,idet,metadata=print:file=-"
    );
    let mut argv = Argv::ffmpeg();
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-vf", &vf, "-an", "-f", "null", "-"]);
    let spawned = spawn::run(&argv, g.timeout, false)?;
    let spawned = spawn::require_ok(&argv, spawned)?;
    // detect logs land on stderr; metadata=print:file=- writes to stdout
    let log = format!(
        "{}\n{}",
        spawn::stderr_str(&spawned),
        spawn::stdout_str(&spawned).unwrap_or("")
    );

    let mut black_ranges: Vec<serde_json::Value> = Vec::new();
    let mut freeze_starts: Vec<f64> = Vec::new();
    let mut freeze_ends: Vec<f64> = Vec::new();
    let mut black_frames = 0usize;
    let mut flash_frames = 0usize;
    let mut idet_counts = (0usize, 0usize, 0usize, 0usize); // tff, bff, prog, undet
    let mut flash_max = 0.0f64;
    for line in log.lines() {
        if let Some(rest) = line.split("black_start:").nth(1) {
            let s = rest
                .split_whitespace()
                .next()
                .unwrap_or("")
                .parse::<f64>()
                .unwrap_or(0.0);
            let e = rest
                .split("black_end:")
                .nth(1)
                .and_then(|r| r.split_whitespace().next())
                .and_then(|t| t.parse::<f64>().ok())
                .unwrap_or(s);
            black_ranges.push(json!({"start": s, "end": e, "duration": e - s}));
        }
        if let Some(rest) = line.split("freeze_start:").nth(1) {
            if let Ok(v) = rest.trim().parse::<f64>() {
                freeze_starts.push(v);
            }
        }
        if let Some(rest) = line.split("freeze_end:").nth(1) {
            if let Ok(v) = rest.trim().parse::<f64>() {
                freeze_ends.push(v);
            }
        }
        if line.contains("pblack:") {
            black_frames += 1;
        }
        if line.contains("Single frame detection:") {
            let grab = |tag: &str| -> usize {
                line.split(tag)
                    .nth(1)
                    .and_then(|r| r.trim_start_matches(':').split_whitespace().next())
                    .and_then(|v| v.parse::<usize>().ok())
                    .unwrap_or(0)
            };
            idet_counts = (
                grab("TFF"),
                grab("BFF"),
                grab("Progressive"),
                grab("Undetermined"),
            );
        }
        if let Some(rest) = line.split("lavfi.photosensitivity.badness=").nth(1) {
            let b = rest.trim().parse::<f64>().unwrap_or(0.0);
            // badness ~1.8 on smoothly animated content, ~3+ on real strobes
            if b > 2.0 {
                flash_frames += 1;
                flash_max = flash_max.max(b);
            }
        }
    }
    let freeze_ranges: Vec<serde_json::Value> = freeze_starts
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let e = freeze_ends.get(i).copied().unwrap_or(probe.duration);
            json!({"start": s, "end": e, "duration": e - s})
        })
        .collect();

    Ok(Contract::ok("scan", None, Some(probe)).with_extra(json!({
        "freeze_min": freeze_min,
        "black_min": black_min,
        "luma_threshold": thresh,
        "black_ranges": black_ranges,
        "freeze_ranges": freeze_ranges,
        "black_frames": black_frames,
        // photosensitive-epilepsy QC: frames where luminance oscillates enough
        // to flag (Harding-style heuristic; ship with a warning card if >0)
        "flash_frames": flash_frames,
        "flash_max_badness": flash_max,
        // interlace verdict from idet single-frame detection
        "interlaced": idet_counts.0 + idet_counts.1 > idet_counts.2,
        "frames_tff": idet_counts.0,
        "frames_bff": idet_counts.1,
        "frames_progressive": idet_counts.2,
        "frames_undetermined": idet_counts.3,
    })))
}

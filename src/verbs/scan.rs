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
    let scdet_leg = if args.scenes { ",scdet=t=8" } else { "" };
    let vf = format!(
        "blackdetect=d={black_min}:pic_th=0.98,blackframe=thresh={thresh:.0}:amount=98,freezedetect=d={freeze_min},photosensitivity=bypass=1,idet,entropy=mode=diff{scdet_leg},metadata=print:file=-"
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
    let mut entropy_vals: Vec<f64> = Vec::new();
    let mut scene_cuts: Vec<f64> = Vec::new();
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
        if let Some(rest) = line.split("lavfi.scd.time:").nth(1) {
            if let Ok(t) = rest.trim().split(' ').next().unwrap_or("").parse::<f64>() {
                scene_cuts.push(t);
            }
        }
        if let Some(rest) = line.split("normalized_entropy.diff.Y=").nth(1) {
            if let Ok(v) = rest.trim().split(' ').next().unwrap_or("").parse::<f64>() {
                entropy_vals.push(v);
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
    // stereo mono-compat QC: Pearson r between L/R decoded in one extra pass.
    // Near +1 = mono-like (fine), near 0 = decorrelated, <0 = out-of-phase —
    // the last one collapses on mono speakers (podcast/phone playback).
    let phase_corr = if probe.channels == Some(2) {
        // pipe-buffer deadlock guard: PCM exceeds the 64KB pipe fast, and
        // spawn::run waits for exit before draining — write to a temp file
        let tmp = match tempfile::NamedTempFile::new() {
            Ok(t) => t,
            Err(_) => {
                return Ok(
                    Contract::ok("scan", None, Some(probe)).with_extra(json!({"phase_corr": null}))
                )
            }
        };
        let pcm_path = tmp.path().to_path_buf();
        let mut argv = Argv::ffmpeg();
        argv.push("-i");
        argv.push(&args.input);
        argv.extend([
            "-vn",
            "-f",
            "s16le",
            "-acodec",
            "pcm_s16le",
            "-t",
            "10",
            "-y",
        ]);
        argv.push(&pcm_path);
        spawn::run(&argv, g.timeout, false)
            .ok()
            .filter(|sp| sp.status_ok)
            .and_then(|_| std::fs::read(&pcm_path).ok())
            .map(|pcm| {
                let pcm = &pcm[..];
                let mut sx = 0f64;
                let mut sy = 0f64;
                let mut sxx = 0f64;
                let mut syy = 0f64;
                let mut sxy = 0f64;
                let mut n = 0usize;
                // cap at ~10s of 44.1k stereo to bound CPU
                for pair in pcm.chunks_exact(4).take(44100 * 10) {
                    let l = i16::from_le_bytes([pair[0], pair[1]]) as f64;
                    let r = i16::from_le_bytes([pair[2], pair[3]]) as f64;
                    sx += l;
                    sy += r;
                    sxx += l * l;
                    syy += r * r;
                    sxy += l * r;
                    n += 1;
                }
                if n == 0 {
                    return None;
                }
                let nf = n as f64;
                let num = sxy - sx * sy / nf;
                let den = ((sxx - sx * sx / nf) * (syy - sy * sy / nf)).sqrt();
                if den > 0.0 {
                    Some((num / den * 1000.0).round() / 1000.0)
                } else {
                    Some(1.0)
                }
            })
            .and_then(|x| x)
    } else {
        None
    };

    let freeze_ranges: Vec<serde_json::Value> = freeze_starts
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let e = freeze_ends.get(i).copied().unwrap_or(probe.duration);
            json!({"start": s, "end": e, "duration": e - s})
        })
        .collect();

    // blur QC: entropy diff-mode normalized Y < threshold reads as soft/OOF.
    let blur_th = args.blur.unwrap_or(0.45).clamp(0.0, 1.0);
    let blur_frames = entropy_vals.iter().filter(|&&v| v < blur_th).count();
    let blur_mean = if entropy_vals.is_empty() {
        None
    } else {
        Some(entropy_vals.iter().sum::<f64>() / entropy_vals.len() as f64)
    };
    let blur_min = entropy_vals.iter().cloned().reduce(f64::min);

    // volumedetect pass: peak + mean dB (clip check + cheap loudness read)
    let (mut audio_max_db, mut audio_mean_db): (Option<f64>, Option<f64>) = (None, None);
    if probe.has_audio {
        let mut argv = Argv::ffmpeg();
        argv.push("-i");
        argv.push(&args.input);
        argv.extend(["-af", "volumedetect", "-f", "null", "-"]);
        if let Ok(sp) = spawn::run(&argv, g.timeout, false) {
            let stderr = String::from_utf8_lossy(&sp.stderr);
            for line in stderr.lines() {
                if let Some(v) = line.split("max_volume:").nth(1) {
                    audio_max_db = v.trim().trim_end_matches(" dB").parse().ok();
                }
                if let Some(v) = line.split("mean_volume:").nth(1) {
                    audio_mean_db = v.trim().trim_end_matches(" dB").parse().ok();
                }
            }
        }
    }

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
        "scene_cuts": scene_cuts,
        "flash_max_badness": flash_max,
        // interlace verdict from idet single-frame detection
        "interlaced": idet_counts.0 + idet_counts.1 > idet_counts.2,
        "frames_tff": idet_counts.0,
        "frames_bff": idet_counts.1,
        "frames_progressive": idet_counts.2,
        "frames_undetermined": idet_counts.3,
        // L/R phase correlation, stereo inputs only: <0 collapses in mono
        "phase_corr": phase_corr,
        // peak level (>= -0.5 dB clips on most encoders) + programme mean
        "audio_max_db": audio_max_db,
        "audio_mean_db": audio_mean_db,
        // entropy blur QC: normalized luma-diff entropy per frame —
        // soft/out-of-focus stretches sink under blur_threshold
        "blur_threshold": blur_th,
        "blur_frames": blur_frames,
        "blur_mean": blur_mean,
        "blur_min": blur_min,
    })))
}

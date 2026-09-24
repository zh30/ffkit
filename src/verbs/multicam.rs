//! `multicam` — angle switching across two aligned recordings.
//! `--at t1,t2,...` flips to the other camera at each timestamp (starts on
//! input 0). Pair it with `align` first when the two takes aren't synced.

use std::path::Path;

use serde_json::json;

use crate::cli::{Globals, MulticamArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::time::parse_time;

pub fn run(args: MulticamArgs, g: &Globals) -> Result<Contract, Error> {
    let cams = [&args.cam_a, &args.cam_b];
    let probes = [
        engine::probe_or_err(&args.cam_a, g)?,
        engine::probe_or_err(&args.cam_b, g)?,
    ];
    if probes.iter().any(|p| !p.has_video) {
        return Err(Error::input("multicam: both inputs need a video stream"));
    }
    if probes[0].has_audio != probes[1].has_audio {
        return Err(Error::input(
            "multicam: both inputs need the same stream layout (audio present in both or neither)",
        ));
    }
    let dur = probes
        .iter()
        .map(|p| p.duration)
        .fold(f64::INFINITY, f64::min);
    if args.at.is_empty() {
        return Err(Error::input("multicam needs at least one --at cut"));
    }

    // --align: cross-correlate the two audio tracks (same pass as `align`)
    // and shift camera B's legs by the detected offset — multicam without a
    // separate align run when the two takes started at different wall times.
    let mut b_shift = 0.0f64;
    let mut aligned_ms: Option<f64> = None;
    if args.align {
        if !probes[0].has_audio {
            return Err(Error::input("--align needs an audio track in both inputs"));
        }
        let a = crate::verbs::align::pcm(&args.cam_a, Some(30.0))?;
        let b = crate::verbs::align::pcm(&args.cam_b, Some(30.0))?;
        let lag = crate::verbs::align::detect_lag(
            &a,
            &b,
            (30.0 * crate::verbs::align::SAMPLE_RATE as f64) as usize,
        )
        .ok_or_else(|| Error::input("no usable audio to align (too short)"))?;
        let ms = lag as f64 * 1000.0 / crate::verbs::align::SAMPLE_RATE as f64;
        b_shift = ms / 1000.0;
        aligned_ms = Some((ms * 10.0).round() / 10.0);
    }
    let mut cuts: Vec<f64> = Vec::new();
    for s in &args.at {
        let t = if s.trim().eq_ignore_ascii_case("end") {
            dur - 0.06
        } else {
            parse_time(s)?
        };
        if !(0.05..dur - 0.05).contains(&t) {
            return Err(Error::input(format!(
                "--at {s} is outside the {:.2}s shared window",
                dur
            )));
        }
        cuts.push(t);
    }
    cuts.sort_by(|a, b| a.total_cmp(b));
    cuts.dedup();
    if cuts.len() > 200 {
        return Err(Error::input("multicam is capped at 200 switches"));
    }

    let vw = probes[0].width.unwrap_or(1280);
    let vh = probes[0].height.unwrap_or(720);

    // Boundaries → segments alternating between the two inputs (start on A).
    let mut bounds = vec![0.0];
    bounds.extend(cuts.iter().copied());
    bounds.push(dur);
    let segs: Vec<(f64, f64, usize)> = bounds
        .windows(2)
        .enumerate()
        .map(|(i, w)| (w[0], w[1], i % 2))
        .collect();

    let mut fc: Vec<String> = Vec::new();
    // Pre-scale the second camera onto the first's canvas if it differs.
    for i in 1..2usize {
        fc.push(format!(
            "[{i}:v]scale={vw}:{vh}:force_original_aspect_ratio=decrease,pad={vw}:{vh}:(ow-iw)/2:(oh-ih)/2,setsar=1[cs{i}]"
        ));
    }
    let mut ins = String::new();
    let has_audio = probes[0].has_audio;
    for (k, (s, e, src)) in segs.iter().enumerate() {
        let vsrc = if *src == 0 {
            format!("{src}:v")
        } else {
            format!("cs{src}")
        };
        // --align shifts camera B's legs by the detected offset: its take
        // started `b_shift` seconds off A's clock, so content for shared
        // timeline time T lives at T + b_shift in B's file.
        let (s2, e2) = if *src == 1 && b_shift != 0.0 {
            let lo = (s + b_shift).max(0.0);
            let hi = (e + b_shift).max(lo + 0.02).min(probes[1].duration);
            (lo, hi)
        } else {
            (*s, *e)
        };
        fc.push(format!(
            "[{vsrc}]trim=start={s2:.3}:end={e2:.3},setpts=PTS-STARTPTS[vs{k}]"
        ));
        ins.push_str(&format!("[vs{k}]"));
        if has_audio && !args.keep_audio {
            fc.push(format!(
                "[{src}:a]atrim=start={s2:.3}:end={e2:.3},asetpts=PTS-STARTPTS[as{k}]"
            ));
            ins.push_str(&format!("[as{k}]"));
        }
    }
    let n = segs.len();
    if let Some(f) = args.transition {
        if f <= 0.0 || segs.iter().any(|(s, e, _)| e - s <= f) {
            return Err(Error::input(
                "--transition must be > 0 and shorter than every segment",
            ));
        }
        // xfade chain: switch k lands fade earlier each time (offset = T_k - k*f).
        let mut prev_v = "vs0".to_string();
        let mut prev_a = "as0".to_string();
        let audio = has_audio && !args.keep_audio;
        for (k, seg) in segs.iter().enumerate().take(n).skip(1) {
            let last = k == n - 1;
            let ov = if last {
                "vout".to_string()
            } else {
                format!("xv{k}")
            };
            let off = seg.0 - k as f64 * f;
            fc.push(format!(
                "[{prev_v}][vs{k}]xfade=transition=fade:duration={f:.3}:offset={off:.3}[{ov}]"
            ));
            prev_v = ov;
            if audio {
                let oa = if last {
                    "aout".to_string()
                } else {
                    format!("xa{k}")
                };
                fc.push(format!("[{prev_a}][as{k}]acrossfade=d={f:.3}[{oa}]"));
                prev_a = oa;
            }
        }
    } else if has_audio && !args.keep_audio {
        fc.push(format!("{ins}concat=n={n}:v=1:a=1[vout][aout]"));
    } else {
        fc.push(format!("{ins}concat=n={n}:v=1:a=0[vout]"));
    }
    // --keep-audio: camera A runs the whole interview; concat leaves no aout
    // so map the A track straight through.
    let map_a_direct = has_audio && args.keep_audio;

    let mut argv = ffmpeg_base(g.progress);
    for c in cams {
        argv.extend(["-i".to_string(), c.display().to_string()]);
    }
    argv.extend(["-filter_complex".to_string(), fc.join(";")]);
    argv.extend(["-map".to_string(), "[vout]".to_string()]);
    if has_audio {
        argv.extend([
            "-map".to_string(),
            if map_a_direct {
                "0:a".to_string()
            } else {
                "[aout]".to_string()
            },
        ]);
    }
    argv.extend([
        "-c:v".to_string(),
        "libx264".to_string(),
        "-preset".to_string(),
        "fast".to_string(),
        "-crf".to_string(),
        "18".to_string(),
        "-pix_fmt".to_string(),
        "yuv420p".to_string(),
    ]);
    if has_audio {
        argv.extend(["-c:a".to_string(), "aac".to_string()]);
    }
    argv.push(args.output.display().to_string());

    let inputs: Vec<&Path> = vec![&args.cam_a, &args.cam_b];
    let mut c = engine::write_job("multicam", &inputs, &args.output, vec![argv], g)?;
    c = c.with_extra(json!({ "cuts": cuts, "angles": n, "keep_audio": args.keep_audio, "transition": args.transition, "align_offset_ms": aligned_ms }));
    Ok(c)
}

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
    let mut cuts: Vec<f64> = Vec::new();
    for s in &args.at {
        let t = parse_time(s)?;
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
        fc.push(format!(
            "[{vsrc}]trim=start={s:.3}:end={e:.3},setpts=PTS-STARTPTS[vs{k}]"
        ));
        ins.push_str(&format!("[vs{k}]"));
        if has_audio && !args.keep_audio {
            fc.push(format!(
                "[{src}:a]atrim=start={s:.3}:end={e:.3},asetpts=PTS-STARTPTS[as{k}]"
            ));
            ins.push_str(&format!("[as{k}]"));
        }
    }
    let n = segs.len();
    if has_audio && !args.keep_audio {
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
    c = c.with_extra(json!({ "cuts": cuts, "angles": n, "keep_audio": args.keep_audio }));
    Ok(c)
}

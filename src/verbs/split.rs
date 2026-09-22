use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::json;

use crate::cli::{Globals, SplitArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::paths;
use crate::spawn;

/// Split a file into equal-length parts (WhatsApp Status 30s, Stories 60s).
/// Re-encodes with forced keyframes at every boundary so the segment muxer
/// cuts exactly on `--every`, writing `stem_%02d.ext` (or the caller's own
/// printf pattern) next to it.
pub fn run(args: SplitArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_video && !probe.has_audio {
        return Err(Error::input("split: input has no streams"));
    }
    paths::ensure_input(&args.input)?;

    // Boundaries in source seconds: regular grid from --every, or explicit
    // chapter points from --at.
    // --size turns a byte target into an even --every grid: parts ≈ target.
    let every = match &args.size {
        Some(s) => {
            let target = crate::verbs::compress::parse_size(s)? as f64;
            let total = probe.size_bytes.unwrap_or(0) as f64;
            if total <= 0.0 {
                return Err(Error::input("unknown input size — use --every"));
            }
            // Cap parts so each stays ≥0.5s (segment min), else the target
            // is simply unreachable at this duration.
            let n = ((total / target).ceil() as usize)
                .min((probe.duration / 0.5).floor().max(1.0) as usize);
            if n < 2 {
                return Err(Error::input("already under --size — no split needed"));
            }
            if args.every.is_some() || !args.at.is_empty() || args.scenes.is_some() {
                return Err(Error::input(
                    "split --size stands alone (no --every/--at/--scenes)",
                ));
            }
            Some(probe.duration / n as f64)
        }
        None => match args.parts {
            Some(n) => {
                if n < 2 {
                    return Err(Error::input("--parts must be ≥2"));
                }
                if args.every.is_some() || !args.at.is_empty() || args.scenes.is_some() {
                    return Err(Error::input(
                        "split --parts stands alone (no --every/--at/--scenes)",
                    ));
                }
                Some(probe.duration / n as f64)
            }
            None => args.every,
        },
    };

    let mut cuts: Vec<f64> = Vec::new();
    if args.min_silence.is_some() && args.silence.is_none() {
        return Err(Error::input("--min-silence needs --silence"));
    }
    if let Some(thr) = args.silence {
        if every.is_some() || !args.at.is_empty() || args.scenes.is_some() {
            return Err(Error::input(
                "split --silence stands alone (no --every/--at/--scenes/--size/--parts)",
            ));
        }
        if !(-80.0..=-5.0).contains(&thr) {
            return Err(Error::input("--silence threshold must be -80..-5 dB"));
        }
        if !probe.has_audio {
            return Err(Error::input("split --silence needs an audio stream"));
        }
        let min_gap = match args.min_silence {
            Some(m) if m > 0.0 => m,
            Some(_) => return Err(Error::input("--min-silence must be > 0")),
            None => 0.4,
        };
        let silences = crate::silence::detect(&args.input, thr, min_gap, g.timeout, true)?;
        for (a, b) in silences {
            let mid = (a + b) / 2.0;
            if (0.05..probe.duration - 0.05).contains(&mid) {
                cuts.push(mid);
            }
        }
        cuts.sort_by(|a, b| a.total_cmp(b));
        cuts.dedup();
    }
    match (every, args.at.is_empty(), args.scenes) {
        (Some(e), true, None) => {
            if !(0.5..=3600.0).contains(&e) {
                return Err(Error::input("--every must be 0.5..=3600 seconds"));
            }
            let mut k = 1usize;
            while (k as f64) * e < probe.duration - 0.05 {
                cuts.push(k as f64 * e);
                k += 1;
            }
        }
        (None, false, None) => {
            for s in &args.at {
                let t = crate::time::parse_time(s)?;
                if !(0.05..probe.duration - 0.05).contains(&t) {
                    return Err(Error::input(format!(
                        "split --at {s} is outside the {:.2}s source",
                        probe.duration
                    )));
                }
                cuts.push(t);
            }
            cuts.sort_by(|a, b| a.total_cmp(b));
            cuts.dedup();
        }
        (None, _, Some(thr)) => {
            if !(0.05..=0.95).contains(&thr) {
                return Err(Error::input("--scenes threshold must be 0.05..=0.95"));
            }
            cuts = scene_cuts(&args.input, thr, g)?;
        }
        (Some(_), false, None) => {
            return Err(Error::input("split takes --every or --at, not both"));
        }
        (None, true, None) => {
            if args.silence.is_none() {
                return Err(Error::input(
                    "split needs --every S, --at t1,t2,..., --scenes T or --silence dB",
                ));
            }
        }
        (Some(_), _, Some(_)) => {
            return Err(Error::input("split takes --every or --scenes, not both"));
        }
    }
    if cuts.is_empty() {
        return Err(Error::input("split produced no parts inside the source"));
    }
    if cuts.len() > 500 {
        return Err(Error::input("split is capped at 500 boundaries"));
    }

    let out_s = args.output.to_string_lossy().into_owned();
    let template: PathBuf = if out_s.contains('%') {
        PathBuf::from(out_s)
    } else {
        let ext = args
            .output
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("mp4");
        let stem = args
            .output
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "part".into());
        args.output
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(format!("{stem}_%02d.{ext}"))
    };

    // The muxer cuts at the first keyframe *after* a listed time, so list
    // each boundary a hair early — the forced keyframe sitting exactly on it
    // is then the chosen cut point.
    let times = cuts
        .iter()
        .map(|t| format!("{:.3}", t - 0.01))
        .collect::<Vec<_>>()
        .join(",");
    let forced = cuts
        .iter()
        .map(|t| format!("{:.3}", t))
        .collect::<Vec<_>>()
        .join(",");

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    if probe.has_video {
        argv.extend([
            "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
        ]);
        argv.extend(["-force_key_frames", &forced]);
    }
    if probe.has_audio {
        argv.extend(["-c:a", "aac"]);
    }
    argv.extend(["-f", "segment"]);
    if !times.is_empty() {
        argv.extend(["-segment_times", &times]);
    }
    argv.extend(["-reset_timestamps", "1"]);
    argv.push(&template);

    let commands = engine::commands_of(std::slice::from_ref(&argv));
    if g.dry_run {
        return Ok(
            Contract::dry_run("split", Some(paths::display(&template)), Some(probe))
                .with_commands(commands),
        );
    }
    if let Err(e) = engine::run_argvs(&[argv], g) {
        return Ok(Contract::failed("split", &e).with_commands(commands));
    }

    let parts = collect_parts(&template)?;
    if parts.is_empty() {
        return Err(Error::verification(format!(
            "split wrote no parts matching {}",
            template.display()
        )));
    }
    let first = crate::probe::probe(&parts[0], Duration::from_secs(60))?;
    let names: Vec<String> = parts.iter().map(|p| paths::display(p)).collect();
    let c = Contract::ok("split", Some(paths::display(&template)), Some(first))
        .with_commands(commands)
        .with_extra(json!({
            "every": args.every,
            "cuts": cuts,
            "parts": names,
            "count": parts.len(),
        }));
    Ok(c)
}

// List files matching the template's printf pattern: "<pre><digits><post>".
fn collect_parts(template: &Path) -> Result<Vec<PathBuf>, Error> {
    let dir = template
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let name = template
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let (pre, post) = match name.split_once('%') {
        Some((a, b)) => match b.find('.') {
            Some(d) => (a.to_string(), b[d..].to_string()),
            None => (a.to_string(), String::new()),
        },
        None => (name.clone(), String::new()),
    };
    let mut parts = Vec::new();
    for e in std::fs::read_dir(dir)? {
        let e = e?;
        let fname = e.file_name().to_string_lossy().into_owned();
        let Some(rest) = fname.strip_prefix(&pre) else {
            continue;
        };
        let Some(digits) = rest.strip_suffix(&post) else {
            continue;
        };
        if !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()) {
            parts.push(e.path());
        }
    }
    parts.sort();
    Ok(parts)
}

/// Scene-change timestamps via `select=gt(scene,THR)` + showinfo, parsed
/// from stderr at info level (ffmpeg_base pins `error`, so this argv is
/// built by hand — same pattern as `autocrop`).
fn scene_cuts(input: &Path, thr: f64, g: &Globals) -> Result<Vec<f64>, Error> {
    let vf = format!("select='gt(scene,{thr:.2})',showinfo");
    let mut argv = spawn::Argv::ffmpeg();
    argv.extend(["-y", "-hide_banner", "-nostats", "-loglevel", "info", "-i"]);
    argv.push(input);
    argv.extend(["-vf", &vf, "-f", "null", "-"]);
    let spawned = spawn::run(&argv, std::time::Duration::from_secs(600), g.progress)?;
    let spawned = spawn::require_ok(&argv, spawned)?;
    let log = spawn::stderr_str(&spawned);
    let mut out = Vec::new();
    for line in log.lines() {
        if !line.contains("showinfo") {
            continue;
        }
        for tok in line.split_whitespace() {
            if let Some(rest) = tok.strip_prefix("pts_time:") {
                if let Ok(v) = rest.parse::<f64>() {
                    out.push(v);
                }
            }
        }
    }
    out.sort_by(|a, b| a.total_cmp(b));
    out.dedup();
    if out.is_empty() {
        return Err(Error::input(
            "split --scenes found no cuts — try a lower threshold",
        ));
    }
    Ok(out)
}

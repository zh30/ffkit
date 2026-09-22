use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::json;

use crate::cli::{Globals, SplitArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::paths;

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
    let mut cuts: Vec<f64> = Vec::new();
    match (args.every, args.at.is_empty()) {
        (Some(e), true) => {
            if !(0.5..=3600.0).contains(&e) {
                return Err(Error::input("--every must be 0.5..=3600 seconds"));
            }
            let mut k = 1usize;
            while (k as f64) * e < probe.duration - 0.05 {
                cuts.push(k as f64 * e);
                k += 1;
            }
        }
        (None, false) => {
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
        (Some(_), false) => {
            return Err(Error::input("split takes --every or --at, not both"));
        }
        (None, true) => {
            return Err(Error::input("split needs --every S or --at t1,t2,..."));
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
    let dir = template.parent().unwrap_or_else(|| Path::new("."));
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

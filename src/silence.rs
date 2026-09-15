use std::path::Path;
use std::time::Duration;

use crate::error::Error;
use crate::spawn::{self, Argv};

/// Audio-only silencedetect. `fast` downmixes to 16 kHz mono so a 4K take
/// does not decode video or full-rate audio just to map pauses.
pub fn detect(
    path: &Path,
    threshold_db: f64,
    min_duration: f64,
    timeout: Duration,
    fast: bool,
) -> Result<Vec<(f64, f64)>, Error> {
    let duration = crate::probe::probe(path, timeout)?.duration;
    let mut detect = Argv::ffmpeg();
    detect.push("-i");
    detect.push(path);
    detect.push("-vn");
    let af = if fast {
        format!(
            "aformat=sample_rates=16000:channel_layouts=mono,silencedetect=noise={}dB:d={}",
            threshold_db, min_duration
        )
    } else {
        format!("silencedetect=noise={}dB:d={}", threshold_db, min_duration)
    };
    detect.extend(["-af", &af, "-f", "null", "-"]);
    let spawned = spawn::run(&detect, timeout, false)?;
    let spawned = spawn::require_ok(&detect, spawned)?;
    Ok(parse_silences(&spawn::stderr_str(&spawned), duration))
}

pub fn parse_silences(stderr: &str, duration: f64) -> Vec<(f64, f64)> {
    let mut starts = Vec::new();
    let mut ends = Vec::new();
    for line in stderr.lines() {
        if let Some(rest) = line.split("silence_start:").nth(1) {
            if let Ok(v) = rest.split_whitespace().next().unwrap_or("").parse::<f64>() {
                starts.push(v);
            }
        }
        if let Some(rest) = line.split("silence_end:").nth(1) {
            let tok = rest.split('|').next().unwrap_or(rest);
            if let Ok(v) = tok.split_whitespace().next().unwrap_or("").parse::<f64>() {
                ends.push(v);
            }
        }
    }
    let mut out = Vec::new();
    let n = starts.len().min(ends.len());
    for i in 0..n {
        if ends[i] > starts[i] {
            out.push((starts[i], ends[i]));
        }
    }
    if starts.len() > ends.len() {
        let s = *starts.last().unwrap();
        if duration > s {
            out.push((s, duration));
        }
    }
    out
}

pub fn keep_ranges(duration: f64, silences: &[(f64, f64)], pad: f64) -> Vec<(f64, f64)> {
    let mut keeps = Vec::new();
    let mut t = 0.0;
    for &(raw_s, raw_e) in silences {
        let s = (raw_s + pad).min(raw_e);
        let e = (raw_e - pad).max(s);
        if s > t + 0.02 {
            keeps.push((t, s));
        }
        t = e.max(t);
    }
    if duration > t + 0.02 {
        keeps.push((t, duration));
    }
    keeps
}

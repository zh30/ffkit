//! Signal-level measurements of a finished file, for pipeline `expect`.
//!
//! These read the decoded output with ffmpeg's own analyzers (ebur128,
//! blackdetect, freezedetect, ssim), never the verb that wrote it, so a
//! verb bug cannot also hide its own symptom.

use std::path::Path;
use std::time::Duration;

use crate::error::Error;
use crate::spawn::{self, Argv};

#[derive(Debug, Default, PartialEq)]
pub struct Signal {
    /// Integrated loudness, LUFS
    pub lufs: Option<f64>,
    /// True peak, dBTP
    pub true_peak: Option<f64>,
    /// `(start, end)` stretches of black at least `black_min` long
    pub black: Vec<(f64, f64)>,
    /// `(start, end)` stretches of frozen picture at least `freeze_min` long
    pub freeze: Vec<(f64, f64)>,
}

pub struct Ask {
    pub loudness: bool,
    pub black_min: Option<f64>,
    pub freeze_min: Option<f64>,
}

/// One decode pass over `path` running only the analyzers `ask` needs.
pub fn signal(path: &Path, ask: &Ask, timeout: Duration) -> Result<Signal, Error> {
    let mut legs = Vec::new();
    let mut maps = Vec::new();
    let mut vchain = Vec::new();
    if let Some(d) = ask.black_min {
        vchain.push(format!("blackdetect=d={d:.3}:pic_th=0.98"));
    }
    if let Some(d) = ask.freeze_min {
        vchain.push(format!("freezedetect=d={d:.3}"));
    }
    if !vchain.is_empty() {
        legs.push(format!("[0:v:0]{}[v]", vchain.join(",")));
        maps.push("[v]");
    }
    if ask.loudness {
        legs.push("[0:a:0]ebur128=peak=true[a]".to_string());
        maps.push("[a]");
    }
    if legs.is_empty() {
        return Ok(Signal::default());
    }
    let mut argv = Argv::ffmpeg();
    // analyzer lines log at info level — no -loglevel error here
    argv.extend(["-nostats", "-i"]);
    argv.push(path);
    argv.extend(["-filter_complex".to_string(), legs.join(";")]);
    for m in maps {
        argv.extend(["-map", m]);
    }
    argv.extend(["-f", "null", "-"]);
    let spawned = spawn::run(&argv, timeout, false)?;
    let stderr = String::from_utf8_lossy(&spawned.stderr).into_owned();
    if !spawned.status_ok {
        return Err(Error::ffmpeg(stderr));
    }
    Ok(parse_signal(&stderr))
}

pub fn parse_signal(stderr: &str) -> Signal {
    let mut s = Signal::default();
    let mut freeze_start: Option<f64> = None;
    // ebur128 prints a running log, then a Summary block; only trust the summary
    let mut in_summary = false;
    for line in stderr.lines() {
        let t = line.trim();
        if t.ends_with("Summary:") {
            in_summary = true;
        }
        if in_summary {
            if let Some(v) = t.strip_prefix("I:") {
                s.lufs = num(v);
            }
            if let Some(v) = t.strip_prefix("Peak:") {
                // 4.x prints dBFS here, 5.x+ prints dBTP — same value
                s.true_peak = num(v);
            }
        }
        if let Some(rest) = t.split("black_start:").nth(1) {
            let start = rest.split_whitespace().next().and_then(|v| v.parse().ok());
            let end = t
                .split("black_end:")
                .nth(1)
                .and_then(|r| r.split_whitespace().next())
                .and_then(|v| v.parse().ok());
            if let (Some(a), Some(b)) = (start, end) {
                s.black.push((a, b));
            }
        }
        if let Some(v) = t.split("lavfi.freezedetect.freeze_start:").nth(1) {
            freeze_start = num(v);
        }
        if let Some(v) = t.split("lavfi.freezedetect.freeze_end:").nth(1) {
            if let (Some(a), Some(b)) = (freeze_start.take(), num(v)) {
                s.freeze.push((a, b));
            }
        }
    }
    // a freeze that runs to EOF never logs freeze_end
    if let Some(a) = freeze_start {
        s.freeze.push((a, f64::INFINITY));
    }
    s
}

/// Mean SSIM of `out` against `reference`, out scaled onto the reference.
pub fn ssim(out: &Path, reference: &Path, timeout: Duration) -> Result<f64, Error> {
    let mut argv = Argv::ffmpeg();
    argv.extend(["-nostats", "-i"]);
    argv.push(out);
    argv.push("-i");
    argv.push(reference);
    argv.extend([
        "-filter_complex",
        "[0:v:0][1:v:0]scale2ref[o][r];[o][r]ssim",
        "-f",
        "null",
        "-",
    ]);
    let spawned = spawn::run(&argv, timeout, false)?;
    let stderr = String::from_utf8_lossy(&spawned.stderr).into_owned();
    if !spawned.status_ok {
        return Err(Error::ffmpeg(stderr));
    }
    parse_ssim(&stderr).ok_or_else(|| Error::verification("no SSIM line in ffmpeg output"))
}

pub fn parse_ssim(stderr: &str) -> Option<f64> {
    stderr
        .lines()
        .rev()
        .find_map(|l| num(l.split(" All:").nth(1)?))
}

fn num(s: &str) -> Option<f64> {
    s.split_whitespace().next()?.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const EBUR: &str = "\
[Parsed_ebur128_0 @ 0x1] t: 0.2  TARGET:-23 LUFS    M: -60.1 S:-120.7     I: -60.1 LUFS       LRA:   0.0 LU
[Parsed_ebur128_0 @ 0x1] Summary:

  Integrated loudness:
    I:         -14.2 LUFS
    Threshold: -24.4 LUFS

  Loudness range:
    LRA:         3.1 LU

  True peak:
    Peak:       -1.4 dBFS
";

    #[test]
    fn ebur128_summary_not_running_log() {
        let s = parse_signal(EBUR);
        assert_eq!(s.lufs, Some(-14.2));
        assert_eq!(s.true_peak, Some(-1.4));
    }

    #[test]
    fn black_and_freeze_ranges() {
        let s = parse_signal(
            "[blackdetect @ 0x2] black_start:0 black_end:1.5 black_duration:1.5\n\
             [freezedetect @ 0x3] lavfi.freezedetect.freeze_start: 2.04\n\
             [freezedetect @ 0x3] lavfi.freezedetect.freeze_duration: 1.2\n\
             [freezedetect @ 0x3] lavfi.freezedetect.freeze_end: 3.24\n\
             [freezedetect @ 0x3] lavfi.freezedetect.freeze_start: 5\n",
        );
        assert_eq!(s.black, vec![(0.0, 1.5)]);
        assert_eq!(s.freeze, vec![(2.04, 3.24), (5.0, f64::INFINITY)]);
        assert_eq!(s.lufs, None);
    }

    #[test]
    fn ssim_all_line() {
        let l =
            "[Parsed_ssim_1 @ 0x4] SSIM Y:0.991 (20.4) U:0.99 (20) V:0.99 (20) All:0.9902 (20.1)";
        assert_eq!(parse_ssim(l), Some(0.9902));
    }
}

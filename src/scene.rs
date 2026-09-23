use std::path::Path;
use std::time::Duration;

use crate::error::Error;
use crate::spawn::{self, Argv};

/// Scene-change times via ffmpeg's `select=gt(scene,T)` score + `metadata=print`
/// pts_time lines on stderr — the same detector thumb --scenes uses.
pub fn cut_times(path: &Path, threshold: f64, timeout: Duration) -> Result<Vec<f64>, Error> {
    let mut a = Argv::ffmpeg();
    a.push("-i");
    a.push(path);
    a.extend([
        "-vf",
        &format!("select='gt(scene,{threshold})',metadata=mode=print"),
        "-an",
        "-f",
        "null",
        "-",
    ]);
    let spawned = spawn::run(&a, timeout, false)?;
    let spawned = spawn::require_ok(&a, spawned)?;
    Ok(parse_pts(&spawn::stderr_str(&spawned)))
}

fn parse_pts(stderr: &str) -> Vec<f64> {
    stderr
        .lines()
        .filter_map(|l| {
            l.split("pts_time:")
                .nth(1)?
                .split_whitespace()
                .next()?
                .parse()
                .ok()
        })
        .collect()
}

/// Split keep ranges at each scene cut strictly inside them.
pub fn split_at(keeps: &[(f64, f64)], cuts: &[f64]) -> Vec<(f64, f64)> {
    let mut out = Vec::with_capacity(keeps.len() + cuts.len());
    for &(s, e) in keeps {
        let mut start = s;
        for &c in cuts {
            if c > start + 0.01 && c < e - 0.01 {
                out.push((start, c));
                start = c;
            }
        }
        out.push((start, e));
    }
    out
}

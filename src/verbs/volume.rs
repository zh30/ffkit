use serde_json::json;

use crate::cli::{Globals, VolumeArgs};
use crate::contract::{Contract, Status};
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::spawn::{self, Argv};

pub fn run(args: VolumeArgs, g: &Globals) -> Result<Contract, Error> {
    if !(-24.0..=12.0).contains(&args.db) {
        return Err(Error::input("--db must be -24..=12"));
    }
    if args.db == 0.0 {
        return Err(Error::input("--db 0 is a no-op; use a non-zero gain"));
    }
    if args.dur.is_some_and(|d| d <= 0.0) {
        return Err(Error::input("--dur must be positive"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_audio {
        return Err(Error::input("volume: input has no audio stream"));
    }

    let af = match (&args.at, args.dur) {
        (Some(at), dur) => {
            let start = crate::time::resolve_at(at, dur, probe.duration)?;
            if !(0.0..probe.duration).contains(&start) {
                return Err(Error::input("--at is outside the input"));
            }
            let end = dur.map(|d| start + d);
            match end {
                Some(e) if e < probe.duration => {
                    format!("volume={}dB:enable='between(t,{start:.3},{e:.3})'", args.db)
                }
                _ => format!("volume={}dB:enable='gte(t,{start:.3})'", args.db),
            }
        }
        (None, Some(_)) => return Err(Error::input("--dur needs --at")),
        (None, None) => format!("volume={}dB", args.db),
    };
    let af = if let Some(tp) = args.limit {
        if !(-30.0..=0.0).contains(&tp) {
            return Err(Error::input("--limit must be -30..=0 dBTP"));
        }
        // brickwall ceiling after the gain so the boost can't clip
        let lin = 10f64.powf(tp / 20.0);
        format!("{af},alimiter=limit={lin:.4}:level=false")
    } else {
        af
    };

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-af", &af, "-c:a", "aac"]);
    if probe.has_video {
        argv.extend(["-c:v", "copy"]);
    }
    argv.push(&args.output);

    let c = engine::write_job("volume", &[&args.input], &args.output, vec![argv], g)?;
    let mut extra = json!({ "db": args.db });
    if let Some(at) = &args.at {
        extra["at"] = json!(at);
        if let Some(d) = args.dur {
            extra["dur"] = json!(d);
        }
    }
    if matches!(c.status, Status::Ok) {
        if let Ok(m) = mean_volume(&args.output, g.timeout) {
            extra["mean_volume"] = json!(m);
        }
    }
    Ok(c.with_extra(extra))
}

pub fn mean_volume(path: &std::path::Path, timeout: std::time::Duration) -> Result<f64, Error> {
    let mut argv = Argv::ffmpeg();
    argv.push("-i");
    argv.push(path);
    argv.extend(["-af", "volumedetect", "-vn", "-f", "null", "-"]);
    let spawned = spawn::run(&argv, timeout, false)?;
    let spawned = spawn::require_ok(&argv, spawned)?;
    parse_mean_volume(&spawn::stderr_str(&spawned))
        .ok_or_else(|| Error::ffmpeg("volumedetect did not print mean_volume"))
}

pub(crate) fn parse_mean_volume(stderr: &str) -> Option<f64> {
    for line in stderr.lines() {
        if let Some(rest) = line.split("mean_volume:").nth(1) {
            let tok = rest.split_whitespace().next()?;
            return tok.parse().ok();
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_mean() {
        let log = "[Parsed_volumedetect_0 @ 0x] mean_volume: -18.52 dB\n";
        assert!((parse_mean_volume(log).unwrap() + 18.52).abs() < 0.001);
    }
}

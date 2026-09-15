use serde_json::json;

use crate::cli::{Globals, SpeedArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: SpeedArgs, g: &Globals) -> Result<Contract, Error> {
    let factor = args.factor;
    if !factor.is_finite() || !(0.25..=8.0).contains(&factor) {
        return Err(Error::input(
            "--factor must be between 0.25 and 8 (e.g. 2 = twice as fast)",
        ));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_video && !probe.has_audio {
        return Err(Error::input("speed: input has no streams"));
    }

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);

    if probe.has_video {
        argv.extend(["-filter:v", &format!("setpts=PTS/{factor}")]);
        argv.extend([
            "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
        ]);
    } else {
        argv.push("-vn");
    }
    if probe.has_audio {
        argv.extend(["-filter:a", &atempo_chain(factor)?]);
        argv.extend(["-c:a", "aac"]);
    } else {
        argv.push("-an");
    }
    argv.push(&args.output);

    let mut c = engine::write_job("speed", &[&args.input], &args.output, vec![argv], g)?;
    c = c.with_extra(json!({
        "factor": factor,
        "keep_pitch": true,
    }));
    Ok(c)
}

/// atempo accepts 0.5..=2.0 per hop; chain hops for 0.25× / 4× / 8×.
pub(crate) fn atempo_chain(factor: f64) -> Result<String, Error> {
    if !factor.is_finite() || factor <= 0.0 {
        return Err(Error::input("speed factor must be positive"));
    }
    let mut remaining = factor;
    let mut parts: Vec<String> = Vec::new();
    while remaining > 2.0 + 1e-6 {
        parts.push("atempo=2".into());
        remaining /= 2.0;
    }
    while remaining < 0.5 - 1e-6 {
        parts.push("atempo=0.5".into());
        remaining *= 2.0;
    }
    parts.push(format!("atempo={remaining:.5}"));
    Ok(parts.join(","))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atempo_2x_is_single_hop() {
        assert_eq!(atempo_chain(2.0).unwrap(), "atempo=2.00000");
    }

    #[test]
    fn atempo_4x_chains() {
        let s = atempo_chain(4.0).unwrap();
        assert!(s.contains("atempo=2"), "{s}");
        assert_eq!(s.matches("atempo=").count(), 2);
    }

    #[test]
    fn atempo_quarter_chains() {
        let s = atempo_chain(0.25).unwrap();
        assert!(s.contains("atempo=0.5"), "{s}");
    }
}

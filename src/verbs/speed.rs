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
    if args.dur.is_some() && args.at.is_none() {
        return Err(Error::input("--dur needs --at"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_video && !probe.has_audio {
        return Err(Error::input("speed: input has no streams"));
    }

    if args.at.is_some() {
        return windowed(args, factor, &probe, g);
    }

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);

    if args.interp && factor >= 1.0 {
        return Err(Error::input("--interp only helps slow-mo (factor < 1)"));
    }
    if probe.has_video {
        let vf = if args.interp {
            // Upsample fps by blend-interpolating so the stretch stays smooth:
            // src_fps/factor real frames per source second, then re-time.
            let out_fps = (probe.fps.unwrap_or(30.0) / factor).min(120.0).max(1.0);
            format!("minterpolate=fps={out_fps:.3}:mi_mode=blend,setpts=PTS/{factor}")
        } else {
            format!("setpts=PTS/{factor}")
        };
        argv.extend(["-filter:v", &vf]);
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
        "interp": args.interp,
    }));
    Ok(c)
}

/// Speed-ramp: only the [at, at+dur] window (or [at, end]) gets the factor,
/// via a 3-segment trim/concat so the rest of the clip is untouched.
fn windowed(
    args: SpeedArgs,
    factor: f64,
    probe: &crate::probe::Probe,
    g: &Globals,
) -> Result<Contract, Error> {
    let at = crate::time::parse_time(args.at.as_deref().unwrap())?;
    if !(0.0..probe.duration - 0.1).contains(&at) {
        return Err(Error::input("--at must land inside the input"));
    }
    let end = match args.dur {
        Some(d) if d <= 0.0 => return Err(Error::input("--dur must be positive")),
        Some(d) => (at + d).min(probe.duration),
        None => probe.duration,
    };
    if end - at < 0.1 {
        return Err(Error::input("speed window is under 0.1s"));
    }
    let has_v = probe.has_video;
    let has_a = probe.has_audio;
    let mut seg: Vec<String> = Vec::new();
    // video segments: [0,at) normal, [at,end) sped, [end,) normal
    if has_v {
        seg.push(format!("[0:v]trim=0:{at:.3},setpts=PTS-STARTPTS[v0]"));
        seg.push(format!(
            "[0:v]trim={at:.3}:{end:.3},setpts=(PTS-STARTPTS)/{factor}[v1]"
        ));
        seg.push(format!("[0:v]trim=start={end:.3},setpts=PTS-STARTPTS[v2]"));
    }
    if has_a {
        let chain = atempo_chain(factor)?;
        seg.push(format!("[0:a]atrim=0:{at:.3},asetpts=PTS-STARTPTS[a0]"));
        seg.push(format!(
            "[0:a]atrim={at:.3}:{end:.3},asetpts=PTS-STARTPTS,{chain}[a1]"
        ));
        seg.push(format!(
            "[0:a]atrim=start={end:.3},asetpts=PTS-STARTPTS[a2]"
        ));
    }
    let (nv, na) = (if has_v { 1 } else { 0 }, if has_a { 1 } else { 0 });
    let mut ins = String::new();
    for i in 0..3 {
        if has_v {
            ins.push_str(&format!("[v{i}]"));
        }
        if has_a {
            ins.push_str(&format!("[a{i}]"));
        }
    }
    let mut outs = String::new();
    if has_v {
        outs.push_str("[vout]");
    }
    if has_a {
        outs.push_str("[aout]");
    }
    seg.push(format!("{ins}concat=n=3:v={nv}:a={na}{outs}"));
    let fc = seg.join(";");

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-filter_complex", &fc]);
    if has_v {
        argv.extend(["-map", "[vout]"]);
    }
    if has_a {
        argv.extend(["-map", "[aout]", "-c:a", "aac"]);
    }
    if has_v {
        argv.extend([
            "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
        ]);
    }
    argv.push(&args.output);

    let c = engine::write_job("speed", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "factor": factor,
        "at": at,
        "dur": end - at,
        "keep_pitch": true,
    })))
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

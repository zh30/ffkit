use serde_json::json;

use crate::cli::{BleepArgs, Globals};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

/// Mute a window under a sine beep — the classic swear censor. The original
/// track ducks to silence inside the window; a `sine` source of the same
/// length mixes on top so the gap reads as an intentional censor.
pub fn run(args: BleepArgs, g: &Globals) -> Result<Contract, Error> {
    if args.dur <= 0.0 {
        return Err(Error::input("--dur must be positive"));
    }
    if !(0.0..=1.0).contains(&args.level) {
        return Err(Error::input("--level must be 0..=1"));
    }
    if !(20.0..=20000.0).contains(&args.freq) {
        return Err(Error::input("--freq must be 20..=20000 Hz"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_audio {
        return Err(Error::input("bleep: input has no audio stream"));
    }
    // --at takes a comma list: censor several words in one pass.
    let mut windows: Vec<(f64, f64)> = Vec::new();
    for part in args.at.split(',') {
        let start = crate::time::resolve_at(part.trim(), Some(args.dur), probe.duration)?;
        if !(0.0..probe.duration).contains(&start) {
            return Err(Error::input("--at is outside the input"));
        }
        windows.push((start, (start + args.dur).min(probe.duration)));
    }
    if windows.len() > 20 {
        return Err(Error::input("bleep is capped at 20 windows"));
    }
    windows.sort_by(|a, b| a.0.total_cmp(&b.0));

    let mute = windows
        .iter()
        .map(|(s, e)| format!("between(t,{s:.3},{e:.3})"))
        .collect::<Vec<_>>()
        .join("+");
    let mut fc = format!("[0:a]volume=0:enable='{mute}'[dry];");
    for (i, (s, _)) in windows.iter().enumerate() {
        fc.push_str(&format!(
            "sine=frequency={freq}:duration={dur}[tone{i}];\
             [tone{i}]adelay={ms}:all=1,volume={lvl}[bp{i}];",
            freq = args.freq,
            dur = args.dur,
            lvl = args.level,
            ms = (s * 1000.0).round() as u64,
        ));
    }
    fc.push_str("[dry]");
    for i in 0..windows.len() {
        fc.push_str(&format!("[bp{i}]"));
    }
    fc.push_str(&format!(
        "amix=inputs={}:duration=first:normalize=0[aout]",
        windows.len() + 1
    ));
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-filter_complex", &fc, "-map", "0:v?", "-map", "[aout]"]);
    if probe.has_video {
        argv.extend(["-c:v", "copy"]);
    }
    argv.extend(["-c:a", "aac"]);
    argv.push(&args.output);

    let c = engine::write_job("bleep", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "at": args.at,
        "dur": args.dur,
        "freq": args.freq,
        "level": args.level,
    })))
}

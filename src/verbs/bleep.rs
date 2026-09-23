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
    let start = crate::time::resolve_at(&args.at, Some(args.dur), probe.duration)?;
    if !(0.0..probe.duration).contains(&start) {
        return Err(Error::input("--at is outside the input"));
    }
    let end = (start + args.dur).min(probe.duration);

    let fc = format!(
        "[0:a]volume=0:enable='between(t,{start:.3},{end:.3})'[dry];\
         sine=frequency={freq}:duration={dur}[tone];\
         [tone]adelay={ms}:all=1,volume={lvl}[bp];\
         [dry][bp]amix=inputs=2:duration=first:normalize=0[aout]",
        freq = args.freq,
        dur = args.dur,
        lvl = args.level,
        ms = (start * 1000.0).round() as u64,
    );
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

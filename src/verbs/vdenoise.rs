use serde_json::json;

use crate::cli::{Globals, VdenoiseArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: VdenoiseArgs, g: &Globals) -> Result<Contract, Error> {
    if !(0.5..=30.0).contains(&args.strength) {
        return Err(Error::input("--strength must be 0.5..=30"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "vdenoise")?;

    let s = args.strength;
    // --at/--dur: nlmeans accepts timeline `enable`, so a window is a flag
    // on the filter, not a split graph — cheap to keep the rest untouched.
    let vf = match &args.at {
        Some(raw) => {
            let at = crate::time::resolve_at(raw, args.dur, probe.duration)?;
            if !(0.0..probe.duration).contains(&at) {
                return Err(Error::input("--at is outside the input"));
            }
            match args.dur {
                Some(d) if at + d < probe.duration => {
                    format!("nlmeans=s={s:.1}:enable='between(t,{at:.3},{:.3})'", at + d)
                }
                _ => format!("nlmeans=s={s:.1}:enable='gte(t,{at:.3})'"),
            }
        }
        None => {
            if args.dur.is_some() {
                return Err(Error::input("--dur needs --at"));
            }
            format!("nlmeans=s={s:.1}")
        }
    };
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend([
        "-vf", &vf, "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
    ]);
    if probe.has_audio {
        argv.extend(["-c:a", "copy"]);
    }
    argv.push(&args.output);

    let c = engine::write_job("vdenoise", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "strength": s,
        "filter": "nlmeans",
    })))
}

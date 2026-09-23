use serde_json::json;

use crate::cli::{Globals, WbArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: WbArgs, g: &Globals) -> Result<Contract, Error> {
    if !(0.0..=1.0).contains(&args.strength) {
        return Err(Error::input("--strength must be 0..1"));
    }
    if !(0.0..=1.0).contains(&args.independence) {
        return Err(Error::input("--independence must be 0..1"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "wb")?;

    // normalize stretches each channel's histogram to full range — with
    // independence=1 that is per-channel, which removes color casts exactly
    // like a white-balance pick on neutral gray; smoothing temporal-averages
    // the range so the correction doesn't breathe frame to frame
    let chain = format!(
        "normalize=blackpt=black:whitept=white:smoothing={}:independence={:.3}:strength={:.3}",
        args.smooth, args.independence, args.strength
    );
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    match &args.at {
        Some(s) => {
            let en = crate::time::enable_expr(s, args.dur, probe.duration)?.replace("(t,", "(T,");
            let fc =
                format!("[0:v]split[m][f];[f]{chain}[p];[m][p]blend=all_expr='if({en},B,A)'[v]");
            argv.extend(["-filter_complex", &fc, "-map", "[v]", "-map", "0:a?"]);
        }
        None => {
            if args.dur.is_some() {
                return Err(Error::input("--dur needs --at"));
            }
            argv.extend(["-vf", &chain]);
        }
    }
    argv.extend([
        "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
    ]);
    if probe.has_audio {
        argv.extend(["-c:a", "copy"]);
    }
    argv.push(&args.output);

    let c = engine::write_job("wb", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "strength": args.strength,
        "independence": args.independence,
        "smooth": args.smooth,
        "filter": "normalize",
    })))
}

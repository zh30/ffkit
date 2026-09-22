use serde_json::json;

use crate::cli::{Globals, SharpenArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: SharpenArgs, g: &Globals) -> Result<Contract, Error> {
    if !(0.3..=2.0).contains(&args.amount) {
        return Err(Error::input("--amount must be 0.3..=2"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "sharpen")?;

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    let vf = match &args.at {
        Some(s) => {
            let at = crate::time::parse_time(s)?;
            if !(0.0..probe.duration).contains(&at) {
                return Err(Error::input("--at is outside the input"));
            }
            match args.dur {
                Some(d) if at + d < probe.duration => format!(
                    "unsharp=5:5:{}:5:5:0.0:enable='between(t,{at:.3},{:.3})'",
                    args.amount,
                    at + d
                ),
                _ => format!(
                    "unsharp=5:5:{}:5:5:0.0:enable='gte(t,{at:.3})'",
                    args.amount
                ),
            }
        }
        None => {
            if args.dur.is_some() {
                return Err(Error::input("--dur needs --at"));
            }
            format!("unsharp=5:5:{}:5:5:0.0", args.amount)
        }
    };
    argv.extend([
        "-vf", &vf, "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
    ]);
    if probe.has_audio {
        argv.extend(["-c:a", "copy"]);
    }
    argv.push(&args.output);

    let c = engine::write_job("sharpen", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "amount": args.amount,
        "filter": "unsharp",
    })))
}

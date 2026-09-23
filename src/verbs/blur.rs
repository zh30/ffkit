use serde_json::json;

use crate::cli::{BlurArgs, Globals};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: BlurArgs, g: &Globals) -> Result<Contract, Error> {
    if !(0.5..=20.0).contains(&args.sigma) {
        return Err(Error::input("--sigma must be 0.5..=20"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "blur")?;

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    let vf = match &args.at {
        Some(s) => {
            let at = crate::time::resolve_at(s, args.dur, probe.duration)?;
            if !(0.0..probe.duration).contains(&at) {
                return Err(Error::input("--at is outside the input"));
            }
            match args.dur {
                Some(d) if at + d < probe.duration => format!(
                    "gblur=sigma={}:enable='between(t,{at:.3},{:.3})'",
                    args.sigma,
                    at + d
                ),
                _ => format!("gblur=sigma={}:enable='gte(t,{at:.3})'", args.sigma),
            }
        }
        None => {
            if args.dur.is_some() {
                return Err(Error::input("--dur needs --at"));
            }
            format!("gblur=sigma={}", args.sigma)
        }
    };
    argv.extend([
        "-vf", &vf, "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
    ]);
    if probe.has_audio {
        argv.extend(["-c:a", "copy"]);
    }
    argv.push(&args.output);

    let c = engine::write_job("blur", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "sigma": args.sigma,
        "filter": "gblur",
    })))
}

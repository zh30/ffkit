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
        Some(s) => format!(
            "gblur=sigma={}:enable='{}'",
            args.sigma,
            crate::time::enable_expr(s, args.dur, probe.duration)?
        ),
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

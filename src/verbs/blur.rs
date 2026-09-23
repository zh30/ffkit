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
    let base = match args.engine.unwrap_or(crate::cli::BlurEngine::Gblur) {
        crate::cli::BlurEngine::Gblur => format!("gblur=sigma={}", args.sigma),
        crate::cli::BlurEngine::Directional => {
            let a = args.angle.unwrap_or(45.0);
            if !(0.0..=360.0).contains(&a) {
                return Err(Error::input("--angle must be 0..360"));
            }
            format!("dblur=angle={a}:radius={:.1}", args.sigma * 4.0)
        }
    };
    let vf = match &args.at {
        Some(s) => format!(
            "{base}:enable='{}'",
            crate::time::enable_expr(s, args.dur, probe.duration)?
        ),
        None => {
            if args.dur.is_some() {
                return Err(Error::input("--dur needs --at"));
            }
            base
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
        "filter": match args.engine.unwrap_or(crate::cli::BlurEngine::Gblur) {
            crate::cli::BlurEngine::Gblur => "gblur",
            crate::cli::BlurEngine::Directional => "dblur",
        },
    })))
}

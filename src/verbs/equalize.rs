use serde_json::json;

use crate::cli::{EqualizeArgs, Globals};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

/// Auto-contrast for flat/washed footage: `histeq` histogram equalization.
pub fn run(args: EqualizeArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "equalize")?;
    if !(0.05..=1.0).contains(&args.strength) {
        return Err(Error::input("--strength must be 0.05..=1.0"));
    }
    if !(0.05..=1.0).contains(&args.intensity) {
        return Err(Error::input("--intensity must be 0.05..=1.0"));
    }

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    let base = format!(
        "histeq=strength={s:.3}:intensity={i:.3}",
        s = args.strength,
        i = args.intensity
    );
    let vf = match &args.at {
        Some(raw) => format!(
            "{base}:enable='{}'",
            crate::time::enable_expr(raw, args.dur, probe.duration)?
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

    let c = engine::write_job("equalize", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "strength": args.strength,
        "intensity": args.intensity,
        "filter": "histeq",
    })))
}

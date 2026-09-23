use serde_json::json;

use crate::cli::{DebandArgs, Globals};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

/// Smooth gradient banding (sky/backdrop steps on compressed phone footage).
pub fn run(args: DebandArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "deband")?;
    if !(0.05..=1.0).contains(&args.strength) {
        return Err(Error::input("--strength must be 0.05..=1.0"));
    }
    if !(4..=32).contains(&args.radius) {
        return Err(Error::input("--radius must be 4..=32"));
    }
    // strength 0..1 maps to gradfun 0.51..8: default 1.2 is subtle, heavy
    // banding needs ~4-8 without smearing texture elsewhere.
    let s = 0.51 + args.strength * 7.49;

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    let vf = match &args.at {
        Some(raw) => format!(
            "gradfun=strength={s:.2}:radius={r}:enable='{en}'",
            r = args.radius,
            en = crate::time::enable_expr(raw, args.dur, probe.duration)?
        ),
        None => {
            if args.dur.is_some() {
                return Err(Error::input("--dur needs --at"));
            }
            format!("gradfun=strength={s:.2}:radius={r}", r = args.radius)
        }
    };
    argv.extend([
        "-vf", &vf, "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
    ]);
    if probe.has_audio {
        argv.extend(["-c:a", "copy"]);
    }
    argv.push(&args.output);

    let c = engine::write_job("deband", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "strength": args.strength,
        "radius": args.radius,
        "filter": "gradfun",
    })))
}

use crate::cli::LegalizeArgs;
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use serde_json::json;

/// Clamp luma to broadcast-safe levels (limiter): illegal blacks below 16 and
/// clipped whites above 235 get pinned — YouTube/broadcast QC, tapes, and
/// phone footage that blows past the legal range.
pub fn run(args: LegalizeArgs, g: &crate::cli::Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "legalize")?;
    if args.min >= args.max {
        return Err(Error::input("--min must be below --max"));
    }
    // planes=1 = luma only: chroma planes live in a different legal range
    let mut vf = format!("limiter=min={}:max={}:planes=1", args.min, args.max);
    // --flash: damps seizure-risk luminance swings upstream of the clamp
    // (whole-file safety pass, so it stays out of any --at window)
    if args.flash {
        let t = args.flash_threshold.unwrap_or(1.0);
        if !(0.1..=10.0).contains(&t) {
            return Err(Error::input("--flash-threshold must be 0.1..=10"));
        }
        vf = format!("photosensitivity=threshold={t:.2},{vf}");
    }
    if let Some(s) = &args.at {
        let win = crate::time::enable_expr(s, args.dur, probe.duration)?;
        vf = format!("{vf}:enable='{win}'");
    } else if args.dur.is_some() {
        return Err(Error::input("--dur needs --at"));
    }

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
    let c = engine::write_job("legalize", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "min": args.min,
        "max": args.max,
        "flash": args.flash,
        "filter": vf,
    })))
}

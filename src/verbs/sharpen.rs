use serde_json::json;

use crate::cli::{Globals, SharpenArgs, SharpenEngine};
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
    // cas strength is 0-1; map --amount 0.3-2.0 → 0.15-1.0, capped at 1.
    let base = match args.engine {
        SharpenEngine::Unsharp => format!("unsharp=5:5:{}:5:5:0.0", args.amount),
        SharpenEngine::Cas => format!("cas=strength={:.2}", (args.amount / 2.0).min(1.0)),
        // maskedclamp graph built below — placeholder
        SharpenEngine::Halo => String::new(),
    };
    if matches!(args.engine, SharpenEngine::Halo) {
        // halo-free sharpen: unsharp the original, then clamp each pixel to
        // the blurred base ±strength — edges can't overshoot into halos.
        // maskedclamp takes 3 inputs (clamped, dark ref, bright ref).
        let o = (args.amount * 6.0) as u32;
        let en = match &args.at {
            Some(s) => format!(
                ":enable='{}'",
                crate::time::enable_expr(s, args.dur, probe.duration)?
            ),
            None => {
                if args.dur.is_some() {
                    return Err(Error::input("--dur needs --at"));
                }
                String::new()
            }
        };
        let fc = format!(
            "[0:v]split[m][bb];[bb]gblur=sigma=3,split[bd][br];[m]unsharp=5:5:{a}[sh];[sh][bd][br]maskedclamp=overshoot={o}:undershoot={o}{en}[v]",
            a = args.amount
        );
        argv.extend(["-filter_complex", &fc, "-map", "[v]", "-map", "0:a?"]);
        argv.extend([
            "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
        ]);
        if probe.has_audio {
            argv.extend(["-c:a", "copy"]);
        }
        argv.push(&args.output);
        let c = engine::write_job("sharpen", &[&args.input], &args.output, vec![argv], g)?;
        return Ok(c.with_extra(json!({
            "amount": args.amount,
            "engine": "halo",
        })));
    }
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

    let c = engine::write_job("sharpen", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "amount": args.amount,
        "filter": match args.engine {
            SharpenEngine::Unsharp => "unsharp",
            SharpenEngine::Cas => "cas",
            SharpenEngine::Halo => "maskedclamp",
        },
    })))
}

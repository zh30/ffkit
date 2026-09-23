use serde_json::json;

use crate::cli::{AmplifyArgs, Globals};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::time::enable_expr;

pub fn run(args: AmplifyArgs, g: &Globals) -> Result<Contract, Error> {
    if !(1.0..=50.0).contains(&args.amount) {
        return Err(Error::input("--amount must be 1..=50"));
    }
    if !(1..=63).contains(&args.radius) {
        return Err(Error::input("--radius must be 1..=63"));
    }
    if args.threshold > 255 {
        return Err(Error::input("--threshold must be 0..=255"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "amplify")?;

    // amplify magnifies per-pixel diffs below `threshold` across `radius`
    // frames: subtle motion (breathing, pulses, machine shake) becomes visible
    let chain = format!(
        "amplify=radius={r}:factor={f:.1}:threshold={t}",
        r = args.radius,
        f = args.amount,
        t = args.threshold
    );
    let fc = match &args.at {
        Some(a) => {
            let en = enable_expr(a, args.dur, probe.duration)?;
            format!(
                "[0:v]split[m][f];[f]{chain}[p];[m][p]blend=all_expr='if({en},B,A)'[v]",
                en = en.replace("(t,", "(T,")
            )
        }
        None => {
            if args.dur.is_some() {
                return Err(Error::input("--dur needs --at"));
            }
            format!("[0:v]{chain}[p];[p]copy[v]")
        }
    };

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-filter_complex", &fc, "-map", "[v]", "-map", "0:a?"]);
    argv.extend([
        "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
    ]);
    if probe.has_audio {
        argv.extend(["-c:a", "copy"]);
    }
    argv.push(&args.output);

    let c = engine::write_job("amplify", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "amount": args.amount,
        "radius": args.radius,
        "threshold": args.threshold,
        "filter": chain
    })))
}

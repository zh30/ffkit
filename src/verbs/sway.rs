use serde_json::json;

use crate::cli::{Globals, SwayArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::time::enable_expr;

pub fn run(args: SwayArgs, g: &Globals) -> Result<Contract, Error> {
    if !(0.05..=3.0).contains(&args.rate) {
        return Err(Error::input("--rate must be 0.05..=3 sway cycles/sec"));
    }
    if !(2..=80).contains(&args.px) {
        return Err(Error::input("--px must be 2..=80 pixel drift"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "sway")?;
    let (w, h) = (probe.width.unwrap_or(0), probe.height.unwrap_or(0));
    let (px, pw, ph) = (args.px, w + args.px * 2, h + args.px * 2);

    // handheld drift: pad a margin then crop per-frame x/y on a slow sine
    let chain = format!(
        "pad={pw}:{ph}:{px}:{px},crop={w}:{h}:x='{px}+{amp}*sin(2*PI*t*{r1:.3})':y='{px}+{amp}*cos(2*PI*t*{r2:.3})'",
        amp = args.px - 2,
        r1 = args.rate,
        r2 = args.rate * 1.4
    );
    let fc = match &args.at {
        Some(a) => {
            let en = enable_expr(a, args.dur, probe.duration)?;
            format!(
                "[0:v]split[m][f];[f]{chain}[s];[m][s]blend=all_expr='if({en},B,A)'[v]",
                en = en.replace("(t,", "(T,")
            )
        }
        None => {
            if args.dur.is_some() {
                return Err(Error::input("--dur needs --at"));
            }
            format!("[0:v]{chain}[s];[s]copy[v]")
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

    let c2 = engine::write_job("sway", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c2.with_extra(json!({ "rate": args.rate, "px": args.px })))
}

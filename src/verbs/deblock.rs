use serde_json::json;

use crate::cli::{DeblockArgs, Globals};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::time::enable_expr;

/// DCT block-boundary removal for heavily compressed sources (phone screen
/// recordings, re-uploaded clips). --strength scales the three detection
/// thresholds; the stock defaults are nearly a no-op so we always run the
/// strong filter variant.
pub fn run(args: DeblockArgs, g: &Globals) -> Result<Contract, Error> {
    if !(0.05..=0.95).contains(&args.strength) {
        return Err(Error::input("--strength must be 0.05..=0.95"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "deblock")?;

    let s = args.strength;
    let chain = format!("deblock=filter=strong:block=8:alpha={s:.2}:beta={s:.2}:gamma={s:.2}");
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

    let c = engine::write_job("deblock", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({ "strength": s, "filter": chain })))
}

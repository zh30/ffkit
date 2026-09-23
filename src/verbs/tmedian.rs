use serde_json::json;

use crate::cli::{Globals, TmedianArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::time::enable_expr;

/// Temporal median: each output pixel is the median of `2*radius+1` frames,
/// so anything visible for less than half the window vanishes — people
/// walking through a timelapse, cars on a tripod street shot, rain streaks.
/// Note the output is `2*radius` frames shorter at the edges.
pub fn run(args: TmedianArgs, g: &Globals) -> Result<Contract, Error> {
    if !(1..=127).contains(&args.radius) {
        return Err(Error::input("--radius must be 1..=127"));
    }
    if !(0.0..=1.0).contains(&args.percentile) {
        return Err(Error::input("--percentile must be 0..=1"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "tmedian")?;

    let chain = format!(
        "tmedian=radius={r}:percentile={p:.2}",
        r = args.radius,
        p = args.percentile
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

    let c = engine::write_job("tmedian", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "radius": args.radius,
        "percentile": args.percentile,
        "edge_frames_dropped": args.radius * 2,
        "filter": chain
    })))
}

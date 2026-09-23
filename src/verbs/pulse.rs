use serde_json::json;

use crate::cli::{Globals, PulseArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: PulseArgs, g: &Globals) -> Result<Contract, Error> {
    if !(0.1..=10.0).contains(&args.rate) {
        return Err(Error::input("--rate must be 0.1..=10 breaths/sec"));
    }
    if !(0.005..=0.3).contains(&args.depth) {
        return Err(Error::input("--depth must be 0.005..=0.3 zoom amplitude"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "pulse")?;
    let (w, h) = (probe.width.unwrap_or(0), probe.height.unwrap_or(0));
    let fps = probe.fps.unwrap_or(30.0).max(1.0);

    // breathing zoom: per-frame zoompan with a sine zoom factor
    let period_frames = (fps / args.rate).max(1.0);
    let chain = format!(
        "zoompan=z='1+{amp}*sin(2*PI*on/{per:.3})':d=1:s={w}x{h}",
        amp = args.depth,
        per = period_frames
    );

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    let fc = match &args.at {
        Some(a) => {
            let en = crate::time::enable_expr(a, args.dur, probe.duration)?;
            format!(
                "[0:v]split[m][f];[f]{chain}[pul];[m][pul]blend=all_expr='if({en},B,A)'[v]",
                en = en.replace("(t,", "(T,")
            )
        }
        None => {
            if args.dur.is_some() {
                return Err(Error::input("--dur needs --at"));
            }
            format!("[0:v]{chain}[pul];[pul]copy[v]")
        }
    };
    argv.extend(["-filter_complex", &fc, "-map", "[v]", "-map", "0:a?"]);
    argv.extend([
        "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
    ]);
    if probe.has_audio {
        argv.extend(["-c:a", "copy"]);
    }
    argv.push(&args.output);

    let c2 = engine::write_job("pulse", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c2.with_extra(json!({
        "rate": args.rate,
        "depth": args.depth,
    })))
}

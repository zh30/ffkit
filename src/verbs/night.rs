use serde_json::json;

use crate::cli::{Globals, NightArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: NightArgs, g: &Globals) -> Result<Contract, Error> {
    if !(0.0..=30.0).contains(&args.grain) {
        return Err(Error::input("--grain must be 0..=30 noise level"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "night")?;

    // night-vision: desaturate, push green mids, add grain + vignette
    let chain = format!(
        "hue=s=0,colorbalance=gm=0.55:gs=0.3:bm=-0.3,noise=alls={gr}:allf=t,vignette",
        gr = args.grain
    );
    let fc = match &args.at {
        Some(a) => {
            let en = crate::time::enable_expr(a, args.dur, probe.duration)?;
            format!(
                "[0:v]split[m][f];[f]{chain}[n];[m][n]blend=all_expr='if({en},B,A)'[v]",
                en = en.replace("(t,", "(T,")
            )
        }
        None => {
            if args.dur.is_some() {
                return Err(Error::input("--dur needs --at"));
            }
            format!("[0:v]{chain}[n];[n]copy[v]")
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

    let c2 = engine::write_job("night", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c2.with_extra(json!({ "grain": args.grain })))
}

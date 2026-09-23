use serde_json::json;

use crate::cli::{Globals, VhsArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: VhsArgs, g: &Globals) -> Result<Contract, Error> {
    if !(0.0..=3.0).contains(&args.strength) {
        return Err(Error::input("--strength must be 0..=3"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "vhs")?;

    // tape noise + chroma shift + scanlines
    let n = (8.0 * args.strength).round() as i32;
    let s = (3.0 * args.strength).round() as i32;
    let chain =
        format!("noise=alls={n}:allf=t+u,rgbashift=rh={s}:bh=-{s},drawgrid=w=iw:h=3:c=black@0.30");

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    match &args.at {
        Some(s2) => {
            // drawgrid has no timeline flag — blend the VHS branch in
            let en = crate::time::enable_expr(s2, args.dur, probe.duration)?.replace("(t,", "(T,");
            let fc =
                format!("[0:v]split[m][f];[f]{chain}[x];[m][x]blend=all_expr='if({en},B,A)'[v]");
            argv.extend(["-filter_complex", &fc, "-map", "[v]", "-map", "0:a?"]);
        }
        None => {
            if args.dur.is_some() {
                return Err(Error::input("--dur needs --at"));
            }
            argv.extend(["-vf", &chain]);
        }
    }
    argv.extend([
        "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
    ]);
    if probe.has_audio {
        argv.extend(["-c:a", "copy"]);
    }
    argv.push(&args.output);

    let c = engine::write_job("vhs", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "strength": args.strength,
        "filter": "noise+rgbashift+drawgrid",
    })))
}

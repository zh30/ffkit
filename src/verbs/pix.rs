use serde_json::json;

use crate::cli::{Globals, PixArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: PixArgs, g: &Globals) -> Result<Contract, Error> {
    if !(2.0..=64.0).contains(&args.strength) {
        return Err(Error::input("--strength must be 2..=64"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "pix")?;

    // downscale + nearest-neighbor upscale = chunky retro pixelation; the up-scale
    // targets the probed dims (iw inside a chain is the previous filter's size)
    let k = args.strength.round() as i32;
    let (ow, oh) = (probe.width.unwrap_or(0), probe.height.unwrap_or(0));
    let chain =
        format!("scale=w=iw/{k}:h=ih/{k}:flags=neighbor,scale=w={ow}:h={oh}:flags=neighbor");
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    match &args.at {
        Some(s) => {
            let en = crate::time::enable_expr(s, args.dur, probe.duration)?.replace("(t,", "(T,");
            let fc =
                format!("[0:v]split[m][f];[f]{chain}[p];[m][p]blend=all_expr='if({en},B,A)'[v]");
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

    let c = engine::write_job("pix", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "strength": args.strength,
        "filter": "neighbor-scale pair",
    })))
}

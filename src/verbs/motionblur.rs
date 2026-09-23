use serde_json::json;

use crate::cli::{Globals, MotionBlurArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: MotionBlurArgs, g: &Globals) -> Result<Contract, Error> {
    if !(2..=8).contains(&args.frames) {
        return Err(Error::input("--frames must be 2..=8"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "motionblur")?;

    // temporal average = shutter smear; tmix blends forward (lagging ghost)
    let chain = if args.frames == 2 {
        "tblend=all_mode=average".to_string()
    } else {
        format!("tmix=frames={}", args.frames)
    };

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    match &args.at {
        Some(s) => {
            let en = crate::time::enable_expr(s, args.dur, probe.duration)?.replace("(t,", "(T,");
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

    let c = engine::write_job("motionblur", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "frames": args.frames,
        "filter": chain,
    })))
}

use serde_json::json;

use crate::cli::{Globals, GlowArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: GlowArgs, g: &Globals) -> Result<Contract, Error> {
    if !(0.5..=40.0).contains(&args.strength) {
        return Err(Error::input("--strength must be 0.5..=40"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "glow")?;

    // blurred copy screen-blended back over the sharp frame = bloom
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
        "[0:v]split[a][b];[b]gblur=sigma={}[b];[a][b]blend=all_mode=screen{en}[v]",
        args.strength
    );

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

    let c = engine::write_job("glow", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "strength": args.strength,
        "filter": "gblur+screen blend",
    })))
}

use serde_json::json;

use crate::cli::{Globals, MirrorArgs, MirrorAxis};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: MirrorArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "mirror")?;

    // mirror half the frame across the center axis (dance/symmetry look)
    let mir = match args.axis {
        MirrorAxis::X => {
            "crop=iw/2:ih:0:0,split[cl][cr];[cr]hflip[crf];[cl][crf]hstack[mir];".to_string()
        }
        MirrorAxis::Y => {
            "crop=iw:ih/2:0:0,split[ct][cb];[cb]vflip[cbf];[ct][cbf]vstack[mir];".to_string()
        }
    };
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    match &args.at {
        Some(s) => {
            // blend swaps in the mirrored branch inside the window(s)
            let en = crate::time::enable_expr(s, args.dur, probe.duration)?.replace("(t,", "(T,");
            let fc = format!("[0:v]split[m][f];[f]{mir}[m][mir]blend=all_expr='if({en},B,A)'[v]");
            argv.extend(["-filter_complex", &fc, "-map", "[v]", "-map", "0:a?"]);
        }
        None => {
            if args.dur.is_some() {
                return Err(Error::input("--dur needs --at"));
            }
            let fc = format!("[0:v]{mir}[mir]copy[v]");
            argv.extend(["-filter_complex", &fc, "-map", "[v]", "-map", "0:a?"]);
        }
    }
    argv.extend([
        "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
    ]);
    if probe.has_audio {
        argv.extend(["-c:a", "copy"]);
    }
    argv.push(&args.output);

    let c = engine::write_job("mirror", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "axis": format!("{:?}", args.axis).to_lowercase(),
        "filter": "crop+flip+stack",
    })))
}

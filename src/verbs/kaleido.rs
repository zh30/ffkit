use serde_json::json;

use crate::cli::{Globals, KaleidoArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: KaleidoArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "kaleido")?;

    // top-left quadrant mirrored into a 2x2 mandala
    let chain = "crop=iw/2:ih/2:0:0,split[qa][qb];[qb]hflip[qf];[qa][qf]hstack[row];[row]split[ra][rb];[rb]vflip[rf];[ra][rf]vstack";

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    let fc = match &args.at {
        Some(a) => {
            let en = crate::time::enable_expr(a, args.dur, probe.duration)?;
            format!(
                "[0:v]split[m][f];[f]{chain}[kal];[m][kal]blend=all_expr='if({en},B,A)'[v]",
                en = en.replace("(t,", "(T,")
            )
        }
        None => {
            if args.dur.is_some() {
                return Err(Error::input("--dur needs --at"));
            }
            format!("[0:v]{chain}[kal];[kal]copy[v]")
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

    let c2 = engine::write_job("kaleido", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c2.with_extra(json!({})))
}

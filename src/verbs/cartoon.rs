use serde_json::json;

use crate::cli::{CartoonArgs, Globals};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: CartoonArgs, g: &Globals) -> Result<Contract, Error> {
    if !(2..=16).contains(&args.levels) {
        return Err(Error::input("--levels must be 2..=16"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "cartoon")?;

    // posterize the base, draw dark ink outlines from edge-detect on top
    let chain = format!(
        "split[a][b];[a]elbg=l={l}[base];[b]edgedetect=mode=wires:low=0.08:high=0.25,negate[lines];[base][lines]blend=all_mode=multiply",
        l = args.levels
    );

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    let fc = match &args.at {
        Some(a) => {
            let en = crate::time::enable_expr(a, args.dur, probe.duration)?;
            format!(
                "[0:v]split[m][f];[f]{chain}[cart];[m][cart]blend=all_expr='if({en},B,A)'[v]",
                en = en.replace("(t,", "(T,")
            )
        }
        None => {
            if args.dur.is_some() {
                return Err(Error::input("--dur needs --at"));
            }
            format!("[0:v]{chain}[cart];[cart]copy[v]")
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

    let c2 = engine::write_job("cartoon", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c2.with_extra(json!({ "levels": args.levels })))
}

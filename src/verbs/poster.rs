use serde_json::json;

use crate::cli::{Globals, PosterArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: PosterArgs, g: &Globals) -> Result<Contract, Error> {
    if !(2..=64).contains(&args.levels) {
        return Err(Error::input("--levels must be 2..=64"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "poster")?;

    // elbg quantizes to N palette colors = pop-art posterization
    let chain = format!("elbg=l={}", args.levels);
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    match &args.at {
        Some(s) => {
            // elbg has no timeline flag — blend swaps the posterized branch in
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

    let c = engine::write_job("poster", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "levels": args.levels,
        "filter": "elbg",
    })))
}

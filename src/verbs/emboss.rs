use serde_json::json;

use crate::cli::{EmbossArgs, Globals};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::time::enable_expr;

pub fn run(args: EmbossArgs, g: &Globals) -> Result<Contract, Error> {
    if !(0.0..=1.0).contains(&args.amount) {
        return Err(Error::input("--amount must be 0..=1 (mix with source)"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "emboss")?;
    let en = match &args.at {
        Some(a) => format!(":enable='{}'", enable_expr(a, args.dur, probe.duration)?),
        None => {
            if args.dur.is_some() {
                return Err(Error::input("--dur needs --at"));
            }
            String::new()
        }
    };

    let chain = format!("convolution='-2 -1 0 -1 1 1 0 1 2'{en}");

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    let fc = if args.amount < 1.0 {
        format!(
            "[0:v]split[a][b];[b]{chain}[e];[a][e]blend=all_mode=normal:all_opacity={:.3}[v]",
            args.amount
        )
    } else {
        format!("[0:v]{chain}[v]")
    };
    argv.extend(["-filter_complex", &fc, "-map", "[v]", "-map", "0:a?"]);
    argv.extend([
        "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
    ]);
    if probe.has_audio {
        argv.extend(["-c:a", "copy"]);
    }
    argv.push(&args.output);

    let c2 = engine::write_job("emboss", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c2.with_extra(json!({ "amount": args.amount })))
}

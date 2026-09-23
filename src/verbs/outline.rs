use serde_json::json;

use crate::cli::{Globals, OutlineArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::time::enable_expr;

pub fn run(args: OutlineArgs, g: &Globals) -> Result<Contract, Error> {
    if !(0.0..=1.0).contains(&args.strength) {
        return Err(Error::input("--strength must be 0..=1 edge threshold"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "outline")?;
    let en = match &args.at {
        Some(a) => format!(":enable='{}'", enable_expr(a, args.dur, probe.duration)?),
        None => {
            if args.dur.is_some() {
                return Err(Error::input("--dur needs --at"));
            }
            String::new()
        }
    };

    // ink the detected edges black over the footage
    let lo = 0.05 + args.strength * 0.25;
    let hi = (lo * 3.0).min(0.9);
    let fc = format!(
        "[0:v]split[a][b];[b]edgedetect=mode=wires:low={lo:.3}:high={hi:.3},negate[ln];\
         [a][ln]blend=all_mode=multiply{en}[v]"
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

    let c2 = engine::write_job("outline", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c2.with_extra(json!({ "strength": args.strength })))
}

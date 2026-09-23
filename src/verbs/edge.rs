use serde_json::json;

use crate::cli::{EdgeArgs, EdgeMode, Globals};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::time::enable_expr;

pub fn run(args: EdgeArgs, g: &Globals) -> Result<Contract, Error> {
    let mode = match args.mode {
        EdgeMode::Wires => "wires",
        EdgeMode::Colormix => "colormix",
    };
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "edge")?;
    let en = match &args.at {
        Some(a) => format!(":enable='{}'", enable_expr(a, args.dur, probe.duration)?),
        None => {
            if args.dur.is_some() {
                return Err(Error::input("--dur needs --at"));
            }
            String::new()
        }
    };

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    let vf = format!(
        "edgedetect=mode={mode}:low={:.3}:high={:.3}{en}",
        args.low, args.high
    );
    argv.extend(["-vf", &vf]);
    argv.extend([
        "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
    ]);
    if probe.has_audio {
        argv.extend(["-c:a", "copy"]);
    }
    argv.push(&args.output);

    let c2 = engine::write_job("edge", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c2.with_extra(json!({ "mode": mode })))
}

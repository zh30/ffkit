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
    let kernel = args.engine.and_then(|e| match e {
        crate::cli::EdgeEngine::Edgedetect => None,
        crate::cli::EdgeEngine::Link => Some("link"),
        crate::cli::EdgeEngine::Sobel => Some("sobel"),
        crate::cli::EdgeEngine::Kirsch => Some("kirsch"),
        crate::cli::EdgeEngine::Roberts => Some("roberts"),
        crate::cli::EdgeEngine::Prewitt => Some("prewitt"),
    });
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
    let vf = match kernel {
        // classic convolution kernels: bright edges on black, no thresholds —
        // cruder and crunchier than the Canny-style edgedetect path
        Some(k) if k != "link" => format!("{k}=planes=15{en}"),
        None => format!(
            "edgedetect=mode={mode}:low={:.3}:high={:.3}{en}",
            args.low, args.high
        ),
        _ => String::new(),
    };
    if kernel == Some("link") {
        // hysteresis grows dilated strong edges into the weak map: connected
        // contours survive, isolated specks die — labelled fc (vf can't split)
        let fc = format!(
            "[0:v]edgedetect=mode={mode}:low={:.3}:high={:.3},split[w][s];[w]gblur=sigma=2[d];[d][s]hysteresis[v]",
            args.low, args.high
        );
        argv.extend(["-filter_complex", &fc, "-map", "[v]", "-map", "0:a?"]);
    } else {
        argv.extend(["-vf", &vf]);
    }
    argv.extend([
        "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
    ]);
    if probe.has_audio {
        argv.extend(["-c:a", "copy"]);
    }
    argv.push(&args.output);

    let c2 = engine::write_job("edge", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c2.with_extra(json!({ "mode": mode, "engine": kernel.unwrap_or("edgedetect") })))
}

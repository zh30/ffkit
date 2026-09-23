use serde_json::json;

use crate::cli::{Globals, SmoothArgs, SmoothEngine};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

/// Edge-preserving blur — softens skin/skies while keeping outlines.
/// smartblur positive `ls` flattens flat regions only; the `lt` threshold
/// keeps real edges untouched (the classic "beauty" trick).
pub fn run(args: SmoothArgs, g: &Globals) -> Result<Contract, Error> {
    if !(0.1..=1.0).contains(&args.strength) {
        return Err(Error::input("--strength must be 0.1..1"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "smooth")?;

    let s = args.strength;
    let chain = match args.engine.unwrap_or(SmoothEngine::Smartblur) {
        SmoothEngine::Smartblur => format!(
            "smartblur=lr={:.2}:ls={:.2}:lt={:.0}",
            0.5 + 2.0 * s,
            0.3 + 0.65 * s,
            5.0 + 20.0 * s
        ),
        // bilateral keeps chroma+edges, drops luma noise — planes=1 = luma only
        // morphological close/open — texture smoothing with zero blur halo;
        // strength → pass count (1-3)
        SmoothEngine::Deflate => {
            vec!["deflate".to_string(); (args.strength * 6.0).ceil().clamp(1.0, 3.0) as usize]
                .join(",")
        }
        SmoothEngine::Inflate => {
            vec!["inflate".to_string(); (args.strength * 6.0).ceil().clamp(1.0, 3.0) as usize]
                .join(",")
        }
        SmoothEngine::Pp7 => {
            let qp = (1.0 + args.strength * 5.0).round() as u32;
            format!("pp7=qp={qp}:mode=medium")
        }
        SmoothEngine::Uspp => {
            // postproc quality 0..8; strength scales the deblock strength
            let q = (args.strength * 8.0).round() as u32;
            format!("uspp=quality={q}:qp=4")
        }
        SmoothEngine::Bilateral => format!(
            "bilateral=sigmaS={:.1}:sigmaR={:.2}:planes=1",
            4.0 + 12.0 * s,
            0.10 + 0.40 * s
        ),
    };

    let chain = match &args.at {
        Some(raw) => format!(
            "{chain}:enable='{}'",
            crate::time::enable_expr(raw, args.dur, probe.duration)?
        ),
        None => {
            if args.dur.is_some() {
                return Err(Error::input("--dur needs --at"));
            }
            chain
        }
    };

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-vf", &chain]);
    argv.extend([
        "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
    ]);
    if probe.has_audio {
        argv.extend(["-c:a", "copy"]);
    }
    argv.push(&args.output);

    let c = engine::write_job("smooth", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "strength": s,
        "filter": "smartblur",
    })))
}

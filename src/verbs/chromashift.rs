use serde_json::json;

use crate::cli::{ChromaEdge, ChromashiftArgs, Globals};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::time::enable_expr;

/// Nudge the chroma planes by whole pixels to fix chroma misregistration —
/// the colored halo on tape captures, telecine, or badly encoded downloads.
pub fn run(args: ChromashiftArgs, g: &Globals) -> Result<Contract, Error> {
    if !(-255..=255).contains(&args.x) || !(-255..=255).contains(&args.y) {
        return Err(Error::input("--x/--y must be -255..=255"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "chromashift")?;

    let edge = match args.edge {
        ChromaEdge::Wrap => "wrap",
        ChromaEdge::Smear => "smear",
    };
    let chain = format!(
        "chromashift=cbh={x}:cbv={y}:crh={x}:crv={y}:edge={edge}",
        x = args.x,
        y = args.y
    );
    let fc = match &args.at {
        Some(a) => {
            let en = enable_expr(a, args.dur, probe.duration)?;
            format!(
                "[0:v]split[m][f];[f]{chain}[p];[m][p]blend=all_expr='if({en},B,A)'[v]",
                en = en.replace("(t,", "(T,")
            )
        }
        None => {
            if args.dur.is_some() {
                return Err(Error::input("--dur needs --at"));
            }
            format!("[0:v]{chain}[p];[p]copy[v]")
        }
    };

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

    let c = engine::write_job("chromashift", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({ "x": args.x, "y": args.y, "edge": edge })))
}

use serde_json::json;

use crate::cli::{Globals, StackArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

/// Median-stack N videos of the same locked-off scene: each output pixel is
/// the median across inputs, so anything present in fewer than half the
/// frames vanishes — tourists crossing a museum shot, traffic on a street
/// timelapse, per-file sensor noise (3+ copies = ~1/sqrt(N) noise).
/// Audio is taken from the first input.
pub fn run(args: StackArgs, g: &Globals) -> Result<Contract, Error> {
    let min = if matches!(args.mode, crate::cli::StackMode::Median) {
        3
    } else {
        2
    };
    if args.inputs.len() < min {
        return Err(Error::input(format!("stack needs at least {min} inputs")));
    }
    let mut probes = Vec::with_capacity(args.inputs.len());
    for i in &args.inputs {
        probes.push(engine::probe_or_err(i, g)?);
    }
    let (w, h) = (probes[0].width.unwrap_or(0), probes[0].height.unwrap_or(0));
    if w == 0 || h == 0 {
        return Err(Error::input("stack: first input has no video stream"));
    }
    for (i, p) in probes.iter().enumerate().skip(1) {
        if p.width != Some(w) || p.height != Some(h) {
            return Err(Error::input(format!(
                "stack: input {} is {}x{} but input 1 is {}x{} — conform first",
                i + 1,
                p.width.unwrap_or(0),
                p.height.unwrap_or(0),
                w,
                h
            )));
        }
    }
    if !(0.0..=1.0).contains(&args.percentile) {
        return Err(Error::input("--percentile must be 0..=1"));
    }

    // --weights is a mean-blend control; it changes nothing on the
    // percentile modes.
    let weights: Option<Vec<f64>> = match &args.weights {
        Some(raw) => {
            if !matches!(args.mode, crate::cli::StackMode::Mean) {
                return Err(Error::input("--weights only applies to --mode mean"));
            }
            let ws: Vec<f64> = raw
                .split(',')
                .map(|s| s.trim().parse::<f64>())
                .collect::<Result<_, _>>()
                .map_err(|_| Error::input("--weights wants comma numbers"))?;
            if ws.len() != args.inputs.len() || ws.iter().any(|w| *w <= 0.0) {
                return Err(Error::input("--weights: one positive number per input"));
            }
            Some(ws)
        }
        None => None,
    };

    let n = args.inputs.len();
    let pads: String = (0..n).map(|i| format!("[{i}:v]")).collect();
    let fc = match args.mode {
        crate::cli::StackMode::Median => {
            format!(
                "{pads}xmedian=inputs={n}:percentile={:.2}[v]",
                args.percentile.clamp(0.0, 1.0)
            )
        }
        crate::cli::StackMode::Max => format!("{pads}maskedmax[v]"),
        crate::cli::StackMode::Min => format!("{pads}maskedmin[v]"),
        // mix normalizes by the weight sum (scale=0), so weights read as
        // exposure shares: "3,1" = 75%/25% double exposure
        crate::cli::StackMode::Mean => {
            let ws = weights
                .map(|v| {
                    v.iter()
                        .map(|w| format!("{w:.4}"))
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .unwrap_or_else(|| "1 ".repeat(n).trim_end().to_string());
            format!("{pads}mix=inputs={n}:weights='{ws}':duration=first[v]")
        }
    };

    let mut argv = ffmpeg_base(g.progress);
    for i in &args.inputs {
        argv.push("-i");
        argv.push(i);
    }
    argv.extend(["-filter_complex", &fc, "-map", "[v]", "-map", "0:a?"]);
    argv.extend([
        "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
    ]);
    if probes[0].has_audio {
        argv.extend(["-c:a", "copy"]);
    }
    argv.push(&args.output);

    let inputs: Vec<&std::path::Path> = args.inputs.iter().map(|p| p.as_path()).collect();
    let c = engine::write_job("stack", &inputs, &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "inputs": n,
        "percentile": args.percentile,
        "mode": format!("{:?}", args.mode).to_lowercase(),
        "weights": args.weights,
        "filter": match args.mode { crate::cli::StackMode::Median => "xmedian", crate::cli::StackMode::Max => "maskedmax", crate::cli::StackMode::Min => "maskedmin", crate::cli::StackMode::Mean => "mix" }
    })))
}

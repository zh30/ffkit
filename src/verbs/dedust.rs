use crate::cli::{DedustArgs, DedustEngine};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use serde_json::json;

/// Remove dust specks / hot pixels with morphology — unlike blur, only the
/// specks change. Bright specks erode, dark specks dilate, on the luma plane.
/// Default targets bright specks (white dust on scans); --dark for dark ones.
pub fn run(args: DedustArgs, g: &crate::cli::Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "dedust")?;
    let size = args.size.clamp(1, 4);
    let light = args.light || !args.dark;

    // erosion kills bright peaks (luma plane); dilation kills dark pockets.
    // threshold1/2/3=0 leaves chroma untouched — only luma specks are removed.
    // --engine temporal instead min/maxes each pixel against the previous
    // frame (tlut2): one-frame sparkles & VHS dropouts vanish entirely.
    let mut parts: Vec<String> = Vec::new();
    match args.engine {
        Some(DedustEngine::Temporal) => {
            let op = if args.dark { "max" } else { "min" };
            parts.push(format!(
                "tlut2=c0='{op}(x,y)':c1='{op}(x,y)':c2='{op}(x,y)'"
            ));
        }
        _ => {
            for _ in 0..size {
                if light {
                    parts.push("erosion=threshold1=0:threshold2=0:threshold3=0".to_string());
                }
                if args.dark {
                    parts.push("dilation=threshold1=0:threshold2=0:threshold3=0".to_string());
                }
            }
        }
    }
    let mut vf = parts.join(",");
    if let Some(s) = &args.at {
        let win = crate::time::enable_expr(s, args.dur, probe.duration)?;
        vf = vf
            .split(',')
            .map(|seg| format!("{seg}:enable='{win}'"))
            .collect::<Vec<_>>()
            .join(",");
    } else if args.dur.is_some() {
        return Err(Error::input("--dur needs --at"));
    }

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend([
        "-vf", &vf, "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
    ]);
    if probe.has_audio {
        argv.extend(["-c:a", "copy"]);
    }
    argv.push(&args.output);
    let c = engine::write_job("dedust", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "size": size,
        "light": light,
        "dark": args.dark,
        "engine": format!("{:?}", args.engine.unwrap_or(DedustEngine::Morpho)).to_lowercase(),
        "filter": vf,
    })))
}

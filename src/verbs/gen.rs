use serde_json::json;

use crate::cli::{GenArgs, Globals};
use crate::color;
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

/// Generative animated backgrounds straight from lavfi sources — no input
/// file. mandelbrot zooms in forever, gradients drifts through colors,
/// life runs a cellular automaton. For music visualizers, VJ loops, text-card
/// backdrops.
pub fn run(args: GenArgs, g: &Globals) -> Result<Contract, Error> {
    let mut it = args.size.split('x');
    let w: u32 = it.next().and_then(|v| v.parse().ok()).unwrap_or(0);
    let h: u32 = it.next().and_then(|v| v.parse().ok()).unwrap_or(0);
    if w < 32 || h < 32 || w > 8192 || h > 8192 {
        return Err(Error::input("--size WxH between 32x32 and 8192x8192"));
    }
    if !(1.0..=120.0).contains(&args.fps) || !(0.1..=3600.0).contains(&args.dur) {
        return Err(Error::input("--fps 1..120, --dur 0.1..3600"));
    }
    if args.rule != 0 && !(0..=255).contains(&args.rule) {
        return Err(Error::input("--rule 0..255"));
    }
    let src = match args.pattern.as_str() {
        "mandelbrot" => format!(
            "mandelbrot=size={}x{}:rate={}:start_scale=3:end_scale={}",
            w, h, args.fps, args.zoom
        ),
        "gradients" => {
            let mut s = format!(
                "gradients=size={}x{}:rate={}:speed={}",
                w, h, args.fps, args.speed
            );
            for (i, c) in args
                .colors
                .as_deref()
                .unwrap_or("")
                .split(',')
                .filter(|v| !v.trim().is_empty())
                .take(8)
                .enumerate()
            {
                color::rgb(c.trim())
                    .map_err(|_| Error::input(format!("--colors entry '{c}' is not a color")))?;
                let col = color::lavfi(c.trim());
                s.push_str(&format!(":c{}={col}", i));
            }
            if let Some(seed) = args.seed {
                s.push_str(&format!(":seed={seed}"));
            }
            s
        }
        "life" => format!(
            "cellauto=size={}x{}:rate={}:rule={}",
            w, h, args.fps, args.rule
        ),
        "noise" | "tone" => String::new(),
        _ => {
            return Err(Error::input(
                "--pattern: mandelbrot | gradients | life | noise | tone",
            ))
        }
    };
    if matches!(args.pattern.as_str(), "noise" | "tone") {
        // audio-only beds: pink noise (roomtone/dither bed) or a sine tone
        let audio_src = if args.pattern == "tone" {
            let f = args.freq.unwrap_or(440.0).clamp(20.0, 20000.0);
            format!("sine=frequency={f}:sample_rate=44100")
        } else {
            "anoisesrc=color=pink:sample_rate=44100".to_string()
        };
        let mut argv = ffmpeg_base(g.progress);
        argv.extend([
            "-f",
            "lavfi",
            "-i",
            &audio_src,
            "-t",
            &format!("{:.3}", args.dur),
        ]);
        argv.extend(["-c:a", "aac"]);
        argv.push(&args.output);
        let c = engine::write_job("gen", &[], &args.output, vec![argv], g)?;
        return Ok(c.with_extra(json!({
            "pattern": args.pattern,
            "freq": args.freq,
            "duration_s": args.dur,
        })));
    }
    let mut argv = ffmpeg_base(g.progress);
    argv.extend(["-f", "lavfi", "-i"]);
    argv.push(&src);
    argv.extend([
        "-t",
        &format!("{:.3}", args.dur),
        "-c:v",
        "libx264",
        "-preset",
        "fast",
        "-crf",
        "18",
        "-pix_fmt",
        "yuv420p",
    ]);
    argv.push(&args.output);
    let c = engine::write_job("gen", &[], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "pattern": args.pattern,
        "size": format!("{w}x{h}"),
        "dur": args.dur,
        "source": src,
    })))
}

use serde_json::json;

use crate::cli::{Globals, SolidArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::time::fmt_time;

/// Generate a solid-color clip (intro cards, lyric/backplate backgrounds,
/// b-roll spacers). No input file needed.
pub fn run(args: SolidArgs, g: &Globals) -> Result<Contract, Error> {
    if args.dur <= 0.0 {
        return Err(Error::input("--dur must be > 0"));
    }
    let (w, h) = args
        .size
        .split_once('x')
        .and_then(|(a, b)| Some((a.parse::<u32>().ok()?, b.parse::<u32>().ok()?)))
        .ok_or_else(|| Error::input(format!("--size must be WxH, got {}", args.size)))?;
    if w == 0 || h == 0 {
        return Err(Error::input("--size must be positive"));
    }
    let lavfi = match &args.gradient {
        Some(grad) => {
            let (c0, c1) = grad
                .split_once(':')
                .or_else(|| grad.split_once(','))
                .ok_or_else(|| Error::input("--gradient must look like RRGGBB:RRGGBB"))?;
            let c0 = crate::color::lavfi(c0);
            let c1 = crate::color::lavfi(c1);
            format!(
                "gradients=c0={c0}:c1={c1}:s={w}x{h}:d={}:speed=0.02:rate=30",
                fmt_time(args.dur)
            )
        }
        None => {
            let color = crate::color::lavfi(&args.color);
            format!("color=c={color}:s={w}x{h}:d={}:rate=30", fmt_time(args.dur))
        }
    };

    let mut argv = ffmpeg_base(g.progress);
    argv.extend(["-f", "lavfi", "-i", &lavfi]);

    let tmp = if args.text.is_some() {
        Some(tempfile::tempdir().map_err(|e| Error::output(e.to_string()))?)
    } else {
        None
    };
    if let Some(text) = &args.text {
        let font_path = crate::font::resolve(args.font.as_deref().map(std::path::Path::new))?;
        let font_bytes = std::fs::read(&font_path)?;
        let fg = crate::color::rgb(args.text_color.as_deref().unwrap_or("ffffff"))?;
        let img = crate::raster::render_title_styled(text, &font_bytes, w, fg, 1.0)?;
        let png = tmp.as_ref().unwrap().path().join("t.png");
        img.save(&png)
            .map_err(|e| Error::output(format!("write solid text png: {e}")))?;
        argv.extend(["-i", &png.display().to_string()]);
    }

    if args.audio {
        argv.extend(["-f", "lavfi", "-i", "anullsrc=r=48000:cl=stereo"]);
    }
    match (args.text.is_some(), args.audio) {
        (true, true) => argv.extend([
            "-filter_complex",
            "[0:v][1:v]overlay=(W-w)/2:(H-h)/2[v]",
            "-map",
            "[v]",
            "-map",
            "2:a",
            "-shortest",
        ]),
        (true, false) => argv.extend([
            "-filter_complex",
            "[0:v][1:v]overlay=(W-w)/2:(H-h)/2[v]",
            "-map",
            "[v]",
        ]),
        (false, true) => argv.extend(["-map", "0:v", "-map", "1:a", "-shortest"]),
        (false, false) => {}
    }
    if args.audio {
        argv.extend(["-c:a", "aac"]);
    }
    argv.extend([
        "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
    ]);
    argv.push(&args.output);

    let c = engine::write_job("solid", &[], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "color": format!("#{}", args.color.trim_start_matches("0x").trim_start_matches('#')),
        "gradient": args.gradient,
        "size": format!("{w}x{h}"),
        "dur": args.dur,
        "audio": args.audio,
    })))
}

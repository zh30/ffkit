use serde_json::json;
use std::path::Path;

use crate::cli::{Globals, MemeArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

fn parse_hex(c: &str) -> Result<[u8; 3], Error> {
    let c = c.trim_start_matches('#');
    if c.len() != 6 {
        return Err(Error::input("--color must be RRGGBB hex"));
    }
    let b = |i: usize| -> Result<u8, Error> {
        u8::from_str_radix(&c[i..i + 2], 16).map_err(|_| Error::input("--color must be RRGGBB hex"))
    };
    Ok([b(0)?, b(2)?, b(4)?])
}

pub fn run(args: MemeArgs, g: &Globals) -> Result<Contract, Error> {
    if args.top.is_none() && args.bottom.is_none() {
        return Err(Error::input("pass --top and/or --bottom text"));
    }
    if !(0.25..=8.0).contains(&args.size) {
        return Err(Error::input("--size must be 0.25..8"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "meme")?;

    let font_path = crate::font::resolve(args.font.as_deref().map(Path::new))?;
    let font_bytes = std::fs::read(&font_path)?;
    let vw = probe.width.unwrap_or(1280);
    let fg = match &args.color {
        Some(c) => parse_hex(c)?,
        None => [255, 255, 255],
    };

    let tmp = tempfile::tempdir().map_err(|e| Error::output(e.to_string()))?;
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);

    // One PNG input per text run; overlaid top-center / bottom-center.
    let texts = [
        (args.top.as_deref(), "(H-h)*0.04"),
        (args.bottom.as_deref(), "(H-h)*0.96"),
    ];
    let mut n_png = 0usize;
    let mut segs = Vec::new();
    let mut prev = "[0:v]".to_string();
    for (text, y) in texts {
        let Some(text) = text else { continue };
        let img = crate::raster::render_title_styled(text, &font_bytes, vw, fg, args.size as f32)?;
        let png = tmp.path().join(format!("t{n_png}.png"));
        img.save(&png)
            .map_err(|e| Error::output(format!("write meme text png: {e}")))?;
        argv.push("-i");
        argv.push(png);
        n_png += 1;
        let label = format!("m{n_png}");
        segs.push(format!("{prev}[{n_png}:v]overlay=x=(W-w)/2:y={y}[{label}]"));
        prev = format!("[{label}]");
    }
    let fc = segs.join(";");
    let last_label = format!("m{n_png}");
    argv.extend(["-filter_complex", &fc, "-map", &format!("[{last_label}]")]);
    if probe.has_audio {
        argv.extend(["-map", "0:a?", "-c:a", "copy"]);
    }
    argv.extend([
        "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
    ]);
    argv.push(&args.output);

    let c = engine::write_job("meme", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "top": args.top,
        "bottom": args.bottom,
        "font": font_path.display().to_string(),
    })))
}

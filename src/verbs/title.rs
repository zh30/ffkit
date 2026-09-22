use std::path::Path;

use serde_json::json;

use crate::cli::{Globals, TitleArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

/// Greedy word-wrap: break lines at ~`n` chars on spaces.
pub(crate) fn wrap(text: &str, n: usize) -> String {
    let mut lines: Vec<String> = Vec::new();
    for raw_line in text.lines() {
        let mut cur = String::new();
        for w in raw_line.split_whitespace() {
            if !cur.is_empty() && cur.len() + 1 + w.len() > n {
                lines.push(std::mem::take(&mut cur));
            }
            if !cur.is_empty() {
                cur.push(' ');
            }
            cur.push_str(w);
        }
        lines.push(std::mem::take(&mut cur));
    }
    lines.join("\n")
}

pub fn run(args: TitleArgs, g: &Globals) -> Result<Contract, Error> {
    let raw = args.text.trim();
    if raw.is_empty() {
        return Err(Error::input("--text is empty"));
    }
    let wrapped;
    let text = match args.wrap {
        Some(n) if n >= 4 => {
            wrapped = crate::verbs::title::wrap(raw, n as usize);
            wrapped.as_str()
        }
        Some(_) => return Err(Error::input("--wrap needs at least 4 chars")),
        None => raw,
    };
    if args.duration <= 0.0 {
        return Err(Error::input("--duration must be > 0"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "title")?;
    let at = match &args.at {
        Some(s) => crate::time::parse_time(s)?,
        None => 0.0,
    };
    if at < 0.0 || at >= probe.duration {
        return Err(Error::input("--at must land inside the input"));
    }
    let until = (at + args.duration).min(probe.duration);
    let font_path = crate::font::resolve(args.font.as_deref().map(Path::new))?;
    let font_bytes = std::fs::read(&font_path)?;
    let vw = probe.width.unwrap_or(1280);
    let fg = match &args.color {
        Some(c) => crate::color::rgb(c)?,
        None => [255, 255, 255],
    };
    if !(0.25..=8.0).contains(&args.size) {
        return Err(Error::input("--size must be 0.25..8"));
    }
    let img = match &args.outline {
        Some(c) => {
            let oc = crate::color::rgb(c)?;
            // stroke ~6% of glyph height so it scales with --size
            let ow = ((vw as f32 / 8.0 * args.size as f32) * 0.06)
                .round()
                .clamp(2.0, 24.0) as u32;
            crate::raster::render_title_outlined(
                text,
                &font_bytes,
                vw,
                fg,
                args.size as f32,
                (oc, ow),
            )?
        }
        None => match args.shadow {
            Some(b) => crate::raster::render_title_shadow(
                text,
                &font_bytes,
                vw,
                fg,
                args.size as f32,
                b.min(48),
            )?,
            None => {
                let mut img = crate::raster::render_title_styled(
                    text,
                    &font_bytes,
                    vw,
                    fg,
                    args.size as f32,
                )?;
                if let Some(b) = &args.box_color {
                    let [r, g_, b_] = crate::color::rgb(b)?;
                    let pad = (img.height() / 2).max(8);
                    let mut card = image::RgbaImage::new(img.width() + 2 * pad, img.height() + pad);
                    for px in card.pixels_mut() {
                        *px = image::Rgba([r, g_, b_, 200]);
                    }
                    image::imageops::overlay(&mut card, &img, pad as i64, (pad / 2) as i64);
                    img = card;
                }
                img
            }
        },
    };
    let tmp = tempfile::tempdir().map_err(|e| Error::output(e.to_string()))?;
    let png = tmp.path().join("title.png");
    img.save(&png)
        .map_err(|e| Error::output(format!("write title png: {e}")))?;

    let (x, y) = if args.tile > 0 {
        ("", "")
    } else {
        match args.position.as_str() {
            "center" => ("(W-w)/2", "(H-h)/2"),
            "top" => ("(W-w)/2", "trunc(H*0.18)"),
            "bottom" => ("(W-w)/2", "trunc(H*0.78)"),
            "top-left" => ("trunc(W*0.06)", "trunc(H*0.10)"),
            "top-right" => ("W-w-trunc(W*0.06)", "trunc(H*0.10)"),
            "bottom-left" => ("trunc(W*0.06)", "H-h-trunc(H*0.10)"),
            "bottom-right" => ("W-w-trunc(W*0.06)", "H-h-trunc(H*0.10)"),
            other => {
                return Err(Error::input(format!(
                    "--position {other}: use center, top, bottom or a corner"
                )));
            }
        }
    };
    let fade = args.fade.clamp(0.0, ((until - at) / 2.0).max(0.0));
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    if fade > 0.0 {
        // Looping the PNG gives the still advancing pts so alpha fades animate.
        argv.extend(["-loop", "1", "-framerate", "30"]);
    }
    argv.push("-i");
    argv.push(&png);
    let (pre, ovl) = if fade > 0.0 {
        (
            format!(
                "[1:v]format=rgba,fade=t=in:st={at:.3}:d={fade:.3}:alpha=1,fade=t=out:st={:.3}:d={fade:.3}:alpha=1[ovl];",
                until - fade
            ),
            "ovl",
        )
    } else {
        (String::new(), "1:v")
    };
    let shortest = if fade > 0.0 { ":shortest=1" } else { "" };
    let fc = if args.tile > 0 {
        // N copies on a diagonal cascade — text draft watermark
        let n = args.tile.clamp(2, 6);
        let mut seg: Vec<String> = Vec::new();
        let mut prev = "[0:v]".to_string();
        for i in 0..n {
            let fx = i as f64 / n as f64;
            let fy = (i as f64 + 0.5) / n as f64;
            let lab = if i + 1 == n {
                "vout".to_string()
            } else {
                format!("t{i}")
            };
            seg.push(format!(
                "{prev}[{ovl}]overlay=x={fx:.3}*(W-w):y={fy:.3}*(H-h):enable='between(t,{at:.3},{until:.3})'{shortest}[{lab}]"
            ));
            prev = format!("[{lab}]");
        }
        seg.join(";")
    } else {
        format!("[0:v][{ovl}]overlay=x={x}:y={y}:enable='between(t,{at:.3},{until:.3})'{shortest}[vout]")
    };
    let fc = format!("{pre}{fc}");
    argv.extend(["-filter_complex", &fc, "-map", "[vout]"]);
    if probe.has_audio {
        argv.extend(["-map", "0:a", "-c:a", "copy"]);
    }
    argv.extend([
        "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
    ]);
    argv.push(&args.output);

    let c = engine::write_job("title", &[&args.input], &args.output, vec![argv], g)?;
    drop(tmp);
    Ok(c.with_extra(json!({
        "text": text,
        "duration": until,
        "position": if args.tile > 0 { format!("tile-{}", args.tile) } else { args.position },
        "at": at,
        "font": font_path.display().to_string(),
    })))
}

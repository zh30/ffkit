use serde_json::json;
use std::path::Path;

use crate::cli::{Globals, MemeArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

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
        Some(c) => crate::color::rgb(c)?,
        None => [255, 255, 255],
    };

    let enable = match (&args.at, args.dur) {
        (Some(at), dur) => {
            if at.contains(',') && dur.is_none() {
                return Err(Error::input("a comma list of --at times needs --dur"));
            }
            let mut starts = Vec::new();
            for part in at.split(',') {
                starts.push(crate::time::resolve_at(part.trim(), dur, probe.duration)?);
            }
            if starts.len() == 1 {
                let start = starts[0];
                match dur {
                    Some(d) => format!(":enable='between(t,{start:.3},{:.3})'", start + d),
                    None => format!(":enable='gte(t,{start:.3})'"),
                }
            } else {
                let d = dur.unwrap();
                let expr = starts
                    .iter()
                    .map(|s| format!("between(t,{s:.3},{:.3})", s + d))
                    .collect::<Vec<_>>()
                    .join("+");
                format!(":enable='{expr}'")
            }
        }
        (None, Some(_)) => return Err(Error::input("--dur needs --at")),
        (None, None) => String::new(),
    };
    let tmp = tempfile::tempdir().map_err(|e| Error::output(e.to_string()))?;
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);

    // One PNG input per text run; overlaid top-center / bottom-center
    // (or stacked per --position).
    if let Some(n) = args.wrap {
        if n < 4 {
            return Err(Error::input("--wrap must be ≥ 4 columns"));
        }
    }
    let wrapped = |s: Option<&String>| -> Option<String> {
        s.map(|t| match args.wrap {
            Some(n) => crate::verbs::title::wrap(t, n as usize),
            None => t.clone(),
        })
    };
    let texts = [wrapped(args.top.as_ref()), wrapped(args.bottom.as_ref())];
    let mut renders: Vec<(u32, image::RgbaImage)> = Vec::new();
    for text in texts.iter().flatten() {
        let text = text.as_str();
        let img = if args.outline > 0 {
            crate::raster::render_title_outlined(
                text,
                &font_bytes,
                vw,
                fg,
                args.size as f32,
                ([0, 0, 0], args.outline),
            )?
        } else {
            match args.align {
                Some(al) => crate::raster::render_title_aligned(
                    text,
                    &font_bytes,
                    vw,
                    fg,
                    args.size as f32,
                    al,
                )?,
                None => {
                    crate::raster::render_title_styled(text, &font_bytes, vw, fg, args.size as f32)?
                }
            }
        };
        renders.push((img.height(), img));
    }
    // y per rendered PNG, in render order (top text first).
    let ys: Vec<String> = match args.position {
        None | Some(crate::cli::MemePos::Top) => {
            let mut v = vec![];
            if args.top.is_some() {
                v.push("(H-h)*0.04".to_string());
            }
            if args.bottom.is_some() {
                v.push("(H-h)*0.96".to_string());
            }
            v
        }
        Some(crate::cli::MemePos::Center) | Some(crate::cli::MemePos::Bottom) => {
            let vh = probe.height.unwrap_or(720) as f64;
            let hs: Vec<f64> = renders.iter().map(|(h, _)| *h as f64).collect();
            let gap = vh * 0.02;
            let block: f64 = hs.iter().sum::<f64>() + gap * (hs.len() as f64 - 1.0).max(0.0);
            let mut y = match args.position.unwrap() {
                crate::cli::MemePos::Center => (vh - block) / 2.0,
                _ => vh * 0.96 - block,
            };
            let mut v = Vec::new();
            for h in &hs {
                v.push(format!("{y:.0}"));
                y += h + gap;
            }
            v
        }
    };
    let mut n_png = 0usize;
    let mut segs = Vec::new();
    let mut prev = "[0:v]".to_string();
    for ((_, img), y) in renders.into_iter().zip(ys) {
        let png = tmp.path().join(format!("t{n_png}.png"));
        img.save(&png)
            .map_err(|e| Error::output(format!("write meme text png: {e}")))?;
        argv.push("-i");
        argv.push(png);
        n_png += 1;
        let label = format!("m{n_png}");
        segs.push(format!(
            "{prev}[{n_png}:v]overlay=x=(W-w)/2:y={y}{enable}[{label}]"
        ));
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

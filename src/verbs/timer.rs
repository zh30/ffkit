use std::path::Path;

use serde_json::json;

use crate::cli::{Globals, TimerArgs, TimerFormat};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

fn parse_hex(c: &str) -> Option<[u8; 3]> {
    let s = c.trim_start_matches('#');
    if s.len() != 6 {
        return None;
    }
    let v = u32::from_str_radix(s, 16).ok()?;
    Some([
        ((v >> 16) & 255) as u8,
        ((v >> 8) & 255) as u8,
        (v & 255) as u8,
    ])
}

pub fn run(args: TimerArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "timer")?;
    let at = args.at.unwrap_or(0.0);
    let until = at + args.dur.unwrap_or(f64::MAX).min(86400.0);
    let font_path = crate::font::resolve(args.font.as_deref().map(Path::new))?;
    let font_bytes =
        std::fs::read(&font_path).map_err(|e| Error::input(format!("read font: {e}")))?;
    let fg = match &args.color {
        Some(c) => parse_hex(c).ok_or_else(|| Error::input("--color must be RRGGBB hex"))?,
        None => [255, 255, 255],
    };
    let vw = probe.width.unwrap_or(1280);

    // 60-cell digit sprite ("00".."59", each centered in a fixed-width cell)
    // + a ":" image. Cells animate via crop x='mod(floor(t),60)*cell' on a
    // looped stream — no drawtext/libfreetype needed.
    let tmp = tempfile::tempdir().map_err(|e| Error::output(e.to_string()))?;
    let mut cells: Vec<image::RgbaImage> = Vec::new();
    let (mut cw, mut ch) = (0u32, 0u32);
    let ms = matches!(args.format, TimerFormat::Ms);
    let ncells: u32 = if ms { 100 } else { 60 };
    for i in 0..ncells {
        let img = crate::raster::render_title_styled(
            &format!("{i:02}"),
            &font_bytes,
            vw,
            fg,
            args.size as f32,
        )?;
        cw = cw.max(img.width());
        ch = ch.max(img.height());
        cells.push(img);
    }
    let mut sprite = image::RgbaImage::new(cw * ncells, ch);
    for (i, cell) in cells.iter().enumerate() {
        image::imageops::overlay(
            &mut sprite,
            cell,
            (i as u32 * cw + (cw - cell.width()) / 2) as i64,
            0,
        );
    }
    let sprite_path = tmp.path().join("digits.png");
    sprite
        .save(&sprite_path)
        .map_err(|e| Error::output(format!("write sprite: {e}")))?;
    let colon = crate::raster::render_title_styled(":", &font_bytes, vw, fg, args.size as f32)?;
    let colw = colon.width();
    let colon_path = tmp.path().join("colon.png");
    colon
        .save(&colon_path)
        .map_err(|e| Error::output(format!("write colon: {e}")))?;
    let dot = crate::raster::render_title_styled(".", &font_bytes, vw, fg, args.size as f32)?;
    let dotw = dot.width();
    let dot_path = tmp.path().join("dot.png");
    dot.save(&dot_path)
        .map_err(|e| Error::output(format!("write dot: {e}")))?;

    let hours = probe.duration > 3600.0 || at > 3600.0;
    // Layout: [hh:]mm:ss — each digit field is one sprite cell wide.
    let fields = if hours { 3 } else { 2 };
    let colons = fields - 1;
    let total_w = fields * cw + colons * colw;
    let m = args.margin;
    let (x0, y) = match args.position.as_str() {
        "top-left" => (format!("{m}"), format!("{m}")),
        "top" => (format!("(W-{total_w})/2"), format!("{m}")),
        "top-right" => (format!("W-{total_w}-{m}"), format!("{m}")),
        "center" => (format!("(W-{total_w})/2"), format!("(H-{ch})/2")),
        "bottom-left" => (format!("{m}"), format!("H-{ch}-{m}")),
        "bottom" => (format!("(W-{total_w})/2"), format!("H-{ch}-{m}")),
        "bottom-right" => (format!("W-{total_w}-{m}"), format!("H-{ch}-{m}")),
        other => {
            return Err(Error::input(format!(
                "unknown --position {other}; use top-left, top, top-right, center, bottom-left, bottom, bottom-right"
            )))
        }
    };
    // x offsets of each field inside the sprite grid, left→right:
    // [hh] [:] [mm] [:] [ss]
    let mut xparts: Vec<(String, u32)> = Vec::new(); // (kind, sprite-cell-multiplier)
    if hours {
        xparts.push(("hh".into(), cw));
    }
    xparts.push(("mm".into(), cw));
    xparts.push(("ss".into(), cw));
    if ms {
        xparts.push(("cs".into(), cw));
    }

    let fps = probe.fps.unwrap_or(30.0).max(1.0);
    let enable = format!("enable='between(t,{at:.3},{until:.3})'");

    let split_labels: String = (0..xparts.len()).map(|i| format!("[sp{i}]")).collect();
    let mut fc = format!(
        "[1:v]format=rgba[spr];[spr]split={}{split_labels}",
        xparts.len()
    );
    // crop each field out of the advancing sprite
    for (i, (kind, _)) in xparts.iter().enumerate() {
        let expr = match kind.as_str() {
            "hh" => format!("min(99,floor((t-{at:.3})/3600))"),
            "mm" => format!("mod(floor((t-{at:.3})/60),60)"),
            "cs" => format!("mod(floor((t-{at:.3})*100),100)"),
            _ => format!("mod(floor(t-{at:.3}),60)"),
        };
        fc.push_str(&format!(
            ";[sp{i}]crop=w={cw}:h={ch}:x='{expr}*{cw}':y=0[f{i}]"
        ));
    }
    // overlay chain: field, colon, field, colon, field
    let mut cur = "0:v".to_string();
    let mut xoff = 0u32; // pixel offset from x0
    let mut pass = 0usize;
    for (i, (kind, w)) in xparts.iter().enumerate() {
        if i > 0 {
            // ":" between fields, "." before centiseconds
            let (sep, sepw) = if kind == "cs" { (3, dotw) } else { (2, colw) };
            let out = format!("v{pass}");
            fc.push_str(&format!(
                ";[{cur}][{sep}:v]overlay=x={x0}+{xoff}:y={y}:shortest=1:{enable}[{out}]"
            ));
            cur = out;
            xoff += sepw;
            pass += 1;
        }
        let _ = kind;
        let out = format!("v{pass}");
        fc.push_str(&format!(
            ";[{cur}][f{i}]overlay=x={x0}+{xoff}:y={y}:shortest=1:{enable}[{out}]"
        ));
        cur = out;
        xoff += w;
        pass += 1;
    }

    let mut argv = ffmpeg_base(g.progress);
    argv.extend(["-i".to_string(), args.input.display().to_string()]);
    argv.extend([
        "-loop".to_string(),
        "1".to_string(),
        "-framerate".to_string(),
        format!("{fps:.3}"),
        "-i".to_string(),
        sprite_path.display().to_string(),
    ]);
    argv.extend([
        "-loop".to_string(),
        "1".to_string(),
        "-framerate".to_string(),
        format!("{fps:.3}"),
        "-i".to_string(),
        colon_path.display().to_string(),
    ]);
    argv.extend([
        "-loop".to_string(),
        "1".to_string(),
        "-framerate".to_string(),
        format!("{fps:.3}"),
        "-i".to_string(),
        dot_path.display().to_string(),
    ]);
    argv.extend(["-filter_complex".to_string(), fc]);
    argv.extend(["-map".to_string(), format!("[{cur}]")]);
    if probe.has_audio {
        argv.extend([
            "-map".to_string(),
            "0:a?".to_string(),
            "-c:a".to_string(),
            "copy".to_string(),
        ]);
    }
    argv.extend([
        "-c:v".to_string(),
        "libx264".to_string(),
        "-preset".to_string(),
        "fast".to_string(),
        "-crf".to_string(),
        "18".to_string(),
        "-pix_fmt".to_string(),
        "yuv420p".to_string(),
    ]);
    argv.push(args.output.display().to_string());

    let inputs: Vec<&Path> = vec![&args.input];
    let mut c = engine::write_job("timer", &inputs, &args.output, vec![argv], g)?;
    c = c.with_extra(json!({
        "position": args.position,
        "at": at,
        "until": if args.dur.is_some() { Some(until) } else { None },
        "hours": hours,
    }));
    Ok(c)
}

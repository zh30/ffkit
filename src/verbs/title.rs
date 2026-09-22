use std::path::Path;

use serde_json::json;

use crate::cli::{Globals, TitleArgs};
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

pub fn run(args: TitleArgs, g: &Globals) -> Result<Contract, Error> {
    let text = args.text.trim();
    if text.is_empty() {
        return Err(Error::input("--text is empty"));
    }
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
        Some(c) => parse_hex(c)?,
        None => [255, 255, 255],
    };
    if !(0.25..=8.0).contains(&args.size) {
        return Err(Error::input("--size must be 0.25..8"));
    }
    let img = crate::raster::render_title_styled(text, &font_bytes, vw, fg, args.size as f32)?;
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
            other => {
                return Err(Error::input(format!(
                    "--position {other}: use center, top or bottom"
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

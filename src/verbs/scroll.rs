//! `scroll` — roll a block of text up the frame (end credits, cast lists).
//! The text renders to a tall PNG once, then a time-driven `overlay` moves it
//! from below the frame to above it across the window.

use std::path::Path;

use serde_json::json;

use crate::cli::{Globals, ScrollArgs};
use crate::color::rgb as parse_rgb;
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::raster::render_title_styled;
use crate::time::resolve_at;

pub fn run(args: ScrollArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "scroll")?;

    let text = if let Some(t) = &args.text {
        t.clone()
    } else {
        std::fs::read_to_string(args.file.as_ref().unwrap()).map_err(|e| {
            Error::input(format!(
                "--file: {}: {e}",
                args.file.as_ref().unwrap().display()
            ))
        })?
    };
    if text.lines().all(|l| l.trim().is_empty()) {
        return Err(Error::input("credits text is empty"));
    }
    if !(0.25..=4.0).contains(&args.size) {
        return Err(Error::input("--size must be 0.25..4"));
    }

    if let Some(d) = args.dur {
        if d <= 0.5 {
            return Err(Error::input("--dur must be > 0.5 seconds"));
        }
    }
    if let Some(sp) = args.speed {
        if args.dur.is_some() {
            return Err(Error::input("--speed sets the pacing itself — drop --dur"));
        }
        if sp <= 1.0 {
            return Err(Error::input("--speed is px/s — must be > 1"));
        }
    }
    // --at takes a comma list: replay the roll at several marks (`end` ok).
    let mut windows: Vec<(f64, f64)> = Vec::new();
    match &args.at {
        Some(raw) => {
            for part in raw.split(',') {
                let t0 = resolve_at(part.trim(), args.dur, probe.duration)
                    .map_err(|e| Error::input(format!("--at: {e}")))?;
                let d = args.dur.unwrap_or(probe.duration - t0);
                windows.push((t0, d.min(probe.duration - t0)));
            }
        }
        None => windows.push((0.0, args.dur.unwrap_or(probe.duration))),
    }
    if windows.iter().any(|(_, d)| *d <= 0.5) {
        return Err(Error::input(
            "no room for the roll — --at + --dur must stay inside the video",
        ));
    }

    let font_path = crate::font::resolve(args.font.as_deref().map(Path::new))?;
    let font_bytes = std::fs::read(&font_path)?;
    let vw = probe.width.unwrap_or(1280);
    let fg = match &args.color {
        Some(c) => parse_rgb(c)?,
        None => [255, 255, 255],
    };
    use crate::cli::ScrollMode;
    let ticker = matches!(args.mode, ScrollMode::Ticker);
    // Ticker text must stay one long line — give the canvas 6× the frame.
    let canvas_w = if ticker { vw * 6 } else { vw };
    let owned;
    let text = match args.wrap {
        Some(n) if n >= 4 => {
            owned = crate::verbs::title::wrap(&text, n as usize);
            owned.as_str()
        }
        Some(_) => return Err(Error::input("--wrap must be ≥ 4 columns")),
        None => text.as_str(),
    };
    let img = match args.align {
        Some(al) => crate::raster::render_title_aligned(
            text,
            &font_bytes,
            canvas_w,
            fg,
            args.size as f32,
            al,
        )?,
        None => render_title_styled(text, &font_bytes, canvas_w, fg, args.size as f32)?,
    };
    let tmp = tempfile::tempdir().map_err(|e| Error::output(e.to_string()))?;
    let png = tmp.path().join("credits.png");
    img.save(&png)
        .map_err(|e| Error::output(format!("write credits png: {e}")))?;

    if args.bg.is_some() && !ticker {
        return Err(Error::input("--bg applies to --mode ticker"));
    }
    // Looped still so the overlay's t-driven expression animates.
    // Up: y slides H → -h. Ticker: x slides W → -w, pinned near the bottom.
    // One drawbox (ticker bg) + one overlay per --at window.
    let mut fc = String::new();
    let mut prev = "0:v".to_string();
    if let Some(b) = &args.bg {
        let travel = vw as f64 + img.width() as f64;
        for (i, (at, dur)) in windows.iter().enumerate() {
            let dur = args.speed.map(|sp| (travel / sp).min(*dur)).unwrap_or(*dur);
            fc.push_str(&format!(
                "[{prev}]drawbox=x=0:y=ih-{bar}:w=iw:h={bar}:color={c}@0.85:t=fill:enable='between(t,{at:.3},{end:.3})'[bg{i}];",
                bar = img.height() + 20,
                c = crate::color::lavfi(b),
                end = at + dur,
            ));
            prev = format!("bg{i}");
        }
    }
    for (i, (at, dur)) in windows.iter().enumerate() {
        let out = if i + 1 == windows.len() {
            "vout".to_string()
        } else {
            format!("v{i}")
        };
        let travel = if ticker {
            vw as f64 + img.width() as f64
        } else {
            probe.height.unwrap_or(720) as f64 + img.height() as f64
        };
        let dur = args.speed.map(|sp| (travel / sp).min(*dur)).unwrap_or(*dur);
        if ticker {
            fc.push_str(&format!(
                "[{prev}][1:v]overlay=x=W-(W+w)*((t-{at:.3})/{dur:.3}):y=H-h-40:enable='between(t,{at:.3},{end:.3})'[{out}];",
                end = at + dur,
            ));
        } else {
            fc.push_str(&format!(
                "[{prev}][1:v]overlay=x=(W-w)/2:y=H-(H+h)*((t-{at:.3})/{dur:.3}):enable='between(t,{at:.3},{end:.3})'[{out}];",
                end = at + dur,
            ));
        }
        prev = out;
    }
    let fc = fc.trim_end_matches(';').to_string();
    let max_end = windows.iter().map(|(a, d)| a + d).fold(0.0, f64::max);
    let mut argv = ffmpeg_base(g.progress);
    argv.extend(["-i".to_string(), args.input.display().to_string()]);
    // -t bounds the otherwise-infinite looped PNG so the graph drains.
    argv.extend([
        "-loop".to_string(),
        "1".to_string(),
        "-framerate".to_string(),
        "30".to_string(),
        "-t".to_string(),
        format!("{max_end:.3}"),
    ]);
    argv.push(std::ffi::OsString::from("-i"));
    argv.push(png.display().to_string());
    argv.extend([
        "-filter_complex".to_string(),
        fc,
        "-map".to_string(),
        "[vout]".to_string(),
    ]);
    if probe.has_audio {
        argv.extend([
            "-map".to_string(),
            "0:a".to_string(),
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
    let mut c = engine::write_job("scroll", &inputs, &args.output, vec![argv], g)?;
    c = c.with_extra(json!({
        "windows": windows.iter().map(|(a, d)| json!({ "at": a, "dur": d })).collect::<Vec<_>>(),
    }));
    Ok(c)
}

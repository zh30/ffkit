use serde_json::json;
use std::path::Path;

use crate::cli::{CountdownArgs, Globals};
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

/// Rasterized 3-2-1(-GO) intro overlay: one PNG input per run, each shown
/// `--each` seconds via `enable='between(t,a,b)'`.
pub fn run(args: CountdownArgs, g: &Globals) -> Result<Contract, Error> {
    // --target HH:MM[:SS]: count to a local wall-clock time — the "premiere
    // at 20:00" overlay. Seconds until that moment, rolling to tomorrow
    // when the time already passed today.
    let mut count = args.from;
    let mut target_label: Option<String> = None;
    if let Some(raw) = &args.target {
        if args.from != 3 {
            return Err(Error::input("--target sets the count — drop --from"));
        }
        let parts: Vec<&str> = raw.split(':').collect();
        let (h, m, s) = match parts.as_slice() {
            [h, m] => (*h, *m, "0"),
            [h, m, s] => (*h, *m, *s),
            _ => return Err(Error::input("--target must be HH:MM or HH:MM:SS")),
        };
        let secs = |v: &str| v.trim().parse::<f64>();
        let (h, m, s) = match (secs(h), secs(m), secs(s)) {
            (Ok(h), Ok(m), Ok(s)) if h < 24.0 && m < 60.0 && s < 60.0 => (h, m, s),
            _ => return Err(Error::input("--target must be HH:MM or HH:MM:SS")),
        };
        let delta = (h * 3600.0 + m * 60.0 + s - crate::verbs::timer::local_clock_secs(false))
            .rem_euclid(86400.0);
        count = delta.round() as u32;
        if !(1..=600).contains(&count) {
            return Err(Error::input(
                "--target lands more than 10 min ahead — use --from for long counts",
            ));
        }
        target_label = Some(raw.clone());
    }
    if !(1..=600).contains(&count) {
        return Err(Error::input("--from must be 1..=600"));
    }
    if args.each <= 0.0 {
        return Err(Error::input("--each must be > 0 seconds"));
    }
    if !(0.25..=8.0).contains(&args.size) {
        return Err(Error::input("--size must be 0.25..8"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "countdown")?;
    let at = match &args.at {
        Some(raw) => crate::time::resolve_frame_at(raw.trim(), probe.duration)?,
        None => 0.0,
    };
    if at < 0.0 {
        return Err(Error::input("--at must be >= 0"));
    }

    let font_path = crate::font::resolve(args.font.as_deref().map(Path::new))?;
    let font_bytes = std::fs::read(&font_path)?;
    let vw = probe.width.unwrap_or(1280);
    let fg = match &args.color {
        Some(c) => parse_hex(c)?,
        None => [255, 255, 255],
    };

    let fmt = |n: u32| -> String {
        match args.format.as_str() {
            "mm:ss" => format!("{}:{:02}", n / 60, n % 60),
            "h:mm:ss" => format!("{}:{:02}:{:02}", n / 3600, (n % 3600) / 60, n % 60),
            "s" => n.to_string(),
            other => other.to_string(),
        }
    };
    if !matches!(args.format.as_str(), "s" | "mm:ss" | "h:mm:ss") {
        return Err(Error::input("--format must be s, mm:ss or h:mm:ss"));
    }
    // Text runs: countdown digits, then optional GO.
    let mut runs: Vec<String> = (1..=count).rev().map(&fmt).collect();
    if let Some(go) = &args.go {
        runs.push(go.clone());
    }

    let tmp = tempfile::tempdir().map_err(|e| Error::output(e.to_string()))?;
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);

    // Optional label shown above the digits across the whole count window.
    let mut label_png = None;
    if let Some(label) = &args.text {
        let mut img = crate::raster::render_title_styled(label, &font_bytes, vw, fg, 1.0)?;
        if let Some(op) = args.opacity {
            crate::raster::alpha_scale(&mut img, op)?;
        }
        let png = tmp.path().join("label.png");
        img.save(&png)
            .map_err(|e| Error::output(format!("write countdown label png: {e}")))?;
        label_png = Some(png);
    }

    let (px, py) = match &args.position {
        Some(p) => crate::verbs::overlay::overlay_xy(p, 24)?,
        None => ("(W-w)/2".to_string(), "(H-h)/2".to_string()),
    };
    let mut segs = Vec::new();
    let mut prev = "[0:v]".to_string();
    let bg_rgb = match &args.bg {
        Some(c) => Some(crate::color::rgb(c)?),
        None => None,
    };
    for (i, text) in runs.iter().enumerate() {
        let img = crate::raster::render_title_styled(text, &font_bytes, vw, fg, args.size as f32)?;
        let png = tmp.path().join(format!("n{i}.png"));
        let img = match &bg_rgb {
            Some([r, g, b]) => {
                let pad = (img.height() / 3).max(8);
                let mut card = image::RgbaImage::new(img.width() + 2 * pad, img.height() + pad);
                for px in card.pixels_mut() {
                    *px = image::Rgba([*r, *g, *b, 200]);
                }
                image::imageops::overlay(&mut card, &img, pad as i64, (pad / 2) as i64);
                card
            }
            None => img,
        };
        let mut img = img;
        if let Some(op) = args.opacity {
            crate::raster::alpha_scale(&mut img, op)?;
        }
        img.save(&png)
            .map_err(|e| Error::output(format!("write countdown png: {e}")))?;
        argv.push("-i");
        argv.push(png);
        let a = at + i as f64 * args.each;
        let b = a + args.each;
        let label = format!("c{i}");
        segs.push(format!(
            "{prev}[{}:v]overlay=x={px}:y={py}:enable='between(t,{a:.3},{b:.3})'[{label}]",
            i + 1
        ));
        prev = format!("[{label}]");
    }
    if let Some(png) = &label_png {
        argv.push("-i");
        argv.push(png);
        let end = at + args.from as f64 * args.each;
        let lab_i = runs.len() + 1;
        let next = "l0".to_string();
        segs.push(format!(
            "{prev}[{lab_i}:v]overlay=(W-w)/2:(H*0.30):enable='between(t,{at:.3},{end:.3})'[{next}]"
        ));
        prev = format!("[{next}]");
    }
    let mut fc = segs.join(";");
    if args.beep {
        let win = runs.len() as f64 * args.each;
        // aevalsrc: 880Hz sine gated to the first 120ms of each tick window
        // aevalsrc: --tone Hz sine gated to the first 120ms of each tick window
        let hz = args.tone.unwrap_or(880.0).clamp(20.0, 20000.0);
        argv.extend([
            "-f",
            "lavfi",
            "-i",
            &format!(
                "aevalsrc='sin(2*PI*{hz}*t)*lt(mod(t-{at:.3},{ea:.3}),0.12)':d={dd:.3}:s=44100",
                hz = hz,
                ea = args.each,
                dd = at + win
            ),
        ]);
        let beep_idx = runs.len() + 1 + label_png.is_some() as usize;
        if probe.has_audio {
            fc.push_str(&format!(
                ";[0:a][{beep_idx}:a]amix=inputs=2:duration=first[aout]"
            ));
        } else {
            fc.push_str(&format!(";[{beep_idx}:a]anull[aout]"));
        }
    }
    argv.extend(["-filter_complex", &fc, "-map", &prev]);
    if args.beep {
        argv.extend(["-map", "[aout]", "-c:a", "aac"]);
    } else if probe.has_audio {
        argv.extend(["-map", "0:a?", "-c:a", "copy"]);
    }
    argv.extend([
        "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
    ]);
    argv.push(&args.output);

    let c = engine::write_job("countdown", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "from": count,
        "each": args.each,
        "at": at,
        "go": args.go,
        "target": target_label,
    })))
}

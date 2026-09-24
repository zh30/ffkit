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

// "HH:MM:SS:FF" / "HH:MM:SS;FF" (the `;` drop-frame convention — display
// intent only; frame numbering is straight-count) → start seconds
fn parse_tc(raw: &str, fps: f64) -> Result<f64, Error> {
    let norm = raw.replace(';', ":");
    let v: Vec<&str> = norm.split(':').collect();
    if v.len() != 4 {
        return Err(Error::input("--tc needs HH:MM:SS:FF (e.g. 01:00:00:00)"));
    }
    let h: f64 = v[0]
        .parse()
        .map_err(|_| Error::input("--tc needs HH:MM:SS:FF"))?;
    let m: f64 = v[1]
        .parse()
        .map_err(|_| Error::input("--tc needs HH:MM:SS:FF"))?;
    let s: f64 = v[2]
        .parse()
        .map_err(|_| Error::input("--tc needs HH:MM:SS:FF"))?;
    let f: f64 = v[3]
        .parse()
        .map_err(|_| Error::input("--tc needs HH:MM:SS:FF"))?;
    Ok(h * 3600.0 + m * 60.0 + s + f / fps)
}

// Seconds into the local day — reads the TZ offset out of `date +%z`
// (UTC when date is unavailable).
pub(crate) fn local_clock_secs() -> f64 {
    let utc = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let off = std::process::Command::new("date")
        .arg("+%z")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| {
            let s = s.trim();
            let (sign, d) = s.split_at(1);
            let h: i64 = d.get(..2)?.parse().ok()?;
            let m: i64 = d.get(2..4)?.parse().ok()?;
            Some(if sign == "-" {
                -(h * 3600 + m * 60)
            } else {
                h * 3600 + m * 60
            })
        })
        .unwrap_or(0);
    ((utc + off).rem_euclid(86400)) as f64
}

// Local wall date YYYY-MM-DD — `date` output when available, UTC civil
// (Hinnant) as the fallback.
fn local_date_str() -> String {
    let out = std::process::Command::new("date")
        .arg("+%Y-%m-%d")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| s.len() == 10);
    if let Some(s) = out {
        return s;
    }
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let z = secs.div_euclid(86400) + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}

pub fn run(args: TimerArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "timer")?;
    let at = match &args.at {
        Some(raw) => crate::time::resolve_frame_at(raw.trim(), probe.duration)?,
        None => 0.0,
    };
    let until = at + args.dur.unwrap_or(f64::MAX).min(86400.0);
    let fps = probe.fps.unwrap_or(30.0).max(1.0);
    if args.tc.is_some() && (args.down || args.start.is_some()) {
        return Err(Error::input("--tc can't combine with --down/--start"));
    }
    if args.clock && (args.down || args.start.is_some() || args.tc.is_some()) {
        return Err(Error::input(
            "--clock seeds from the system clock — drop --down/--start/--tc",
        ));
    }
    if args.date
        && (args.down
            || args.start.is_some()
            || args.tc.is_some()
            || matches!(args.format, TimerFormat::Ms))
    {
        return Err(Error::input(
            "--date is a calendar readout — drop --down/--start/--tc/--format ms",
        ));
    }
    // A date without --clock draws the calendar alone — no animated fields.
    let date_only = args.date && !args.clock;
    if args.tc.is_some() && matches!(args.format, TimerFormat::Ms) {
        return Err(Error::input("--tc shows frames instead of centiseconds"));
    }
    let tc_start = args
        .tc
        .as_deref()
        .map(|raw| parse_tc(raw, fps))
        .transpose()?;
    // --down: display the remaining time to the window end
    // --start / --tc seed the readout: up counts N+t-at, down counts N-(t-at).
    let tv = if args.clock {
        format!("{:.3}+(t-{at:.3})", local_clock_secs())
    } else if args.down {
        let start = args
            .start
            .unwrap_or_else(|| args.dur.unwrap_or(probe.duration - at));
        format!("max(0,{start:.3}-(t-{at:.3}))")
    } else {
        let start = tc_start.or(args.start).unwrap_or(0.0);
        format!("{start:.3}+(t-{at:.3})")
    };
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
    // tc needs a cell per frame-per-second (25/30/60…); capped for sanity
    let ncells: u32 = if ms {
        100
    } else if tc_start.is_some() {
        fps.round().clamp(2.0, 240.0) as u32
    } else {
        60
    };
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
    if let Some(op) = args.opacity {
        crate::raster::alpha_scale(&mut sprite, op)?;
    }
    let sprite_path = tmp.path().join("digits.png");
    sprite
        .save(&sprite_path)
        .map_err(|e| Error::output(format!("write sprite: {e}")))?;
    let mut colon = crate::raster::render_title_styled(":", &font_bytes, vw, fg, args.size as f32)?;
    let colw = colon.width();
    if let Some(op) = args.opacity {
        crate::raster::alpha_scale(&mut colon, op)?;
    }
    let colon_path = tmp.path().join("colon.png");
    colon
        .save(&colon_path)
        .map_err(|e| Error::output(format!("write colon: {e}")))?;
    let mut dot = crate::raster::render_title_styled(".", &font_bytes, vw, fg, args.size as f32)?;
    let dotw = dot.width();
    if let Some(op) = args.opacity {
        crate::raster::alpha_scale(&mut dot, op)?;
    }
    let dot_path = tmp.path().join("dot.png");
    dot.save(&dot_path)
        .map_err(|e| Error::output(format!("write dot: {e}")))?;
    // --date: the calendar readout is one static image (no animation cells)
    let date_png = if args.date {
        let mut img = crate::raster::render_title_styled(
            &local_date_str(),
            &font_bytes,
            vw,
            fg,
            args.size as f32,
        )?;
        if let Some(op) = args.opacity {
            crate::raster::alpha_scale(&mut img, op)?;
        }
        let w = img.width();
        let p = tmp.path().join("date.png");
        img.save(&p)
            .map_err(|e| Error::output(format!("write date: {e}")))?;
        Some((p, w))
    } else {
        None
    };

    let hours = probe.duration > 3600.0 || at > 3600.0 || tc_start.is_some() || args.clock;
    // Layout: [hh:]mm:ss[;ff] — each digit field is one sprite cell wide.
    let fields = if hours { 3 } else { 2 };
    let colons = fields - 1 + u32::from(tc_start.is_some());
    let mut total_w = (fields + u32::from(tc_start.is_some())) * cw + colons * colw;
    if ms {
        total_w += dotw + cw;
    }
    // The date prefix takes its rendered width + a half-cell gap.
    if let Some((_, dw)) = &date_png {
        total_w += *dw + cw / 2;
    }
    if date_only {
        total_w = date_png.as_ref().map(|(_, dw)| *dw).unwrap_or(0);
    }
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
    if tc_start.is_some() {
        xparts.push(("ff".into(), cw));
    }

    let enable = format!("enable='between(t,{at:.3},{until:.3})'");

    // --box: a fixed card behind the whole readout
    let box_png = if let Some(bc) = &args.box_color {
        let [r, g_, b_] = crate::color::rgb(bc)?;
        let pad = (ch / 4).max(6);
        let mut card = image::RgbaImage::new(total_w + 2 * pad, ch + pad);
        for px in card.pixels_mut() {
            *px = image::Rgba([r, g_, b_, 200]);
        }
        if let Some(op) = args.opacity {
            crate::raster::alpha_scale(&mut card, op)?;
        }
        let p = tmp.path().join("box.png");
        card.save(&p)
            .map_err(|e| Error::output(format!("write box png: {e}")))?;
        Some((p, pad))
    } else {
        None
    };
    // input order: sprite colon dot (animated modes only) then box then
    // the date image — indices follow the -i sequence pushed below
    let box_idx = if date_only { 1usize } else { 4 };
    let date_idx = box_idx + usize::from(box_png.is_some());

    let split_labels: String = (0..xparts.len()).map(|i| format!("[sp{i}]")).collect();
    let mut fc = String::new();
    if let Some((_, pad)) = &box_png {
        fc.push_str(&format!(
            "[0:v][{box_idx}:v]overlay=x={x0}-{pad}:y={y}-{hp}:shortest=1:{enable}[vbox]",
            hp = pad / 2
        ));
    }
    let mut cur = if box_png.is_some() {
        "vbox".to_string()
    } else {
        "0:v".to_string()
    };
    let mut xoff = 0u32; // pixel offset from x0
    let mut pass = 0usize;
    if let Some((_, dw)) = &date_png {
        // static calendar prefix — a bare --date draws this alone
        let sc = if fc.is_empty() { "" } else { ";" };
        let out = format!("v{pass}");
        fc.push_str(&format!(
            "{sc}[{cur}][{date_idx}:v]overlay=x={x0}:y={y}:shortest=1:{enable}[{out}]"
        ));
        cur = out;
        pass += 1;
        xoff += *dw + cw / 2;
    }
    if !date_only {
        fc.push_str(&format!(
            "{sc}[1:v]format=rgba[spr];[spr]split={}{split_labels}",
            xparts.len(),
            sc = if fc.is_empty() { "" } else { ";" }
        ));
        // crop each field out of the advancing sprite
        for (i, (kind, _)) in xparts.iter().enumerate() {
            let expr = match kind.as_str() {
                "hh" => format!("min(99,floor(({tv})/3600))"),
                "mm" => format!("mod(floor(({tv})/60),60)"),
                "cs" => format!("mod(floor(({tv})*100),100)"),
                "ff" => format!("mod(floor(({tv})*{fps:.5}),{ncells})"),
                _ => format!("mod(floor({tv}),60)"),
            };
            fc.push_str(&format!(
                ";[sp{i}]crop=w={cw}:h={ch}:x='{expr}*{cw}':y=0[f{i}]"
            ));
        }
        // overlay chain: field, colon, field, colon, field
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
    }

    let mut argv = ffmpeg_base(g.progress);
    argv.extend(["-i".to_string(), args.input.display().to_string()]);
    // image inputs in index order: sprite/colon/dot (skipped for a bare
    // --date), then box, then the date card — matches box_idx/date_idx
    if !date_only {
        for p in [&sprite_path, &colon_path, &dot_path] {
            argv.extend([
                "-loop".to_string(),
                "1".to_string(),
                "-framerate".to_string(),
                format!("{fps:.3}"),
                "-i".to_string(),
                p.display().to_string(),
            ]);
        }
    }
    if let Some((bp, _)) = &box_png {
        argv.extend([
            "-loop".to_string(),
            "1".to_string(),
            "-framerate".to_string(),
            format!("{fps:.3}"),
            "-i".to_string(),
            bp.display().to_string(),
        ]);
    }
    if let Some((dp, _)) = &date_png {
        argv.extend([
            "-loop".to_string(),
            "1".to_string(),
            "-framerate".to_string(),
            format!("{fps:.3}"),
            "-i".to_string(),
            dp.display().to_string(),
        ]);
    }
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
        "clock": args.clock,
        "date": args.date,
    }));
    Ok(c)
}

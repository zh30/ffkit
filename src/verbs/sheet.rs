use serde_json::json;

use crate::cli::{Globals, SheetArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: SheetArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "sheet")?;
    if args.cols == 0 || args.rows == 0 || args.cols * args.rows > 100 {
        return Err(Error::input("--cols x --rows must be 1..=100 tiles"));
    }
    let t0 = args
        .from
        .as_deref()
        .map(crate::time::parse_time)
        .transpose()?
        .unwrap_or(0.0);
    let t1 = args
        .to
        .as_deref()
        .map(crate::time::parse_time)
        .transpose()?
        .unwrap_or(probe.duration);
    if t0 < 0.0 || t1 <= t0 || t0 >= probe.duration {
        return Err(Error::input(
            "--from/--to need 0 <= from < to within the input",
        ));
    }
    let t1 = t1.min(probe.duration);
    let span = t1 - t0;
    let n = (args.cols * args.rows) as f64;
    let fps = n / span;
    // Even tile size so h264-ish math stays sane; tiles shrink on huge inputs.
    let w = probe.width.unwrap_or(1280);
    let h = probe.height.unwrap_or(720);
    let tw = args.tile.max(16);
    let th = ((tw * h / w) / 2) * 2;
    let vf = format!(
        "fps={fps:.6},scale={tw}:{th},tile={cols}x{rows}:margin={margin}:padding={pad}:color=0x101010",
        cols = args.cols,
        rows = args.rows,
        margin = args.margin.or(args.pad).unwrap_or(6),
        pad = args.pad.unwrap_or(6),
    );
    let mut argv = ffmpeg_base(g.progress);
    if t0 > 0.0 {
        argv.extend(["-ss", &crate::time::fmt_time(t0)]);
    }
    argv.push("-i");
    argv.push(&args.input);
    if t1 < probe.duration {
        argv.extend(["-t", &format!("{span:.3}")]);
    }
    argv.extend(["-vf", &vf, "-frames:v", "1"]);
    if args.time || args.title.is_some() {
        // Two passes: tile → sheet.png in a tempdir, then overlay rendered
        // PNGs (per-tile timestamps / a title header — no drawtext needed).
        let tmp = tempfile::tempdir().map_err(|e| Error::output(e.to_string()))?;
        let sheet_png = tmp.path().join("sheet.png");
        let mut a1 = argv.clone();
        a1.push(sheet_png.display().to_string());

        let n_tiles = (args.cols * args.rows) as usize;
        let font_path = crate::font::resolve(None)?;
        let font_bytes =
            std::fs::read(&font_path).map_err(|e| Error::input(format!("read font: {e}")))?;
        let margin = args.margin.or(args.pad).unwrap_or(6);
        let pad = args.pad.unwrap_or(6);
        let label_h = (th / 8).max(14);
        let mut a2 = ffmpeg_base(g.progress);
        a2.extend(["-i".to_string(), sheet_png.display().to_string()]);
        let mut fc = String::new();
        let mut cur = "0:v".to_string();
        let mut n_in = 1usize;
        for k in 0..n_tiles {
            let secs = t0 + span * (k as f64 + 0.5) / n_tiles as f64;
            let text = crate::time::fmt_time(secs.max(0.0));
            let img = crate::raster::render_caption_outlined(
                &text,
                &font_bytes,
                tw,
                [255, 255, 255],
                0.9,
                ([0, 0, 0], 2),
            )?;
            let png = tmp.path().join(format!("t{k}.png"));
            img.save(&png)
                .map_err(|e| Error::output(format!("write tile label: {e}")))?;
            a2.extend([
                "-loop".to_string(),
                "1".to_string(),
                "-i".to_string(),
                png.display().to_string(),
            ]);
            n_in += 1;
            let r = k / args.cols as usize;
            let c = k % args.cols as usize;
            let x = margin + c as u32 * (tw + pad);
            let y = margin + r as u32 * (th + pad) + th - label_h - 4;
            let out = format!("l{k}");
            fc.push_str(&format!(
                ";[{cur}][{}:v]overlay=x='{x}+({tw}-w)/2':y={y}:shortest=1[{out}]",
                k + 1
            ));
            cur = out;
        }
        if let Some(ttl) = &args.title {
            // Header row: pad the sheet down by the title card height.
            let sheet_w = margin * 2 + args.cols * tw + pad * (args.cols - 1);
            let card = crate::raster::render_title_outlined(
                ttl,
                &font_bytes,
                sheet_w,
                [255, 255, 255],
                0.7,
                ([0, 0, 0], 2),
            )?;
            let tpng = tmp.path().join("title.png");
            let th_ = card.height();
            card.save(&tpng)
                .map_err(|e| Error::output(format!("write title png: {e}")))?;
            a2.extend([
                "-loop".to_string(),
                "1".to_string(),
                "-i".to_string(),
                tpng.display().to_string(),
            ]);
            fc.push_str(&format!(
                ";[{cur}]pad=iw:ih+{th_}+8:0:{th_}+8:0x101010[pp];[pp][{n_in}:v]overlay=x=(W-w)/2:y=4:shortest=1[vttl]"
            ));
            cur = "vttl".to_string();
        }
        a2.extend([
            "-filter_complex".to_string(),
            fc.trim_start_matches(';').to_string(),
        ]);
        a2.extend(["-map".to_string(), format!("[{cur}]")]);
        a2.extend(["-frames:v".to_string(), "1".to_string()]);
        a2.push(args.output.display().to_string());
        let c = engine::write_job("sheet", &[&args.input], &args.output, vec![a1, a2], g)?;
        drop(tmp);
        return Ok(c.with_extra(json!({
            "cols": args.cols,
            "rows": args.rows,
            "tile": format!("{tw}x{th}"),
            "frames": args.cols * args.rows,
            "time": args.time,
            "title": args.title,
        })));
    }
    argv.push(&args.output);

    let c = engine::write_job("sheet", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "cols": args.cols,
        "rows": args.rows,
        "tile": format!("{tw}x{th}"),
        "frames": args.cols * args.rows,
    })))
}

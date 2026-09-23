use serde_json::json;

use crate::cli::{Globals, GridArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::spawn::Argv;

fn fs_pad(th: u32) -> u32 {
    (th / 12).max(4)
}

fn parse_wxh(s: &str, flag: &str) -> Result<(u32, u32), Error> {
    let (w, h) = s
        .split_once('x')
        .and_then(|(a, b)| Some((a.trim().parse::<u32>().ok()?, b.trim().parse::<u32>().ok()?)))
        .filter(|(w, h)| *w > 0 && *h > 0)
        .ok_or_else(|| Error::input(format!("{flag} must look like 1920x1080 / 2x2")))?;
    Ok((w, h))
}

pub fn run(args: GridArgs, g: &Globals) -> Result<Contract, Error> {
    let n = args.inputs.len();
    if n < 2 {
        return Err(Error::input("grid needs at least 2 inputs"));
    }
    if n > 16 {
        return Err(Error::input("grid accepts at most 16 inputs"));
    }
    let (cols, rows) = parse_wxh(&args.layout, "--layout")?;
    let cells = (cols * rows) as usize;
    if n > cells && !args.focus {
        return Err(Error::input(format!(
            "{n} inputs don't fit a {}x{} grid ({cells} cells)",
            cols, rows
        )));
    }
    if args.focus && n < 2 {
        return Err(Error::input("--focus needs at least 2 inputs"));
    }
    let (cw, ch) = parse_wxh(&args.size, "--size")?;
    let (tw, th) = (cw / cols, ch / rows);
    let gap = args.gap.unwrap_or(0);
    if gap >= tw.min(th) / 2 && !args.focus {
        return Err(Error::input("--gap is too big for the tile size"));
    }
    // Shrink each tile inside its cell, then pad back to the full cell —
    // a uniform gutter around every tile without touching the stack.
    let gutter = args
        .bg
        .as_deref()
        .map(crate::color::lavfi)
        .unwrap_or_else(|| "black".to_string());
    // per-tile rects (x, y, w, h): uniform grid, or --focus hero layout —
    // tile 0 fills the left ~2/3 column, the rest stack in the right column
    let hero_w = (cw * 2).div_ceil(3);
    let side_w = cw - hero_w;
    let side_h = ch / (n as u32 - 1);
    let rects: Vec<(u32, u32, u32, u32)> = (0..n)
        .map(|i| {
            if args.focus {
                if i == 0 {
                    (0, 0, hero_w, ch)
                } else {
                    (
                        hero_w,
                        (i as u32 - 1) * side_h,
                        side_w,
                        if i == n - 1 {
                            ch - (i as u32 - 1) * side_h
                        } else {
                            side_h
                        },
                    )
                }
            } else {
                ((i as u32 % cols) * tw, (i as u32 / cols) * th, tw, th)
            }
        })
        .collect();
    if rects.iter().any(|r| r.2 < 16 || r.3 < 16) {
        return Err(Error::input(
            "--size too small for that --layout (tiles < 16px)",
        ));
    }

    let mut probes = Vec::new();
    for f in &args.inputs {
        let p = engine::probe_or_err(f, g)?;
        engine::need_video(&p, "grid")?;
        probes.push(p);
    }
    let all_audio = probes.iter().all(|p| p.has_audio);
    if let Some(idx) = args.audio {
        if idx >= n {
            return Err(Error::input(format!(
                "--audio {idx} out of range for {n} inputs"
            )));
        }
        if !probes[idx].has_audio {
            return Err(Error::input(format!(
                "input {idx} has no audio for --audio"
            )));
        }
    }

    let mut seg: Vec<String> = Vec::new();
    let mut layout_str = String::new();
    for (i, &(rx, ry, rw, rh)) in rects.iter().enumerate() {
        let (itw, ith) = (rw.saturating_sub(gap), rh.saturating_sub(gap));
        let fit = if args.fill {
            // crop-overflow fill: scale up until the cell is covered, then crop
            format!("scale={itw}:{ith}:force_original_aspect_ratio=increase,crop={itw}:{ith}")
        } else {
            format!("scale={itw}:{ith}:force_original_aspect_ratio=decrease")
        };
        seg.push(format!(
            "[{i}:v]{fit},pad={rw}:{rh}:(ow-iw)/2:(oh-ih)/2:{gutter},setsar=1,fps=30,format=yuv420p[v{i}]"
        ));
        if !layout_str.is_empty() {
            layout_str.push('|');
        }
        layout_str.push_str(&format!("{rx}_{ry}"));
        if args.audio == Some(i) {
            seg.push(format!(
                "[{i}:a]aresample=48000,aformat=channel_layouts=stereo[aout]"
            ));
        } else if args.audio.is_none() && all_audio {
            seg.push(format!(
                "[{i}:a]aresample=48000,aformat=channel_layouts=stereo[a{i}]"
            ));
        }
    }
    let mut label_pngs: Vec<(u32, std::path::PathBuf, tempfile::TempDir)> = Vec::new();
    if let Some(raw) = &args.labels {
        let font_path = crate::font::resolve(None)?;
        let font_bytes =
            std::fs::read(&font_path).map_err(|e| Error::input(format!("read font: {e}")))?;
        for (i, text) in raw.split(',').enumerate().take(n) {
            let text = text.trim();
            if text.is_empty() {
                continue;
            }
            // label fills ~3/4 of a tile row
            let img = crate::raster::render_caption_outlined(
                text,
                &font_bytes,
                rects[i].2,
                [255, 255, 255],
                0.9,
                ([0, 0, 0], 2),
            )?;
            let tmp = tempfile::tempdir().map_err(|e| Error::output(e.to_string()))?;
            let png = tmp.path().join(format!("lbl{i}.png"));
            img.save(&png)
                .map_err(|e| Error::output(format!("write label png: {e}")))?;
            label_pngs.push((i as u32, png, tmp));
        }
    }
    // --time: per-tile mm:ss readout — one shared sprite drives every tile,
    // so all clocks read identically frame-for-frame.
    let mut timer_pngs: Vec<std::path::PathBuf> = Vec::new();
    let mut timer_tmp: Option<tempfile::TempDir> = None;
    if args.time {
        let font_path = crate::font::resolve(None)?;
        let font_bytes =
            std::fs::read(&font_path).map_err(|e| Error::input(format!("read font: {e}")))?;
        // render_title_styled's `size` is a multiplier: px = vw/8 * size —
        // target px ≈ tile_h/8 ⇒ size = tile_h/tile_w (smallest tile decides
        // on --focus so the clock never overflows the skinny column)
        let (mw, mh) = rects
            .iter()
            .fold((u32::MAX, u32::MAX), |(w, h), r| (w.min(r.2), h.min(r.3)));
        let fs = (mh as f32 / mw as f32).clamp(0.2, 3.0);
        let mut cells: Vec<image::RgbaImage> = Vec::new();
        let (mut cw2, mut ch2) = (0u32, 0u32);
        for i in 0..60u32 {
            let img = crate::raster::render_title_styled(
                &format!("{i:02}"),
                &font_bytes,
                tw,
                [255, 255, 255],
                fs,
            )?;
            cw2 = cw2.max(img.width());
            ch2 = ch2.max(img.height());
            cells.push(img);
        }
        let mut sprite = image::RgbaImage::new(cw2 * 60, ch2);
        for (i, cell) in cells.iter().enumerate() {
            image::imageops::overlay(&mut sprite, cell, (i as u32 * cw2) as i64, 0);
        }
        let colon = crate::raster::render_title_styled(":", &font_bytes, tw, [255, 255, 255], fs)?;
        let tmp = tempfile::tempdir().map_err(|e| Error::output(e.to_string()))?;
        let sp = tmp.path().join("tspr.png");
        sprite
            .save(&sp)
            .map_err(|e| Error::output(format!("write sprite: {e}")))?;
        let cp = tmp.path().join("tcol.png");
        colon
            .save(&cp)
            .map_err(|e| Error::output(format!("write colon: {e}")))?;
        timer_pngs.push(sp);
        timer_pngs.push(cp);
        timer_tmp = Some(tmp);
        // input indexes: N videos + labels + sprite + colon
        let spr_idx = n + label_pngs.len();
        let col_idx = spr_idx + 1;
        let m = (ch2 / 4).max(4);
        let total_w = 2 * cw2 + colon.width();
        // split sprite into per-tile mm/ss field feeds + colon feed
        seg.push(format!(
            "[{spr_idx}:v]format=rgba,split={k}{feeds}",
            k = 2 * n,
            feeds = (0..2 * n).map(|i| format!("[tsp{i}]")).collect::<String>(),
        ));
        for i in 0..n {
            seg.push(format!(
                "[tsp{i}]crop=w={cw2}:h={ch2}:x='mod(floor(t/60),60)*{cw2}':y=0[tmm{i}]"
            ));
            seg.push(format!(
                "[tsp{}]crop=w={cw2}:h={ch2}:x='mod(floor(t),60)*{cw2}':y=0[tss{i}]",
                i + n
            ));
        }
        seg.push(format!(
            "[{col_idx}:v]format=rgba,split={n}{feeds}",
            feeds = (0..n).map(|i| format!("[tcl{i}]")).collect::<String>(),
        ));
        // overlay mm:ss at each tile's bottom-right, applied on [v{i}] before xstack
        for (i, &(rx, ry, rw, rh)) in rects.iter().enumerate() {
            let x = rx + rw - (total_w + m).min(rw);
            let y = ry + rh - (ch2 + m).min(rh);
            let w1 = format!("[tv{i}a]");
            let w2 = format!("[tv{i}b]");
            seg.push(format!("[v{i}][tmm{i}]overlay={x}:{y}:shortest=1{w1}"));
            seg.push(format!(
                "{w1}[tcl{i}]overlay={}:{y}:shortest=1{w2}",
                x + cw2
            ));
            seg.push(format!(
                "{w2}[tss{i}]overlay={}:{y}:shortest=1[vt{i}]",
                x + cw2 + colon.width()
            ));
        }
    }
    let ins: String = (0..n)
        .map(|i| {
            if args.time {
                format!("[vt{i}]")
            } else {
                format!("[v{i}]")
            }
        })
        .collect();
    let vfirst = if label_pngs.is_empty() {
        "[vout]"
    } else {
        "[vg]"
    };
    seg.push(format!(
        "{ins}xstack=inputs={n}:layout={layout_str}{vfirst}"
    ));
    if !label_pngs.is_empty() {
        let mut cur = String::from("[vg]");
        for (j, (idx, _, _)) in label_pngs.iter().enumerate() {
            let (rx, ry, rw, rh) = rects[*idx as usize];
            let next = if j + 1 == label_pngs.len() {
                "[vout]".to_string()
            } else {
                format!("[vo{j}]")
            };
            let src = n + j;
            seg.push(format!(
                "{cur}[{src}:v]overlay={rx}+({rw}-w)/2:{ry}+{rh}-h-{pad}:shortest=1{next}",
                pad = (fs_pad(rh)),
            ));
            cur = next;
        }
    }
    if args.audio.is_none() && all_audio {
        let ains: String = (0..n).map(|i| format!("[a{i}]")).collect();
        seg.push(format!("{ains}amix=inputs={n}:normalize=0[aout]"));
    }
    let fc = seg.join(";");

    let mut argv = ffmpeg_base(g.progress);
    for f in &args.inputs {
        argv.push("-i");
        argv.push(f);
    }
    for (_, png, _) in &label_pngs {
        argv.extend(["-loop", "1", "-i"]);
        argv.push(png);
    }
    for png in &timer_pngs {
        argv.extend(["-loop", "1", "-i"]);
        argv.push(png);
    }
    // timer_tmp stays alive until after write_job — the PNGs live inside it
    argv.extend(["-filter_complex", &fc, "-map", "[vout]"]);
    if all_audio || args.audio.is_some() {
        argv.extend(["-map", "[aout]", "-c:a", "aac"]);
    }
    argv.extend([
        "-c:v",
        "libx264",
        "-preset",
        "fast",
        "-crf",
        "18",
        "-pix_fmt",
        "yuv420p",
        "-shortest",
    ]);
    argv.push(&args.output);

    let refs: Vec<&std::path::Path> = args.inputs.iter().map(|p| p.as_path()).collect();
    let argvs: Vec<Argv> = vec![argv];
    let c = engine::write_job("grid", &refs, &args.output, argvs, g)?;
    drop(timer_tmp);
    drop(label_pngs);
    let extra = json!({
        "inputs": n,
        "layout": args.layout,
        "size": args.size,
        "tile": format!("{tw}x{th}"),
        "mixed_audio": all_audio && args.audio.is_none(),
        "audio_from": args.audio,
    });
    Ok(c.with_extra(extra))
}

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
    if n > cells {
        return Err(Error::input(format!(
            "{n} inputs don't fit a {}x{} grid ({cells} cells)",
            cols, rows
        )));
    }
    let (cw, ch) = parse_wxh(&args.size, "--size")?;
    let (tw, th) = (cw / cols, ch / rows);
    if tw < 16 || th < 16 {
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
    for i in 0..n {
        let col = i as u32 % cols;
        let row = i as u32 / cols;
        seg.push(format!(
            "[{i}:v]scale={tw}:{th}:force_original_aspect_ratio=decrease,pad={tw}:{th}:(ow-iw)/2:(oh-ih)/2,setsar=1,fps=30,format=yuv420p[v{i}]"
        ));
        if !layout_str.is_empty() {
            layout_str.push('|');
        }
        layout_str.push_str(&format!("{}_{}", col * tw, row * th));
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
                tw,
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
    let ins: String = (0..n).map(|i| format!("[v{i}]")).collect();
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
            let col = *idx % cols;
            let row = *idx / cols;
            let next = if j + 1 == label_pngs.len() {
                "[vout]".to_string()
            } else {
                format!("[vo{j}]")
            };
            let src = n + j;
            seg.push(format!(
                "{cur}[{src}:v]overlay={cx}+({tw}-w)/2:{ry}+{th}-h-{pad}:shortest=1{next}",
                cx = col * tw,
                ry = row * th,
                pad = (fs_pad(th)),
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

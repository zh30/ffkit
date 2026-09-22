use std::path::Path;

use crate::cli::{CaptionArgs, CaptionMode, CaptionPosition, CaptionSafe, Globals};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::paths;
use crate::srt;

fn caption_hex(c: &str) -> Result<[u8; 3], Error> {
    let c = c.trim_start_matches('#');
    if c.len() != 6 {
        return Err(Error::input("--color must be RRGGBB hex"));
    }
    let b = |i: usize| -> Result<u8, Error> {
        u8::from_str_radix(&c[i..i + 2], 16).map_err(|_| Error::input("--color must be RRGGBB hex"))
    };
    Ok([b(0)?, b(2)?, b(4)?])
}

pub fn run(args: CaptionArgs, g: &Globals) -> Result<Contract, Error> {
    paths::ensure_input(&args.srt)?;
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "caption")?;

    match args.mode {
        CaptionMode::Burn => burn_overlay(&args, g, &probe),
        CaptionMode::Mux => mux(&args, g, probe.has_audio),
    }
}

fn mux(args: &CaptionArgs, g: &Globals, has_audio: bool) -> Result<Contract, Error> {
    let _ = has_audio;
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.push("-i");
    argv.push(&args.srt);
    argv.extend(["-c", "copy"]);
    let ext = args
        .output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("mp4")
        .to_ascii_lowercase();
    if ext == "mp4" || ext == "mov" || ext == "m4v" {
        argv.extend(["-c:s", "mov_text"]);
    }
    argv.push(&args.output);
    engine::write_job(
        "caption",
        &[&args.input, &args.srt],
        &args.output,
        vec![argv],
        g,
    )
}

fn burn_overlay(
    args: &CaptionArgs,
    g: &Globals,
    probe: &crate::probe::Probe,
) -> Result<Contract, Error> {
    let raw = std::fs::read_to_string(&args.srt)?;
    let mut cues = srt::parse_srt(&raw)?;
    if args.shift != 0.0 {
        for c in cues.iter_mut() {
            c.start = (c.start + args.shift).max(0.0);
            c.end = (c.end + args.shift).max(c.start + 0.05);
        }
    }
    if let Some(n) = args.chunk {
        if !(1..=10).contains(&n) {
            return Err(Error::input("--chunk must be 1..=10 words"));
        }
        cues = chunk_cues(cues, n as usize);
    }
    if cues.len() > 80 {
        return Err(Error::input(
            "caption burn supports at most 80 cues; split the srt or use graph",
        ));
    }
    let font_path = crate::font::resolve(args.font.as_deref().map(Path::new))?;
    let font_bytes = std::fs::read(&font_path)?;
    let vw = probe.width.unwrap_or(1280);
    let cap_fg = match &args.color {
        Some(c) => caption_hex(c)?,
        None => [255, 255, 255],
    };
    if !(0.25..=8.0).contains(&args.size) {
        return Err(Error::input("--size must be 0.25..8"));
    }

    let tmp = tempfile::tempdir().map_err(|e| Error::output(e.to_string()))?;
    let mut pngs = Vec::new();
    let outline = match &args.outline {
        Some(c) => Some((caption_hex(c)?, 3u32)),
        None => None,
    };
    for (i, cue) in cues.iter().enumerate() {
        let img = match outline {
            Some(oc) => crate::raster::render_caption_outlined(
                &cue.text,
                &font_bytes,
                vw,
                cap_fg,
                args.size as f32,
                oc,
            )?,
            None => crate::raster::render_caption_styled(
                &cue.text,
                &font_bytes,
                vw,
                cap_fg,
                args.size as f32,
            )?,
        };
        let png = tmp.path().join(format!("c{i}.png"));
        img.save(&png)
            .map_err(|e| Error::output(format!("write caption png: {e}")))?;
        pngs.push(png);
    }

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    for png in &pngs {
        argv.push("-i");
        argv.push(png);
    }

    let y = overlay_y(args.safe, args.position);
    let mut fc = String::new();
    let mut last = "0:v".to_string();
    for (i, cue) in cues.iter().enumerate() {
        let ov_idx = i + 1;
        let out_lab = if i + 1 == cues.len() {
            "vout".to_string()
        } else {
            format!("v{i}")
        };
        fc.push_str(&format!(
            "[{last}][{ov_idx}:v]overlay=x=(W-w)/2:y={y}:enable='between(t,{:.3},{:.3})'[{out_lab}]",
            cue.start, cue.end
        ));
        if i + 1 != cues.len() {
            fc.push(';');
        }
        last = out_lab;
    }

    argv.extend(["-filter_complex", &fc, "-map", "[vout]"]);
    if probe.has_audio {
        argv.extend(["-map", "0:a", "-c:a", "copy"]);
    }
    argv.extend([
        "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
    ]);
    argv.push(&args.output);

    let inputs: Vec<&Path> = vec![&args.input, &args.srt];
    let result = engine::write_job("caption", &inputs, &args.output, vec![argv], g);
    drop(tmp);
    let mut c = result?;
    c = c.with_extra(serde_json::json!({
        "renderer": "overlay",
        "cues": cues.len(),
        "chunk": args.chunk,
        "font": font_path.display().to_string(),
        "safe": match args.safe {
            CaptionSafe::Social => "social",
            CaptionSafe::Off => "off",
        },
        "position": match args.position {
            CaptionPosition::Bottom => "bottom",
            CaptionPosition::Top => "top",
        },
        "bottom_frac": match args.safe {
            CaptionSafe::Social => 0.20,
            CaptionSafe::Off => 0.15,
        },
    }));
    Ok(c)
}

// Split each cue into ≤n-word chunks shown one at a time, sharing the cue's
// span evenly — the chunked "karaoke-ish" look without needing word timing.
fn chunk_cues(cues: Vec<srt::Cue>, n: usize) -> Vec<srt::Cue> {
    let mut out = Vec::new();
    for cue in cues {
        let words: Vec<&str> = cue.text.split_whitespace().collect();
        if words.len() <= n {
            out.push(cue);
            continue;
        }
        let span = cue.end - cue.start;
        let k = words.len().div_ceil(n);
        for (i, group) in words.chunks(n).enumerate() {
            out.push(srt::Cue {
                start: cue.start + span * i as f64 / k as f64,
                end: cue.start + span * (i + 1) as f64 / k as f64,
                text: group.join(" "),
            });
        }
    }
    out
}

fn overlay_y(safe: CaptionSafe, pos: CaptionPosition) -> &'static str {
    match pos {
        // Cross-post 2026 chrome: bottom ~20% (TikTok 320–350px / Reels up to 450px on 1920).
        CaptionPosition::Bottom => match safe {
            CaptionSafe::Social => "H-h-trunc(H*0.20)",
            CaptionSafe::Off => "H-h-trunc(H*0.15)",
        },
        // Top captions sit under ~15% chrome (platform titles/progress).
        CaptionPosition::Top => match safe {
            CaptionSafe::Social => "trunc(H*0.15)",
            CaptionSafe::Off => "trunc(H*0.10)",
        },
    }
}

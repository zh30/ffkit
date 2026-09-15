use std::path::Path;

use crate::cli::{CaptionArgs, CaptionMode, CaptionSafe, Globals};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::paths;
use crate::srt;

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
    let cues = srt::parse_srt(&raw)?;
    if cues.len() > 80 {
        return Err(Error::input(
            "caption burn supports at most 80 cues; split the srt or use graph",
        ));
    }
    let font_path = crate::font::resolve(args.font.as_deref().map(Path::new))?;
    let font_bytes = std::fs::read(&font_path)?;
    let vw = probe.width.unwrap_or(1280);

    let tmp = tempfile::tempdir().map_err(|e| Error::output(e.to_string()))?;
    let mut pngs = Vec::new();
    for (i, cue) in cues.iter().enumerate() {
        let img = crate::raster::render_caption(&cue.text, &font_bytes, vw)?;
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

    let y = overlay_y(args.safe);
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
        "font": font_path.display().to_string(),
        "safe": match args.safe {
            CaptionSafe::Social => "social",
            CaptionSafe::Off => "off",
        },
        "bottom_frac": match args.safe {
            CaptionSafe::Social => 0.20,
            CaptionSafe::Off => 0.15,
        },
    }));
    Ok(c)
}

fn overlay_y(safe: CaptionSafe) -> &'static str {
    match safe {
        // Cross-post 2026 chrome: bottom ~20% (TikTok 320–350px / Reels up to 450px on 1920).
        CaptionSafe::Social => "H-h-trunc(H*0.20)",
        CaptionSafe::Off => "H-h-trunc(H*0.15)",
    }
}

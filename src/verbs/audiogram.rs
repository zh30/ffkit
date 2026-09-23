use std::path::Path;

use serde_json::json;

use crate::cli::{AudiogramArgs, Globals, WaveMode};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::paths;

/// Podcast clip → 1080x1920 video: cover still (or flat colour) with a
/// showwaves strip keyed over it. Audio is re-encoded to aac.
pub fn run(args: AudiogramArgs, g: &Globals) -> Result<Contract, Error> {
    let (w, h) = args
        .size
        .split_once('x')
        .and_then(|(a, b)| Some((a.parse::<u32>().ok()?, b.parse::<u32>().ok()?)))
        .filter(|(w, h)| *w >= 64 && *h >= 64)
        .ok_or_else(|| Error::input("--size must be WxH (min 64x64)"))?;
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_audio {
        return Err(Error::input("audiogram: input has no audio stream"));
    }
    if let Some(img) = &args.image {
        paths::ensure_input(img)?;
    }

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    if let Some(img) = &args.image {
        argv.extend(["-loop", "1", "-i"]);
        argv.push(img);
    } else {
        let bg = args.bg.clone().unwrap_or_else(|| "0x101418".to_string());
        let bg = if bg.starts_with("0x") || bg.chars().all(|c| c.is_ascii_alphabetic()) {
            bg
        } else {
            format!("0x{}", bg.trim_start_matches('#'))
        };
        argv.extend(["-f", "lavfi", "-i", &format!("color=c={bg}:s={w}x{h}:r=30")]);
    }

    // Waveform sits in the lower-middle band — clear of Reels/TikTok top and
    // bottom chrome — and the black showwaves floor is keyed out so the cover
    // shows through. overlay shortest=1 ends [vout] with the waveform: -shortest
    // alone overshoots because the encoder queue keeps the infinite cover
    // going past audio EOF.
    if (args.scale.is_some() || args.split) && matches!(args.mode, WaveMode::Spectrum) {
        return Err(Error::input(
            "--scale/--split apply to waveform modes (not spectrum)",
        ));
    }
    let fs = match &args.fscale {
        Some(s) if ["lin", "log", "rlog"].contains(&s.as_str()) => {
            format!(":fscale={s}")
        }
        Some(_) => return Err(Error::input("--fscale: lin|log|rlog")),
        None => String::new(),
    };
    if args.fscale.is_some() && !matches!(args.mode, WaveMode::Spectrum) {
        return Err(Error::input("--fscale applies to --mode spectrum only"));
    }
    let fps = args.fps.unwrap_or(30.0);
    if !(1.0..=120.0).contains(&fps) {
        return Err(Error::input("--fps must be 1..=120"));
    }
    // showfreqs gained `rate` only in ffmpeg 7; on 4.x the option is absent.
    let freq_rate = if engine::ffmpeg_major().unwrap_or(9) >= 7 {
        format!(":rate={fps}")
    } else {
        if args.fps.is_some() && matches!(args.mode, WaveMode::Spectrum) {
            return Err(Error::input("--fps needs ffmpeg ≥7 with --mode spectrum"));
        }
        String::new()
    };
    // Spectrum renders frequency bars via showfreqs; the rest use showwaves.
    let (wave_src, mode) = match args.mode {
        WaveMode::Spectrum => (
            format!(
                "[0:a]showfreqs=s={{ww}}x{{wh}}:mode=bar{freq_rate}:colors={}{fs}[wv];",
                crate::color::lavfi(&args.color)
            ),
            "spectrum",
        ),
        m => {
            let name = match m {
                WaveMode::Point => "point",
                WaveMode::Line => "line",
                WaveMode::P2p => "p2p",
                WaveMode::Cline => "cline",
                WaveMode::Spectrum => unreachable!(),
            };
            (
                {
                    let sc = wave_scale(&args)?;
                    let sp = if args.split { ":split_channels=1" } else { "" };
                    format!(
                        "[0:a]showwaves=s={{ww}}x{{wh}}:mode={name}:rate={fps}:colors={}:draw=full{sc}{sp}[wv];",
                        crate::color::lavfi(&args.color)
                    )
                },
                name,
            )
        }
    };
    // --text: rasterize a small title into a PNG and overlay it near the top.
    let mut title_png = None;
    if let Some(text) = &args.text {
        let font_path = crate::font::resolve(args.font.as_deref().map(std::path::Path::new))?;
        let font_bytes =
            std::fs::read(&font_path).map_err(|e| Error::input(format!("read font: {e}")))?;
        let img = crate::raster::render_caption(text, &font_bytes, w)?;
        let tmp = tempfile::tempdir().map_err(|e| Error::output(e.to_string()))?;
        let png = tmp.path().join("ag_text.png");
        img.save(&png)
            .map_err(|e| Error::output(format!("write text png: {e}")))?;
        argv.extend(["-loop", "1", "-i"]);
        argv.push(&png);
        title_png = Some(tmp);
    }
    // --progress: thin bar sweeping the bottom edge over the clip duration.
    let mut prog_tmp = None;
    if args.progress {
        let mut bar = image::RgbaImage::new(6, 24);
        for px in bar.pixels_mut() {
            *px = image::Rgba([255, 255, 255, 235]);
        }
        let tmp = tempfile::tempdir().map_err(|e| Error::output(e.to_string()))?;
        let png = tmp.path().join("prog.png");
        bar.save(&png)
            .map_err(|e| Error::output(format!("write progress png: {e}")))?;
        argv.extend(["-loop", "1", "-i"]);
        argv.push(&png);
        prog_tmp = Some(tmp);
    }
    // --subs: burn .srt cues along the bottom strip (podcast-clip captions).
    let mut sub_tmp = None;
    let mut sub_cues: Vec<crate::srt::Cue> = Vec::new();
    if let Some(srt_path) = &args.subs {
        paths::ensure_input(srt_path)?;
        let raw = std::fs::read_to_string(srt_path)
            .map_err(|e| Error::input(format!("read subs: {e}")))?;
        let cues = crate::srt::parse_srt(&raw)?;
        if cues.len() > 60 {
            return Err(Error::input("audiogram --subs supports at most 60 cues"));
        }
        let font_path = crate::font::resolve(args.font.as_deref().map(std::path::Path::new))?;
        let font_bytes =
            std::fs::read(&font_path).map_err(|e| Error::input(format!("read font: {e}")))?;
        let tmp = tempfile::tempdir().map_err(|e| Error::output(e.to_string()))?;
        for (i, cue) in cues.iter().enumerate() {
            let img = crate::raster::render_caption(&cue.text, &font_bytes, w)?;
            let png = tmp.path().join(format!("sub{i}.png"));
            img.save(&png)
                .map_err(|e| Error::output(format!("write sub png: {e}")))?;
            argv.extend(["-loop", "1", "-i"]);
            argv.push(&png);
        }
        sub_cues = cues;
        sub_tmp = Some(tmp);
    }
    let first_sub = 2 + title_png.is_some() as usize + args.progress as usize;
    let prog_idx = if title_png.is_some() { 3 } else { 2 };
    // Chain: [bg][wvk]overlay→[mid] → optional title overlay → optional progress
    // bar. The first token in `tail` labels the waveform overlay's output.
    let mut tail = String::new();
    match (title_png.is_some(), args.progress) {
        (true, true) => tail.push_str(&format!(
            "[mid];[mid][2:v]overlay=(W-w)/2:(H-h)*0.16:shortest=1[mid2];\
             [mid2][{prog_idx}:v]overlay='(W-w)*t/{:.3}':H-h-6:shortest=1[vout]",
            probe.duration.max(0.01)
        )),
        (true, false) => {
            tail.push_str("[mid];[mid][2:v]overlay=(W-w)/2:(H-h)*0.16:shortest=1[vout]")
        }
        (false, true) => tail.push_str(&format!(
            "[mid];[mid][{prog_idx}:v]overlay='(W-w)*t/{:.3}':H-h-6:shortest=1[vout]",
            probe.duration.max(0.01)
        )),
        (false, false) => tail.push_str("[vout]"),
    }
    if !sub_cues.is_empty() {
        tail = tail.replace("[vout]", "[pre]");
        let mut last = "pre".to_string();
        for (i, cue) in sub_cues.iter().enumerate() {
            let lab = if i + 1 == sub_cues.len() {
                "vout".to_string()
            } else {
                format!("cap{i}")
            };
            tail.push_str(&format!(
                ";[{last}][{}:v]overlay=x=(W-w)/2:y=H-h-trunc(H*0.10):enable='between(t,{:.3},{:.3})'[{lab}]",
                first_sub + i,
                cue.start,
                cue.end
            ));
            last = lab;
        }
    }
    let yf = match args.position.as_deref().unwrap_or("bottom") {
        "top" => "0.18",
        "center" | "middle" => "0.50",
        "bottom" => "0.62",
        other => {
            return Err(Error::input(format!(
                "--position must be top/center/bottom (got {other})"
            )))
        }
    };
    let fc = format!(
        "[1:v]scale={w}:{h}:force_original_aspect_ratio=increase,crop={w}:{h},setsar=1[bg];\
              {wave}\
              [wv]colorkey=0x000000:0.12:0.1[wvk];\
              [bg][wvk]overlay=(W-w)/2:(H-h)*{yf}:shortest=1{tail}",
        wave = wave_src
            .replace("{ww}", &((w as f64 * 0.87).round() as u32 & !1).to_string())
            .replace(
                "{wh}",
                &(((h as f64) / 6.0).round().max(40.0) as u32 & !1).to_string()
            ),
        yf = yf,
    );
    argv.extend(["-filter_complex", &fc, "-map", "[vout]", "-map", "0:a"]);
    argv.extend([
        "-c:v",
        "libx264",
        "-preset",
        "fast",
        "-crf",
        "20",
        "-pix_fmt",
        "yuv420p",
        "-c:a",
        "aac",
        "-shortest",
    ]);
    argv.push(&args.output);

    let mut inputs: Vec<&Path> = vec![&args.input];
    if let Some(img) = &args.image {
        inputs.push(img);
    }
    if let Some(s) = &args.subs {
        inputs.push(s);
    }
    let mut c = engine::write_job("audiogram", &inputs, &args.output, vec![argv], g)?;
    drop(prog_tmp);
    drop(sub_tmp);
    c = c.with_extra(json!({
        "frame": "1080x1920",
        "waveform": "showwaves",
        "mode": mode,
        "color": crate::color::lavfi(&args.color),
        "sub_cues": sub_cues.len(),
    }));
    Ok(c)
}

fn wave_scale(args: &crate::cli::AudiogramArgs) -> Result<String, Error> {
    match &args.scale {
        Some(s) => {
            if !["lin", "log", "sqrt", "cbrt"].contains(&s.as_str()) {
                return Err(Error::input("--scale: lin|log|sqrt|cbrt"));
            }
            Ok(format!(":scale={s}"))
        }
        None => Ok(String::new()),
    }
}

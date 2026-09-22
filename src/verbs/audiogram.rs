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
    let mode = match args.mode {
        WaveMode::Point => "point",
        WaveMode::Line => "line",
        WaveMode::P2p => "p2p",
        WaveMode::Cline => "cline",
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
    let tail = if title_png.is_some() {
        // text sits near the top, above the waveform band
        "[mid];[mid][2:v]overlay=(W-w)/2:(H-h)*0.16:shortest=1[vout]"
    } else {
        "[vout]"
    };
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
              [0:a]showwaves=s={ww}x{wh}:mode={mode}:rate=30:colors={}:draw=full[wv];\
              [wv]colorkey=0x000000:0.12:0.1[wvk];\
              [bg][wvk]overlay=(W-w)/2:(H-h)*{yf}:shortest=1{tail}",
        args.color,
        ww = (w as f64 * 0.87).round() as u32 & !1,
        yf = yf,
        wh = ((h as f64) / 6.0).round().max(40.0) as u32 & !1,
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
    let mut c = engine::write_job("audiogram", &inputs, &args.output, vec![argv], g)?;
    c = c.with_extra(json!({
        "frame": "1080x1920",
        "waveform": "showwaves",
        "mode": mode,
        "color": args.color,
    }));
    Ok(c)
}

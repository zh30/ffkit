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
    if args.at.is_some() && (args.from.is_some() || args.to.is_some()) {
        return Err(Error::input("pass --at or --from/--to, not both"));
    }
    if args.dur.is_some() && args.at.is_none() && args.from.is_none() {
        return Err(Error::input("--dur needs --at or --from"));
    }
    if let Some(d) = args.dur {
        if !(0.05..=3600.0).contains(&d) {
            return Err(Error::input("--dur must be 0.05..=3600s"));
        }
    }

    // comma --at: one audiogram per start point → `<stem>_N.<ext>`
    if let Some(raw) = &args.at {
        if raw.split(',').count() > 1 {
            let mut files = Vec::new();
            let mut first = None;
            for (i, part) in raw.split(',').enumerate() {
                let t0 = crate::time::resolve_frame_at(part.trim(), probe.duration)?;
                if t0 < 0.0 || t0 >= probe.duration {
                    return Err(Error::input(format!(
                        "--at {t0} is outside the {:.2}s audio",
                        probe.duration
                    )));
                }
                let out = derive_output(&args.output, i + 1);
                let t1 = args.dur.map(|d| (t0 + d).min(probe.duration));
                let c = render_clip(&args, g, &probe, w, h, t0, t1, &out)?;
                if i == 0 {
                    first = Some(c);
                }
                files.push(out.to_string_lossy().into_owned());
            }
            return Ok(first.unwrap().with_extra(json!({ "files": files })));
        }
    }

    let from = match (&args.from, &args.at) {
        (Some(s), _) | (_, Some(s)) if s.trim().eq_ignore_ascii_case("end") => probe.duration,
        (Some(s), _) | (_, Some(s)) if s.trim().to_ascii_lowercase().starts_with("end-") => {
            probe.duration - crate::time::parse_time(&s.trim()[4..])?
        }
        (Some(s), _) | (_, Some(s)) => crate::time::parse_time(s)?,
        _ => 0.0,
    };
    let to = match (&args.to, args.dur) {
        (Some(s), _) if s.trim().eq_ignore_ascii_case("end") => Some(probe.duration),
        (Some(s), _) => Some(crate::time::parse_time(s)?),
        (None, Some(d)) => Some((from + d).min(probe.duration)),
        (None, None) => None,
    };
    if from < 0.0 {
        return Err(Error::input("--from must be >= 0"));
    }
    if let Some(t) = to {
        if t <= from {
            return Err(Error::input("--to must be later than --from"));
        }
    }
    if from >= probe.duration {
        return Err(Error::input("--from is past the end of the audio"));
    }
    render_clip(&args, g, &probe, w, h, from, to, &args.output)
}

#[allow(clippy::too_many_arguments)]
fn render_clip(
    args: &AudiogramArgs,
    g: &Globals,
    probe: &crate::probe::Probe,
    w: u32,
    h: u32,
    from: f64,
    to: Option<f64>,
    output: &Path,
) -> Result<Contract, Error> {
    let ranged = from > 0.0 || to.is_some();
    let clip_dur = to.unwrap_or(probe.duration) - from;
    // Ranged: atrim clips the audio once, asplit feeds the waveform input
    // ([wvin]) and the mapped audio ([amap]) — pads are single-consumer.
    let (range_fc, awave, amap): (String, &str, &str) = if ranged {
        let end_part = to.map(|t| format!(":end={t}")).unwrap_or_default();
        (
            format!("[0:a]atrim=start={from}{end_part},asetpts=PTS-STARTPTS[a0];[a0]asplit=2[amap][wvin];"),
            "[wvin]",
            "[amap]",
        )
    } else {
        (String::new(), "[0:a]", "0:a")
    };

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
    if (args.scale.is_some() || args.split)
        && matches!(
            args.mode,
            WaveMode::Spectrum
                | WaveMode::Scope
                | WaveMode::Cqt
                | WaveMode::Spectro
                | WaveMode::Spatial
                | WaveMode::Volume
                | WaveMode::Bitscope,
        )
    {
        return Err(Error::input(
            "--scale/--split apply to waveform modes (not spectrum/scope)",
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
                "{awave}showfreqs=s={{ww}}x{{wh}}:mode=bar{freq_rate}:colors={}{fs}[wv];",
                crate::color::lavfi(&args.color)
            ),
            "spectrum",
        ),
        WaveMode::Cqt => (
            format!("{awave}showcqt=s={{ww}}x{{wh}}:rate={fps}[wv];"),
            "cqt",
        ),
        // showspectrum stamps one frame per FFT window; overlay picks up the
        // background CFR timestamps, so no fps resample is needed.
        WaveMode::Spectro => (
            format!("{awave}showspectrum=s={{ww}}x{{wh}}:slide=scroll:scale=log[wv];"),
            "spectro",
        ),
        // aphasemeter outputs two pads — audio out0, video out1: label both
        // and map the audio pad (unconnected output pads stall the graph).
        WaveMode::Phase => (
            format!(
                "{awave}aphasemeter=size={{ww}}x{{wh}}:rate={fps}[pam][wvx];[wvx]format=rgb24[wv];"
            ),
            "phase",
        ),
        // showspatial stamps per-window like showspectrum — no rate arg.
        WaveMode::Spatial => (
            format!("{awave}showspatial=s={{ww}}x{{wh}}[wv];"),
            "spatial",
        ),
        // w is per-channel; stereo needs half the strip each so the pair
        // lands on the {ww}-wide box.
        WaveMode::Volume => (
            format!("{awave}showvolume=r={fps}:w={{vw}}:h={{wh}}:o=h[wv];"),
            "volume",
        ),
        WaveMode::Bitscope => (
            {
                let c = crate::color::lavfi(&args.color);
                format!(
                    "{awave}abitscope=s={{ww}}x{{wh}}:r={fps}:colors={c}|{c}|{c}|{c}|{c}|{c}|{c}|{c}[wv];"
                )
            },
            "bitscope",
        ),
        WaveMode::Scope => {
            let [r, g2, b] = crate::color::rgb(&args.color)?;
            (
                format!(
                    "{awave}avectorscope=s={{ww}}x{{wh}}:r={fps}:draw=line:zoom=2:rc={r}:gc={g2}:bc={b}[wv];"
                ),
                "scope",
            )
        }
        m => {
            let name = match m {
                WaveMode::Point => "point",
                WaveMode::Line => "line",
                WaveMode::P2p => "p2p",
                WaveMode::Cline => "cline",
                WaveMode::Spectrum
                | WaveMode::Scope
                | WaveMode::Cqt
                | WaveMode::Spectro
                | WaveMode::Phase
                | WaveMode::Spatial
                | WaveMode::Volume
                | WaveMode::Bitscope => unreachable!(),
            };
            (
                {
                    let sc = wave_scale(args)?;
                    let sp = if args.split { ":split_channels=1" } else { "" };
                    format!(
                        "{awave}showwaves=s={{ww}}x{{wh}}:mode={name}:rate={fps}:colors={}:draw=full{sc}{sp}[wv];",
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
            clip_dur.max(0.01)
        )),
        (true, false) => {
            tail.push_str("[mid];[mid][2:v]overlay=(W-w)/2:(H-h)*0.16:shortest=1[vout]")
        }
        (false, true) => tail.push_str(&format!(
            "[mid];[mid][{prog_idx}:v]overlay='(W-w)*t/{:.3}':H-h-6:shortest=1[vout]",
            clip_dur.max(0.01)
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
        "{range_fc}[1:v]scale={w}:{h}:force_original_aspect_ratio=increase,crop={w}:{h},setsar=1[bg];\
              {wave}\
              [wv]colorkey=0x000000:0.12:0.1[wvk];\
              [bg][wvk]overlay=(W-w)/2:(H-h)*{yf}:shortest=1{tail}",
        wave = wave_src
            .replace("{ww}", &((w as f64 * 0.87).round() as u32 & !1).to_string())
            .replace(
                "{wh}",
                &(((h as f64) / 6.0).round().max(40.0) as u32 & !1).to_string()
            )
            .replace(
                "{vw}",
                &(((w as f64 * 0.87).round() as u32 / 2).max(80) & !1).to_string()
            ),
        yf = yf,
    );
    let amap = if matches!(args.mode, WaveMode::Phase) {
        "[pam]"
    } else {
        amap
    };
    argv.extend(["-filter_complex", &fc, "-map", "[vout]", "-map", amap]);
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
    argv.push(output);

    let mut inputs: Vec<&Path> = vec![&args.input];
    if let Some(img) = &args.image {
        inputs.push(img);
    }
    if let Some(s) = &args.subs {
        inputs.push(s);
    }
    let mut c = engine::write_job("audiogram", &inputs, output, vec![argv], g)?;
    drop(prog_tmp);
    drop(sub_tmp);
    c = c.with_extra(json!({
        "frame": "1080x1920",
        "waveform": "showwaves",
        "mode": mode,
        "color": crate::color::lavfi(&args.color),
        "sub_cues": sub_cues.len(),
        "clip": { "from": from, "to": to },
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

fn derive_output(base: &Path, i: usize) -> std::path::PathBuf {
    let stem = base.file_stem().and_then(|s| s.to_str()).unwrap_or("clip");
    let ext = base.extension().and_then(|e| e.to_str()).unwrap_or("mp4");
    base.with_file_name(format!("{stem}_{i}.{ext}"))
}

use crate::cli::{ExtractArgs, Globals};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::time::{fmt_time, parse_time};

pub fn run(args: ExtractArgs, g: &Globals) -> Result<Contract, Error> {
    if args.bounce && !args.gif && !args.webp {
        return Err(Error::input("--bounce needs --gif or --webp"));
    }
    if args.loop_count.is_some() && !args.gif && !args.webp {
        return Err(Error::input("--loop needs --gif or --webp"));
    }
    if let Some(n) = args.loop_count {
        if !(-1..=100).contains(&n) {
            return Err(Error::input("--loop must be -1..=100 (-1 = play once)"));
        }
    }
    let ext = args
        .output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if args.alpha && !matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "webp") {
        return Err(Error::input("--alpha only applies to image output"));
    }
    if args.webp && args.gif {
        return Err(Error::input("--webp and --gif are exclusive"));
    }
    if args.lossless && !args.webp {
        return Err(Error::input("--lossless needs --webp"));
    }
    if args.webp && args.colors.is_some() {
        return Err(Error::input(
            "--colors is a GIF-palette knob — not for --webp",
        ));
    }
    if args.transparent && !args.gif && !args.webp {
        return Err(Error::input("--transparent needs --gif or --webp"));
    }
    if args.alpha || args.transparent {
        let probe = engine::probe_or_err(&args.input, g)?;
        let fmt = probe.pix_fmt.as_deref().unwrap_or("");
        let has_alpha = [
            "rgba", "argb", "bgra", "abgr", "yuva", "gbrap", "ya8", "ya16", "rgbaf", "bgraf",
            "rgba64", "bgra64", "vuya", "vuyx", "ayuv", "pal8",
        ]
        .iter()
        .any(|a| fmt.contains(a));
        if !has_alpha {
            return Err(Error::input(format!(
                "input has no alpha channel (pix_fmt {fmt})"
            )));
        }
    }

    let mut argv = ffmpeg_base(g.progress);

    // --audio: demux an audio track untouched (music/dialog rip —
    // no decode, no re-encode; -o extension picks the container,
    // --track picks commentary/stem in a multi-track file)
    if args.audio {
        let probe = engine::probe_or_err(&args.input, g)?;
        if !probe.has_audio {
            return Err(Error::input("extract --audio: input has no audio"));
        }
        let track = args.track.unwrap_or(0);
        argv.push("-i");
        argv.push(&args.input);
        let sel = format!("0:a:{track}");
        argv.extend(["-map", sel.as_str(), "-vn", "-c:a", "copy"]);
        argv.push(&args.output);
        let c = engine::write_job("extract", &[&args.input], &args.output, vec![argv], g)?;
        return Ok(c.with_extra(serde_json::json!({
            "audio": true,
            "acodec": probe.acodec,
            "track": track,
        })));
    }
    // --subs: pull an embedded subtitle track into a text container —
    // .srt/.ass/.vtt picked by -o; subtitle codecs differ so this is a
    // re-mux through the text encoders, not a raw copy
    if args.subs {
        let probe = engine::probe_or_err(&args.input, g)?;
        if probe.subtitle_streams == 0 {
            return Err(Error::input(
                "extract --subs: input has no subtitle streams",
            ));
        }
        let track = args.track.unwrap_or(0);
        if track >= probe.subtitle_streams {
            return Err(Error::input(format!(
                "extract --subs: input has {} subtitle stream(s)",
                probe.subtitle_streams
            )));
        }
        let codec = match ext.as_str() {
            "srt" => "srt",
            "ass" | "ssa" => "ass",
            "vtt" => "webvtt",
            _ => return Err(Error::input("extract --subs: -o must be .srt/.ass/.vtt")),
        };
        argv.push("-i");
        argv.push(&args.input);
        let sel = format!("0:s:{track}");
        argv.extend(["-map", sel.as_str(), "-vn", "-an", "-c:s", codec]);
        argv.push(&args.output);
        let c = engine::write_job("extract", &[&args.input], &args.output, vec![argv], g)?;
        return Ok(c.with_extra(serde_json::json!({
            "subs": true,
            "track": track,
        })));
    }
    if args.track.is_some() {
        return Err(Error::input("extract --track needs --audio or --subs"));
    }

    // comma --at on a still output: one frame per timepoint → `<stem>_N.<ext>`
    if !args.gif && !args.webp {
        if let Some(raw) = &args.at {
            if raw.split(',').count() > 1 {
                if !matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "webp") {
                    return Err(Error::input(
                        "comma --at needs a still output (.png/.jpg/.webp)",
                    ));
                }
                let probe = engine::probe_or_err(&args.input, g)?;
                engine::need_video(&probe, "extract")?;
                let mut files = Vec::new();
                for (i, part) in raw.split(',').enumerate() {
                    let secs = crate::time::resolve_frame_at(part.trim(), probe.duration)?;
                    let inp = args.input.display().to_string();
                    argv.extend(["-ss", &fmt_time(secs), "-i", inp.as_str()]);
                    files.push(derive_output(&args.output, i + 1));
                }
                for (i, f) in files.iter().enumerate() {
                    argv.extend(["-map", &format!("{i}:v"), "-frames:v", "1", "-q:v", "2"]);
                    if let Some(w) = args.width {
                        argv.extend(["-vf", &format!("scale={w}:-2")]);
                    }
                    argv.push(f.as_str());
                }
                let c = engine::write_job(
                    "extract",
                    &[&args.input],
                    std::path::Path::new(&files[0]),
                    vec![argv],
                    g,
                )?;
                let missing: Vec<_> = files
                    .iter()
                    .skip(1)
                    .filter(|f| !std::path::Path::new(f).exists())
                    .collect();
                if !missing.is_empty() {
                    return Err(Error::output(format!(
                        "extract: expected outputs missing: {}",
                        missing
                            .iter()
                            .map(|f| f.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    )));
                }
                return Ok(c.with_extra(serde_json::json!({ "files": files })));
            }
        }
    }

    // comma --at + --gif/--webp: one clip per timepoint → `<stem>_N.<ext>`
    if args.gif || args.webp {
        if let Some(raw) = &args.at {
            if raw.split(',').count() > 1 {
                let probe = engine::probe_or_err(&args.input, g)?;
                engine::need_video(&probe, "extract")?;
                let mut files = Vec::new();
                let mut first = None;
                for (i, part) in raw.split(',').enumerate() {
                    let secs = crate::time::resolve_frame_at(part.trim(), probe.duration)?;
                    let out = derive_output(&args.output, i + 1);
                    let c = if args.webp {
                        render_webp(&args, g, Some(secs), std::path::Path::new(&out))?
                    } else {
                        render_gif(&args, g, Some(secs), std::path::Path::new(&out))?
                    };
                    if i == 0 {
                        first = Some(c);
                    }
                    files.push(out);
                }
                return Ok(first
                    .unwrap()
                    .with_extra(serde_json::json!({ "files": files })));
            }
        }
    }

    // `--at end`: last frame (stills) or the last --dur seconds (--gif).
    let at_secs = match &args.at {
        Some(a) if a.trim().eq_ignore_ascii_case("end") => {
            let probe = engine::probe_or_err(&args.input, g)?;
            let back = args
                .dur
                .unwrap_or(if args.gif || args.webp { 1.0 } else { 0.05 });
            Some((probe.duration - back).max(0.0))
        }
        Some(a) => Some(parse_time(a)?),
        None => None,
    };

    match ext.as_str() {
        _ if args.gif => {
            return render_gif(&args, g, at_secs, &args.output);
        }
        _ if args.webp => {
            return render_webp(&args, g, at_secs, &args.output);
        }
        "png" | "jpg" | "jpeg" | "webp" => {
            if let Some(t) = at_secs {
                argv.extend(["-ss", &fmt_time(t)]);
            }
            argv.push("-i");
            argv.push(&args.input);
            let mut vf = String::new();
            if args.alpha {
                vf.push_str("alphaextract");
            }
            if let Some(w) = args.width {
                if !vf.is_empty() {
                    vf.push(',');
                }
                vf.push_str(&format!("scale={w}:-2"));
            }
            if !vf.is_empty() {
                argv.extend(["-vf", &vf]);
            }
            argv.extend(["-frames:v", "1", "-q:v", "2"]);
        }
        "wav" => {
            argv.push("-i");
            argv.push(&args.input);
            argv.extend(["-vn", "-acodec", "pcm_s16le"]);
        }
        "mp3" => {
            argv.push("-i");
            argv.push(&args.input);
            argv.extend(["-vn", "-c:a", "libmp3lame", "-q:a", "2"]);
        }
        "m4a" | "aac" => {
            argv.push("-i");
            argv.push(&args.input);
            argv.extend(["-vn", "-c:a", "aac", "-b:a", "192k"]);
        }
        "flac" => {
            argv.push("-i");
            argv.push(&args.input);
            argv.extend(["-vn", "-c:a", "flac"]);
        }
        "ogg" | "opus" => {
            argv.push("-i");
            argv.push(&args.input);
            argv.extend(["-vn", "-c:a", "libopus"]);
        }
        "srt" | "ass" | "vtt" => {
            argv.push("-i");
            argv.push(&args.input);
            argv.extend(["-map", "0:s:0"]);
        }
        other => {
            return Err(Error::input(format!(
                "extract: unknown output extension .{other}; use wav/mp3/m4a/png/srt"
            )));
        }
    }
    argv.push(&args.output);
    engine::write_job("extract", &[&args.input], &args.output, vec![argv], g)
}

fn derive_output(base: &std::path::Path, i: usize) -> String {
    let stem = base.file_stem().and_then(|s| s.to_str()).unwrap_or("shot");
    let ext = base.extension().and_then(|e| e.to_str()).unwrap_or("png");
    base.with_file_name(format!("{stem}_{i}.{ext}"))
        .to_string_lossy()
        .to_string()
}

fn render_gif(
    args: &ExtractArgs,
    g: &Globals,
    at_secs: Option<f64>,
    output: &std::path::Path,
) -> Result<Contract, Error> {
    // --bounce handled below in the paletteuse pass
    // 2-pass palette GIF from the window, like transcode --preset gif
    let mut gen = ffmpeg_base(g.progress);
    if let Some(at) = at_secs {
        gen.extend(["-ss", &fmt_time(at)]);
    }
    if let Some(d) = args.dur {
        gen.extend(["-t", &fmt_time(d)]);
    }
    gen.push("-i");
    gen.push(&args.input);
    let fps = args.fps.unwrap_or(10).clamp(1, 30);
    let w = args.width.unwrap_or(480).clamp(16, 1920);
    let scale = format!("fps={fps},scale={w}:-2:flags=lanczos");
    let palette = tempfile::Builder::new()
        .suffix(".png")
        .tempfile()
        .map_err(|e| Error::output(e.to_string()))?;
    let palette_path = palette.path().to_path_buf();
    let max_colors = args.colors.unwrap_or(256).clamp(2, 256);
    let palettegen = if args.transparent {
        // reserve_transparent parks index 255 for full-alpha pixels
        format!("{scale},palettegen=stats_mode=full:max_colors={max_colors}:reserve_transparent=1")
    } else {
        format!("{scale},palettegen=stats_mode=full:max_colors={max_colors}")
    };
    gen.extend(["-vf", palettegen.as_str()]);
    gen.push(&palette_path);

    let mut use_p = ffmpeg_base(g.progress);
    if let Some(at) = at_secs {
        use_p.extend(["-ss", &fmt_time(at)]);
    }
    if let Some(d) = args.dur {
        use_p.extend(["-t", &fmt_time(d)]);
    }
    use_p.push("-i");
    use_p.push(&args.input);
    use_p.push("-i");
    use_p.push(&palette_path);
    let seq = if args.bounce {
        format!("{scale},split[a][b];[b]reverse[r];[a][r]concat=n=2:v=1:a=0[x]")
    } else {
        format!("{scale}[x]")
    };
    let puse = if args.transparent {
        format!("{seq};[x][1:v]paletteuse=dither=bayer:alpha_threshold=128")
    } else {
        format!("{seq};[x][1:v]paletteuse=dither=bayer")
    };
    use_p.extend(["-lavfi", puse.as_str(), "-an"]);
    if let Some(n) = args.loop_count {
        use_p.extend(["-loop", &n.to_string()]);
    }
    use_p.push(output);
    let r = engine::write_job("extract", &[&args.input], output, vec![gen, use_p], g);
    drop(palette);
    r.map(|c| {
        if args.transparent {
            c.with_extra(serde_json::json!({ "transparent": true }))
        } else {
            c
        }
    })
}

fn render_webp(
    args: &ExtractArgs,
    g: &Globals,
    at_secs: Option<f64>,
    output: &std::path::Path,
) -> Result<Contract, Error> {
    // single-pass animated WebP — libwebp carries alpha natively (yuva420p),
    // unlike GIF it needs no palette round-trip
    let mut argv = ffmpeg_base(g.progress);
    if let Some(at) = at_secs {
        argv.extend(["-ss", &fmt_time(at)]);
    }
    if let Some(d) = args.dur {
        argv.extend(["-t", &fmt_time(d)]);
    }
    argv.push("-i");
    argv.push(&args.input);
    let fps = args.fps.unwrap_or(10).clamp(1, 30);
    let w = args.width.unwrap_or(480).clamp(16, 1920);
    let seq = if args.bounce {
        format!("fps={fps},scale={w}:-2:flags=lanczos,split[a][b];[b]reverse[r];[a][r]concat=n=2:v=1:a=0")
    } else {
        format!("fps={fps},scale={w}:-2:flags=lanczos")
    };
    argv.extend(["-lavfi", seq.as_str(), "-an"]);
    argv.extend(["-c:v", "libwebp", "-pix_fmt", "yuva420p"]);
    if args.lossless {
        argv.extend(["-lossless", "1"]);
    }
    if let Some(n) = args.loop_count {
        argv.extend(["-loop", &n.to_string()]);
    }
    argv.push(output);
    let r = engine::write_job("extract", &[&args.input], output, vec![argv], g);
    r.map(|c| c.with_extra(serde_json::json!({ "webp": true, "lossless": args.lossless })))
}

use serde_json::json;

use crate::cli::{Globals, TranscodeArgs, TranscodePreset};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::spawn::Argv;
pub fn run(args: TranscodeArgs, g: &Globals) -> Result<Contract, Error> {
    if args.gop.is_some() && args.copy_video {
        return Err(Error::input(
            "transcode --gop needs a re-encode — drop --copy-video",
        ));
    }
    if args.gop.is_some()
        && matches!(
            args.preset,
            Some(TranscodePreset::Mp3)
                | Some(TranscodePreset::Aac)
                | Some(TranscodePreset::Wav)
                | Some(TranscodePreset::Flac)
                | Some(TranscodePreset::Opus)
        )
    {
        return Err(Error::input(
            "transcode --gop is a video encode flag — audio presets have no keyframes",
        ));
    }
    let ext = args
        .output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let preset = args.preset.unwrap_or(match ext.as_str() {
        "gif" => TranscodePreset::Gif,
        "webm" => TranscodePreset::Webm,
        _ => TranscodePreset::H264,
    });

    let x264spec =
        args.profile.is_some() || args.level.is_some() || args.bf.is_some() || args.tune.is_some();
    if x264spec && args.copy_video {
        return Err(Error::input(
            "transcode --profile/--level/--bf/--tune need a re-encode — drop --copy-video",
        ));
    }
    if x264spec && !matches!(preset, TranscodePreset::H264 | TranscodePreset::Proxy) {
        return Err(Error::input(
            "transcode --profile/--level/--bf/--tune are x264 encode flags — h264/proxy presets only",
        ));
    }
    if args.alpha && !matches!(preset, TranscodePreset::Webm | TranscodePreset::Prores) {
        return Err(Error::input(
            "--alpha needs a webm or prores output (h264/hevc/av1 can't carry alpha)",
        ));
    }

    if args.range.is_some()
        && matches!(
            preset,
            TranscodePreset::Gif
                | TranscodePreset::Mp3
                | TranscodePreset::Aac
                | TranscodePreset::Wav
                | TranscodePreset::Flac
                | TranscodePreset::Opus
        )
    {
        return Err(Error::input("--range applies to video presets only"));
    }
    if args.field_order.is_some() && args.interlaced {
        return Err(Error::input(
            "--field-order relabels parity — --interlaced weaves real fields; pick one",
        ));
    }
    if args.field_order.is_some()
        && matches!(
            preset,
            TranscodePreset::Gif
                | TranscodePreset::Mp3
                | TranscodePreset::Aac
                | TranscodePreset::Wav
                | TranscodePreset::Flac
                | TranscodePreset::Opus
        )
    {
        return Err(Error::input("--field-order applies to video presets only"));
    }
    if args.vbitrate.is_some()
        && matches!(
            preset,
            TranscodePreset::Mp3
                | TranscodePreset::Aac
                | TranscodePreset::Wav
                | TranscodePreset::Flac
                | TranscodePreset::Opus
                | TranscodePreset::Gif
                | TranscodePreset::Prores
                | TranscodePreset::Dnxhd
        )
    {
        return Err(Error::input(
            "--vbitrate applies to video presets (h264/hevc/webm/av1)",
        ));
    }
    if args.copy_video {
        if matches!(preset, TranscodePreset::Gif)
            || matches!(
                preset,
                TranscodePreset::Mp3
                    | TranscodePreset::Aac
                    | TranscodePreset::Wav
                    | TranscodePreset::Flac
                    | TranscodePreset::Opus
            )
        {
            return Err(Error::input(
                "--copy-video applies to video presets (h264/hevc/webm/av1/prores/dnxhd/proxy)",
            ));
        }
        if args.fps.is_some()
            || args.range.is_some()
            || args.interlaced
            || args.field_order.is_some()
            || args.alpha
            || args.vbitrate.is_some()
        {
            return Err(Error::input(
                "--copy-video stream-copies the picture — drop --fps/--range/--interlaced/--field-order/--alpha/--vbitrate",
            ));
        }
    }

    match preset {
        TranscodePreset::Mp3
        | TranscodePreset::Aac
        | TranscodePreset::Wav
        | TranscodePreset::Flac
        | TranscodePreset::Opus => audio_only(&args, g, preset),
        TranscodePreset::Gif => gif(&args, g),
        TranscodePreset::H264 => h264(&args, g),
        TranscodePreset::Hevc => hevc(&args, g),
        TranscodePreset::Webm => webm(&args, g),
        TranscodePreset::Prores => prores(&args, g),
        TranscodePreset::Dnxhd => dnxhd(&args, g),
        TranscodePreset::Av1 => av1(&args, g),
        TranscodePreset::Proxy => proxy(&args, g),
        TranscodePreset::Ffv1 => ffv1(&args, g),
        TranscodePreset::Apng => apng(&args, g),
        TranscodePreset::Mpeg2 => mpeg2(&args, g),
        TranscodePreset::Mpeg1 => mpeg1(&args, g),
        TranscodePreset::Xvid => xvid(&args, g),
        TranscodePreset::Wmv => wmv(&args, g),
        TranscodePreset::Msmpeg4 => msmpeg4(&args, g),
        TranscodePreset::Gpp => gpp(&args, g),
        TranscodePreset::Flv => flv(&args, g),
        TranscodePreset::Theora => theora(&args, g),
    }
}

/// Edit proxy: capped at 540p high, veryfast x264 — smooth NLE scrubbing on
/// long takes / multicam dailies, never a delivery format.
fn proxy(args: &TranscodeArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_video {
        return Err(Error::input("proxy preset: input has no video"));
    }
    let crf = args.crf.unwrap_or(28);
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    if args.copy_video {
        argv.extend(["-c:v", "copy", "-movflags", "+faststart"]);
    } else {
        argv.extend([
            "-c:v",
            "libx264",
            "-preset",
            "veryfast",
            "-crf",
            &crf.to_string(),
            "-pix_fmt",
            "yuv420p",
            "-movflags",
            "+faststart",
        ]);
        x264spec_push(&mut argv, args);
        let mut vf = String::from("scale=w='min(960,iw)':h=-2");
        if let Some(fps) = args.fps {
            vf.push_str(&format!(",fps={fps}"));
        }
        vf.push_str(range_tag(args));
        vf.push_str(&interlace_tag(args));
        vf.push_str(field_tag(args));
        argv.extend(["-vf", &vf]);
    }
    if probe.has_audio {
        if args.copy_audio {
            argv.extend(["-c:a", "copy"]);
        } else {
            argv.extend(["-c:a", "aac", "-b:a", abitrate(args, "96k")]);
        }
    }
    cap_bitrate(&mut argv, &args.vbitrate);
    ar_ac(&mut argv, args);
    gop_push(&mut argv, args);
    argv.push(&args.output);
    let mut c = engine::write_job("transcode", &[&args.input], &args.output, vec![argv], g)?;
    c = c.with_extra(json!({"proxy": true, "copy_video": args.copy_video}));
    Ok(c)
}

fn range_tag(args: &TranscodeArgs) -> &'static str {
    match args.range {
        Some(crate::cli::TranscodeRange::Limited) => ",setparams=range=tv",
        Some(crate::cli::TranscodeRange::Full) => ",setparams=range=pc",
        None => "",
    }
}

fn field_tag(args: &TranscodeArgs) -> &'static str {
    // setparams only relabels the field_order flag — unlike setfield /
    // fieldorder it needs no flagged input, so it also FIXES wrong tags
    match args.field_order {
        Some(crate::cli::FieldOrder::Tff) => ",setparams=field_mode=tff",
        Some(crate::cli::FieldOrder::Bff) => ",setparams=field_mode=bff",
        Some(crate::cli::FieldOrder::Prog) => ",setparams=field_mode=prog",
        None => "",
    }
}

fn interlace_tag(args: &TranscodeArgs) -> String {
    // il interleaves fields (progressive → interlaced), setfield tags tff —
    // broadcast/air-master delivery specs want both the picture woven and
    // the container flag set. weave pairs real consecutive frames into
    // fields — true 2x-rate-to-interlace conversion (halves the fps).
    let mut s = String::new();
    if args.interlaced {
        s.push_str(match args.interlace_mode {
            // tinterlace interleave_top: consecutive frames → alternating
            // top/bottom fields — real temporal interlacing (60p→30i), NOT
            // ffmpeg's weave filter (that one stacks full frames vertically)
            Some(crate::cli::InterlaceKind::Weave) => {
                ",tinterlace=mode=interleave_top,setfield=tff"
            }
            _ => ",il=luma_mode=i:chroma_mode=i,setfield=tff",
        });
    }
    s
}

fn h264(args: &TranscodeArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    let crf = args.crf.unwrap_or(23);
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    if probe.has_video {
        if args.copy_video {
            argv.extend(["-c:v", "copy"]);
        } else {
            argv.extend([
                "-c:v",
                "libx264",
                "-preset",
                "medium",
                "-crf",
                &crf.to_string(),
                "-pix_fmt",
                "yuv420p",
                "-movflags",
                "+faststart",
            ]);
            x264spec_push(&mut argv, args);
            let mut vf = String::from("scale=trunc(iw/2)*2:trunc(ih/2)*2");
            if let Some(fps) = args.fps {
                vf.push_str(&format!(",fps={fps}"));
            }
            vf.push_str(range_tag(args));
            vf.push_str(&interlace_tag(args));
            vf.push_str(field_tag(args));
            argv.extend(["-vf", &vf]);
        }
    }
    if probe.has_audio {
        if args.copy_audio {
            argv.extend(["-c:a", "copy"]);
        } else {
            argv.extend(["-c:a", "aac", "-b:a", abitrate(args, "192k")]);
        }
    }
    cap_bitrate(&mut argv, &args.vbitrate);
    ar_ac(&mut argv, args);
    gop_push(&mut argv, args);
    argv.push(&args.output);
    engine::write_job("transcode", &[&args.input], &args.output, vec![argv], g)
}

fn hevc(args: &TranscodeArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    let crf = args.crf.unwrap_or(28);
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    if probe.has_video {
        if args.copy_video {
            argv.extend(["-c:v", "copy"]);
        } else {
            argv.extend([
                "-c:v",
                "libx265",
                "-preset",
                "medium",
                "-crf",
                &crf.to_string(),
                "-pix_fmt",
                "yuv420p",
                "-tag:v",
                "hvc1",
            ]);
            let mut vf = String::from("scale=trunc(iw/2)*2:trunc(ih/2)*2");
            if let Some(fps) = args.fps {
                vf.push_str(&format!(",fps={fps}"));
            }
            vf.push_str(range_tag(args));
            vf.push_str(&interlace_tag(args));
            vf.push_str(field_tag(args));
            argv.extend(["-vf", &vf]);
        }
    }
    if probe.has_audio {
        if args.copy_audio {
            argv.extend(["-c:a", "copy"]);
        } else {
            argv.extend(["-c:a", "aac", "-b:a", abitrate(args, "192k")]);
        }
    }
    cap_bitrate(&mut argv, &args.vbitrate);
    ar_ac(&mut argv, args);
    gop_push(&mut argv, args);
    argv.push(&args.output);
    engine::write_job("transcode", &[&args.input], &args.output, vec![argv], g)
}

fn webm(args: &TranscodeArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    let crf = args.crf.unwrap_or(32);
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    if probe.has_video {
        if args.copy_video {
            argv.extend(["-c:v", "copy"]);
        } else {
            argv.extend([
                "-c:v",
                "libvpx-vp9",
                "-b:v",
                "0",
                "-crf",
                &crf.to_string(),
                "-pix_fmt",
                if args.alpha { "yuva420p" } else { "yuv420p" },
            ]);
            let mut vf = String::from("scale=trunc(iw/2)*2:trunc(ih/2)*2");
            if let Some(fps) = args.fps {
                vf.push_str(&format!(",fps={fps}"));
            }
            vf.push_str(range_tag(args));
            vf.push_str(&interlace_tag(args));
            vf.push_str(field_tag(args));
            argv.extend(["-vf", &vf]);
        }
    }
    if probe.has_audio {
        if args.copy_audio {
            argv.extend(["-c:a", "copy"]);
        } else {
            argv.extend(["-c:a", "libopus", "-b:a", abitrate(args, "128k")]);
        }
    }
    cap_bitrate(&mut argv, &args.vbitrate);
    ar_ac(&mut argv, args);
    gop_push(&mut argv, args);
    argv.push(&args.output);
    engine::write_job("transcode", &[&args.input], &args.output, vec![argv], g)
}

/// AV1 delivery: libsvtav1 on ffmpeg ≥7 (fast), libaom on 4.x (row-mt +
/// cpu-used for usable speed). YouTube/web prefers AV1 for small files.
fn av1(args: &TranscodeArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    let crf = args.crf.unwrap_or(35).to_string();
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    if probe.has_video {
        if args.copy_video {
            argv.extend(["-c:v", "copy"]);
        } else {
            if crate::engine::ffmpeg_major().unwrap_or(7) >= 7 {
                argv.extend(["-c:v", "libsvtav1", "-preset", "6"]);
            } else {
                argv.extend(["-c:v", "libaom-av1", "-cpu-used", "4", "-row-mt", "1"]);
            }
            argv.extend(["-crf", &crf, "-b:v", "0", "-pix_fmt", "yuv420p"]);
            let mut vf = String::from("scale=trunc(iw/2)*2:trunc(ih/2)*2");
            if let Some(fps) = args.fps {
                vf.push_str(&format!(",fps={fps}"));
            }
            vf.push_str(range_tag(args));
            vf.push_str(&interlace_tag(args));
            vf.push_str(field_tag(args));
            argv.extend(["-vf", &vf]);
        }
    }
    if probe.has_audio {
        if args.copy_audio {
            argv.extend(["-c:a", "copy"]);
        } else {
            argv.extend(["-c:a", "libopus", "-b:a", abitrate(args, "128k")]);
        }
    }
    cap_bitrate(&mut argv, &args.vbitrate);
    ar_ac(&mut argv, args);
    gop_push(&mut argv, args);
    argv.push(&args.output);
    engine::write_job("transcode", &[&args.input], &args.output, vec![argv], g)
}

fn gif(args: &TranscodeArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "transcode")?;
    let palette = tempfile::Builder::new()
        .suffix(".png")
        .tempfile()
        .map_err(|e| Error::output(e.to_string()))?;
    let palette_path = palette.path().to_path_buf();

    let fps = args.fps.unwrap_or(10).clamp(1, 30);
    let width = args.width.unwrap_or(480).clamp(16, 1920);
    let scale = format!("fps={fps},scale={width}:-2:flags=lanczos");
    let mut gen = ffmpeg_base(g.progress);
    gen.push("-i");
    gen.push(&args.input);
    let max_colors = args.colors.unwrap_or(256).clamp(2, 256);
    gen.extend([
        "-vf",
        &format!("{scale},palettegen=stats_mode=full:max_colors={max_colors}"),
    ]);
    gen.push(&palette_path);

    let mut use_p = ffmpeg_base(g.progress);
    use_p.push("-i");
    use_p.push(&args.input);
    use_p.push("-i");
    use_p.push(&palette_path);
    use_p.extend([
        "-lavfi",
        &format!("{scale}[x];[x][1:v]paletteuse=dither=bayer"),
        "-an",
    ]);
    use_p.push(&args.output);

    let result = engine::write_job(
        "transcode",
        &[&args.input],
        &args.output,
        vec![gen, use_p],
        g,
    );
    drop(palette);
    result
}

/// Audio-only delivery: -vn + one codec. `--copy-audio` stream-copies instead
/// of re-encoding (e.g. mp4 → mp3 keeps nothing — copy only fits same-codec).
fn audio_only(
    args: &TranscodeArgs,
    g: &Globals,
    preset: TranscodePreset,
) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_audio {
        return Err(Error::input(
            "audio preset (mp3/aac/wav/flac/opus): input has no audio",
        ));
    }
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.push("-vn");
    if args.copy_audio {
        argv.extend(["-c:a", "copy"]);
    } else {
        match preset {
            TranscodePreset::Mp3 => {
                argv.extend(["-c:a", "libmp3lame", "-b:a", abitrate(args, "192k")])
            }
            TranscodePreset::Wav => argv.extend(["-c:a", "pcm_s16le"]),
            TranscodePreset::Flac => argv.extend(["-c:a", "flac"]),
            TranscodePreset::Opus => {
                argv.extend(["-c:a", "libopus", "-b:a", abitrate(args, "128k")])
            }
            _ => argv.extend(["-c:a", "aac", "-b:a", abitrate(args, "192k")]),
        }
    }
    ar_ac(&mut argv, args);
    gop_push(&mut argv, args);
    argv.push(&args.output);
    let c = engine::write_job("transcode", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({"audio_only": true})))
}

fn prores(args: &TranscodeArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    let ext = args
        .output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if ext != "mov" {
        return Err(Error::input(
            "prores preset wants a .mov output (ProRes + PCM in MOV)",
        ));
    }
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    if probe.has_video {
        if args.copy_video {
            argv.extend(["-c:v", "copy"]);
        } else {
            if args.alpha {
                argv.extend([
                    "-c:v",
                    "prores_ks",
                    "-profile:v",
                    "4",
                    "-pix_fmt",
                    "yuva444p10le",
                    "-vendor",
                    "apl0",
                ]);
            } else {
                argv.extend([
                    "-c:v",
                    "prores_ks",
                    "-profile:v",
                    "3",
                    "-pix_fmt",
                    "yuv422p10le",
                ]);
            }
            let fps_vf = args.fps.map(|fps| format!("fps={fps}")).unwrap_or_default();
            let prores_vf = format!(
                "{fps_vf}{}{}{}",
                range_tag(args),
                interlace_tag(args),
                field_tag(args)
            );
            if !prores_vf.is_empty() {
                argv.extend(["-vf", prores_vf.trim_start_matches(',')]);
            }
        }
    }
    if probe.has_audio {
        if args.copy_audio {
            argv.extend(["-c:a", "copy"]);
        } else {
            argv.extend(["-c:a", "pcm_s16le"]);
        }
    }
    cap_bitrate(&mut argv, &args.vbitrate);
    ar_ac(&mut argv, args);
    gop_push(&mut argv, args);
    argv.push(&args.output);
    engine::write_job("transcode", &[&args.input], &args.output, vec![argv], g)
}

/// DNxHR HQ in .mov — the Avid/Resolve-side edit handoff (ProRes on the
/// Apple side). dnxhr profiles take any resolution; pcm_s16le like prores.
fn dnxhd(args: &TranscodeArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    let ext = args
        .output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if ext != "mov" {
        return Err(Error::input(
            "dnxhd preset wants a .mov output (DNxHR + PCM in MOV)",
        ));
    }
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    if probe.has_video {
        if args.copy_video {
            argv.extend(["-c:v", "copy"]);
        } else {
            argv.extend([
                "-c:v",
                "dnxhd",
                "-profile:v",
                "dnxhr_hq",
                "-pix_fmt",
                "yuv422p",
            ]);
            let mut vf = String::from("scale=trunc(iw/2)*2:trunc(ih/2)*2");
            if let Some(fps) = args.fps {
                vf.push_str(&format!(",fps={fps}"));
            }
            vf.push_str(range_tag(args));
            vf.push_str(&interlace_tag(args));
            vf.push_str(field_tag(args));
            argv.extend(["-vf", &vf]);
        }
    }
    if probe.has_audio {
        if args.copy_audio {
            argv.extend(["-c:a", "copy"]);
        } else {
            argv.extend(["-c:a", "pcm_s16le"]);
        }
    }
    ar_ac(&mut argv, args);
    gop_push(&mut argv, args);
    argv.push(&args.output);
    engine::write_job("transcode", &[&args.input], &args.output, vec![argv], g)
}

fn abitrate<'a>(args: &'a TranscodeArgs, default: &'a str) -> &'a str {
    args.abitrate.as_deref().unwrap_or(default)
}

// --ar/--channels apply only on a real re-encode; a stream copy has no
// encoder to resample/remap
fn ar_ac(argv: &mut Argv, args: &TranscodeArgs) {
    if args.copy_audio {
        return;
    }
    if let Some(ar) = args.ar {
        argv.extend(["-ar", &ar.to_string()]);
    }
    if let Some(ch) = args.channels {
        argv.extend(["-ac", &ch.to_string()]);
    }
}

/// FFV1 lossless archival master — mathematically lossless video + flac
/// audio in .mkv (the museum/NLE-safe intermediate; only mkv carries ffv1).
fn ffv1(args: &TranscodeArgs, g: &Globals) -> Result<Contract, Error> {
    let ext = args
        .output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if ext != "mkv" {
        return Err(Error::input(format!(
            "transcode --preset ffv1 needs a .mkv target (only mkv carries ffv1), not .{ext}"
        )));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_video {
        return Err(Error::input("ffv1 preset: input has no video"));
    }
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-map", "0:v?"]);
    if probe.has_audio {
        argv.extend(["-map", "0:a?"]);
    }
    argv.extend(["-c:v", "ffv1"]);
    if let Some(n) = args.gop {
        argv.extend(["-g", &n.to_string()]);
    }
    if probe.has_audio {
        argv.extend(["-c:a", "flac"]);
    }
    if let Some(fps) = args.fps {
        argv.extend(["-r", &fps.to_string()]);
    }
    argv.push(&args.output);
    let mut c = engine::write_job("transcode", &[&args.input], &args.output, vec![argv], g)?;
    c = c.with_extra(json!({ "preset": "ffv1" }));
    Ok(c)
}

/// MPEG-2 + MP2 — DVD/broadcast legacy master (.mpg/.mpeg/.vob): set-top
/// players, TV ingest, archival interop with decades-old systems.
fn mpeg2(args: &TranscodeArgs, g: &Globals) -> Result<Contract, Error> {
    let ext = args
        .output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if !matches!(ext.as_str(), "mpg" | "mpeg" | "vob") {
        return Err(Error::input(format!(
            "transcode --preset mpeg2 needs a .mpg/.mpeg/.vob target, not .{ext}"
        )));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_video {
        return Err(Error::input("mpeg2 preset: input has no video"));
    }
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-map", "0:v?"]);
    if probe.has_audio {
        argv.extend(["-map", "0:a?"]);
    }
    argv.extend(["-c:v", "mpeg2video"]);
    if let Some(n) = args.gop {
        argv.extend(["-g", &n.to_string()]);
    }
    if probe.has_audio {
        argv.extend(["-c:a", "mp2"]);
    }
    if let Some(fps) = args.fps {
        argv.extend(["-r", &fps.to_string()]);
    }
    argv.push(&args.output);
    let mut c = engine::write_job("transcode", &[&args.input], &args.output, vec![argv], g)?;
    c = c.with_extra(json!({ "preset": "mpeg2" }));
    Ok(c)
}

/// MPEG-1 + MP2 — VCD-era legacy master (.mpg): the oldest digital video
/// format still in playback circulation, max interop with ancient gear.
fn mpeg1(args: &TranscodeArgs, g: &Globals) -> Result<Contract, Error> {
    let ext = args
        .output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if !matches!(ext.as_str(), "mpg" | "mpeg") {
        return Err(Error::input(format!(
            "transcode --preset mpeg1 needs a .mpg/.mpeg target, not .{ext}"
        )));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_video {
        return Err(Error::input("mpeg1 preset: input has no video"));
    }
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-map", "0:v?"]);
    if probe.has_audio {
        argv.extend(["-map", "0:a?"]);
    }
    argv.extend(["-c:v", "mpeg1video"]);
    if let Some(n) = args.gop {
        argv.extend(["-g", &n.to_string()]);
    }
    if probe.has_audio {
        argv.extend(["-c:a", "mp2"]);
    }
    if let Some(fps) = args.fps {
        argv.extend(["-r", &fps.to_string()]);
    }
    argv.push(&args.output);
    let mut c = engine::write_job("transcode", &[&args.input], &args.output, vec![argv], g)?;
    c = c.with_extra(json!({ "preset": "mpeg1" }));
    Ok(c)
}

/// Xvid/MPEG-4 part 2 + MP3 in .avi — the legacy-rip master: DivX-era
/// players, projectors, and set-top boxes that only read .avi containers.
fn xvid(args: &TranscodeArgs, g: &Globals) -> Result<Contract, Error> {
    let ext = args
        .output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if ext != "avi" {
        return Err(Error::input(format!(
            "transcode --preset xvid needs a .avi target, not .{ext}"
        )));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_video {
        return Err(Error::input("xvid preset: input has no video"));
    }
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-map", "0:v?"]);
    if probe.has_audio {
        argv.extend(["-map", "0:a?"]);
    }
    argv.extend(["-c:v", "libxvid", "-vtag", "xvid"]);
    if let Some(n) = args.gop {
        argv.extend(["-g", &n.to_string()]);
    }
    if probe.has_audio {
        argv.extend(["-c:a", "libmp3lame"]);
    }
    if let Some(fps) = args.fps {
        argv.extend(["-r", &fps.to_string()]);
    }
    argv.push(&args.output);
    let mut c = engine::write_job("transcode", &[&args.input], &args.output, vec![argv], g)?;
    c = c.with_extra(json!({ "preset": "xvid" }));
    Ok(c)
}

/// MS-MPEG4 v2 + MP3 in .avi — the pre-DivX Windows codec (MP42 tag):
/// Windows ME-era screen captures and players older than the Xvid era.
/// H.263 + AMR-NB in .3gp — the feature-phone master: MMS-era mobile
/// video and J2ME handsets. AMR-NB speech audio is 8kHz mono — anything
/// else fails the codec's own constraint, so it's forced not probed.
/// FLV1 + MP3 in .flv — the Flash-era web master: YouTube 2005-era uploads,
/// Flash video archives, players that still only speak .flv.
fn flv(args: &TranscodeArgs, g: &Globals) -> Result<Contract, Error> {
    let ext = args
        .output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if ext != "flv" {
        return Err(Error::input(format!(
            "transcode --preset flv needs a .flv target, not .{ext}"
        )));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_video {
        return Err(Error::input("flv preset: input has no video"));
    }
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-map", "0:v?"]);
    if probe.has_audio {
        argv.extend(["-map", "0:a?"]);
    }
    argv.extend(["-c:v", "flv"]);
    if let Some(n) = args.gop {
        argv.extend(["-g", &n.to_string()]);
    }
    if probe.has_audio {
        argv.extend(["-c:a", "libmp3lame"]);
    }
    if let Some(fps) = args.fps {
        argv.extend(["-r", &fps.to_string()]);
    }
    argv.push(&args.output);
    let mut c = engine::write_job("transcode", &[&args.input], &args.output, vec![argv], g)?;
    c = c.with_extra(json!({ "preset": "flv" }));
    Ok(c)
}

/// Theora + Vorbis in .ogv — the open-web master: pre-WebM HTML5 video,
/// Wikipedia/Wikimedia embeds, FLOSS-only playback chains.
fn theora(args: &TranscodeArgs, g: &Globals) -> Result<Contract, Error> {
    let ext = args
        .output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if !matches!(ext.as_str(), "ogv" | "ogg") {
        return Err(Error::input(format!(
            "transcode --preset theora needs a .ogv/.ogg target, not .{ext}"
        )));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_video {
        return Err(Error::input("theora preset: input has no video"));
    }
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-map", "0:v?"]);
    if probe.has_audio {
        argv.extend(["-map", "0:a?"]);
    }
    argv.extend(["-c:v", "libtheora", "-q:v", "5"]);
    if let Some(n) = args.gop {
        argv.extend(["-g", &n.to_string()]);
    }
    if probe.has_audio {
        argv.extend(["-c:a", "libvorbis"]);
    }
    if let Some(fps) = args.fps {
        argv.extend(["-r", &fps.to_string()]);
    }
    argv.push(&args.output);
    let mut c = engine::write_job("transcode", &[&args.input], &args.output, vec![argv], g)?;
    c = c.with_extra(json!({ "preset": "theora" }));
    Ok(c)
}

fn gpp(args: &TranscodeArgs, g: &Globals) -> Result<Contract, Error> {
    let ext = args
        .output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if ext != "3gp" && ext != "3g2" {
        return Err(Error::input(format!(
            "transcode --preset gpp needs a .3gp/.3g2 target, not .{ext}"
        )));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_video {
        return Err(Error::input("gpp preset: input has no video"));
    }
    // H.263 encodes only five fixed picture sizes — snap to the nearest
    // legal canvas, letterboxed so the frame never distorts
    const LEGAL: [(u32, u32); 5] = [(128, 96), (176, 144), (352, 288), (704, 576), (1408, 1152)];
    let area = (probe.width.unwrap_or(176) as u64) * (probe.height.unwrap_or(144) as u64);
    let (w, h) = LEGAL
        .iter()
        .min_by_key(|(lw, lh)| ((lw * lh) as u64).abs_diff(area))
        .copied()
        .unwrap();
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-map", "0:v?"]);
    if probe.has_audio {
        argv.extend(["-map", "0:a?"]);
    }
    argv.extend([
        "-vf",
        &format!("scale=w={w}:h={h}:force_original_aspect_ratio=decrease,pad={w}:{h}:(ow-iw)/2:(oh-ih)/2"),
    ]);
    argv.extend(["-c:v", "h263"]);
    if let Some(n) = args.gop {
        argv.extend(["-g", &n.to_string()]);
    }
    if probe.has_audio {
        argv.extend(["-c:a", "libopencore_amrnb", "-ar", "8000", "-ac", "1"]);
    }
    if let Some(fps) = args.fps {
        argv.extend(["-r", &fps.to_string()]);
    }
    argv.push(&args.output);
    let mut c = engine::write_job("transcode", &[&args.input], &args.output, vec![argv], g)?;
    c = c.with_extra(json!({ "preset": "gpp" }));
    Ok(c)
}

fn msmpeg4(args: &TranscodeArgs, g: &Globals) -> Result<Contract, Error> {
    let ext = args
        .output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if ext != "avi" {
        return Err(Error::input(format!(
            "transcode --preset msmpeg4 needs a .avi target, not .{ext}"
        )));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_video {
        return Err(Error::input("msmpeg4 preset: input has no video"));
    }
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-map", "0:v?"]);
    if probe.has_audio {
        argv.extend(["-map", "0:a?"]);
    }
    // -vtag mp42: legacy players fourcc-check for the MS-MPEG4 v2 tag —
    // an unlabeled stream decodes fine but gets refused by the players
    // this preset exists for
    argv.extend(["-c:v", "msmpeg4v2", "-vtag", "mp42"]);
    if let Some(n) = args.gop {
        argv.extend(["-g", &n.to_string()]);
    }
    if probe.has_audio {
        argv.extend(["-c:a", "libmp3lame"]);
    }
    if let Some(fps) = args.fps {
        argv.extend(["-r", &fps.to_string()]);
    }
    argv.push(&args.output);
    let mut c = engine::write_job("transcode", &[&args.input], &args.output, vec![argv], g)?;
    c = c.with_extra(json!({ "preset": "msmpeg4" }));
    Ok(c)
}

/// WMV2 + WMA in .wmv/.asf — Windows Media-era master: corporate training
/// archives, old PowerPoint-embedded video, Windows-only playback gear.
fn wmv(args: &TranscodeArgs, g: &Globals) -> Result<Contract, Error> {
    let ext = args
        .output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if !matches!(ext.as_str(), "wmv" | "asf") {
        return Err(Error::input(format!(
            "transcode --preset wmv needs a .wmv/.asf target, not .{ext}"
        )));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_video {
        return Err(Error::input("wmv preset: input has no video"));
    }
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-map", "0:v?"]);
    if probe.has_audio {
        argv.extend(["-map", "0:a?"]);
    }
    argv.extend(["-c:v", "wmv2"]);
    if let Some(n) = args.gop {
        argv.extend(["-g", &n.to_string()]);
    }
    if probe.has_audio {
        argv.extend(["-c:a", "wmav2"]);
    }
    if let Some(fps) = args.fps {
        argv.extend(["-r", &fps.to_string()]);
    }
    argv.push(&args.output);
    let mut c = engine::write_job("transcode", &[&args.input], &args.output, vec![argv], g)?;
    c = c.with_extra(json!({ "preset": "wmv" }));
    Ok(c)
}

/// Animated PNG — full-color looping stickers/reactions where gif's 256
/// colors band (apng plays everywhere gif does; loops forever).
fn apng(args: &TranscodeArgs, g: &Globals) -> Result<Contract, Error> {
    let ext = args
        .output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if !matches!(ext.as_str(), "apng" | "png") {
        return Err(Error::input(format!(
            "transcode --preset apng needs a .apng/.png target, not .{ext}"
        )));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_video {
        return Err(Error::input("apng preset: input has no video"));
    }
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-map", "0:v?", "-f", "apng", "-plays", "0"]);
    if let Some(fps) = args.fps {
        argv.extend(["-vf", &format!("fps={fps}")]);
    }
    argv.push(&args.output);
    let mut c = engine::write_job("transcode", &[&args.input], &args.output, vec![argv], g)?;
    c = c.with_extra(json!({ "preset": "apng", "loop": "forever" }));
    Ok(c)
}

fn x264spec_push(argv: &mut Argv, args: &TranscodeArgs) {
    if let Some(p) = args.profile {
        argv.extend(["-profile:v", transcode_profile_name(p)]);
    }
    if let Some(l) = &args.level {
        argv.extend(["-level:v", l]);
    }
    if let Some(n) = args.bf {
        argv.extend(["-bf", &n.to_string()]);
    }
    if let Some(t) = args.tune {
        argv.extend(["-tune", transcode_tune_name(t)]);
    }
}

fn transcode_tune_name(t: crate::cli::TranscodeTune) -> &'static str {
    match t {
        crate::cli::TranscodeTune::Film => "film",
        crate::cli::TranscodeTune::Animation => "animation",
        crate::cli::TranscodeTune::Grain => "grain",
        crate::cli::TranscodeTune::Zerolatency => "zerolatency",
        crate::cli::TranscodeTune::Fastdecode => "fastdecode",
        crate::cli::TranscodeTune::Stillimage => "stillimage",
        crate::cli::TranscodeTune::Psnr => "psnr",
        crate::cli::TranscodeTune::Ssim => "ssim",
    }
}

pub(crate) fn transcode_profile_name(p: crate::cli::TranscodeProfile) -> &'static str {
    match p {
        crate::cli::TranscodeProfile::Baseline => "baseline",
        crate::cli::TranscodeProfile::Main => "main",
        crate::cli::TranscodeProfile::High => "high",
    }
}

fn gop_push(argv: &mut Argv, args: &TranscodeArgs) {
    if let Some(n) = args.gop {
        argv.extend(["-g", &n.to_string()]);
    }
}

fn cap_bitrate(argv: &mut crate::spawn::Argv, rate: &Option<String>) {
    let Some(r) = rate else { return };
    argv.extend(["-maxrate", r.as_str(), "-bufsize", &double_rate(r)]);
}

fn double_rate(r: &str) -> String {
    let (num, suf) = match r.chars().last() {
        Some(c) if c.is_ascii_alphabetic() => (&r[..r.len() - 1], &r[r.len() - 1..]),
        _ => (r, ""),
    };
    let n: f64 = num.parse().unwrap_or(0.0);
    format!("{}{}", (n * 2.0).round() as i64, suf)
}

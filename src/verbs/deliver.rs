use serde_json::json;

use crate::cli::{DeliverArgs, DeliverPlatform, Globals, LogoPos};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::spawn::{self, Argv};
use crate::verbs::loudnorm;

const TARGET_I: f64 = -14.0;
const TARGET_TP: f64 = -1.5;
const TARGET_LRA: f64 = 11.0;
const PODCAST_I: f64 = -16.0;

pub fn run(args: DeliverArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    if args.logo.is_none() && (args.logo_position.is_some() || args.logo_opacity.is_some()) {
        return Err(Error::input("--logo-position/--logo-opacity need --logo"));
    }
    if let Some(op) = args.logo_opacity {
        if !(0.0..=1.0).contains(&op) {
            return Err(Error::input("--logo-opacity must be 0..=1"));
        }
    }
    let audio_pack = matches!(
        args.platform,
        DeliverPlatform::Podcast | DeliverPlatform::Audiobook
    );
    if args.cover.is_some() && !audio_pack {
        return Err(Error::input(
            "deliver --cover only applies to --platform podcast/audiobook (feed art)",
        ));
    }
    if args.chapters.is_some() && !audio_pack {
        return Err(Error::input(
            "deliver --chapters only applies to --platform podcast/audiobook",
        ));
    }
    if audio_pack {
        if args.logo.is_some() || args.intro.is_some() || args.outro.is_some() {
            return Err(Error::input(
                "deliver --logo/--intro/--outro need a video platform — audio packs have no picture",
            ));
        }
        return podcast(args, &probe, g);
    }
    if args.to.is_some() {
        let scheme = args
            .to
            .as_deref()
            .unwrap_or("")
            .split("://")
            .next()
            .unwrap_or("")
            .to_lowercase();
        if !matches!(scheme.as_str(), "rtmp" | "rtmps" | "tcp" | "udp") {
            return Err(Error::input(
                "deliver --to needs an rtmp://, rtmps://, tcp://, or udp:// URL",
            ));
        }
    }
    engine::need_video(&probe, "deliver")?;
    let wrap = args.intro.is_some() || args.outro.is_some();
    if wrap {
        for clip in [args.intro.as_ref(), args.outro.as_ref()]
            .into_iter()
            .flatten()
        {
            crate::paths::ensure_input(clip)?;
            let cp = engine::probe_or_err(clip, g)?;
            engine::need_video(&cp, "deliver --intro/--outro")?;
            if probe.has_audio && !cp.has_audio {
                return Err(Error::input(
                    "deliver --intro/--outro clips need an audio track to match the main audio",
                ));
            }
        }
    }

    let (fw, fh) = match args.platform {
        DeliverPlatform::Youtube | DeliverPlatform::Bilibili | DeliverPlatform::Linkedin => {
            (1920, 1080)
        }
        DeliverPlatform::X => (1280, 720),
        DeliverPlatform::Square => (1080, 1080),
        DeliverPlatform::Xhs => (1080, 1440),
        DeliverPlatform::Wechat => (1080, 1260),
        DeliverPlatform::Pinterest => (1000, 1500),
        _ => (1080, 1920),
    };
    let mut vf = format!(
        "scale={fw}:{fh}:force_original_aspect_ratio=decrease,pad={fw}:{fh}:(ow-iw)/2:(oh-ih)/2:black,setsar=1,fps={fps}",
        fps = args.fps.unwrap_or(30)
    );
    let mut subs_part: Option<String> = None;
    if let Some(subs) = &args.subs {
        // The subtitles filter parses `:` `'` `,` in filenames — escape them.
        let path = subs
            .canonicalize()
            .map_err(|e| Error::input(format!("{subs:?}: {e}")))?
            .to_string_lossy()
            .replace('\\', "\\\\")
            .replace('\'', "\\'");
        subs_part = Some(format!(",subtitles=filename='{path}'"));
        vf.push_str(subs_part.as_deref().unwrap());
    }
    vf.push_str(",format=yuv420p");
    let platform = platform_name(args.platform);

    let mut apply = ffmpeg_base(g.progress);
    apply.push("-i");
    apply.push(&args.input);
    let mut ni = 1u32;
    let intro_i = if let Some(p) = &args.intro {
        apply.push("-i");
        apply.push(p);
        let i = ni;
        ni += 1;
        Some(i)
    } else {
        None
    };
    let outro_i = if let Some(p) = &args.outro {
        apply.push("-i");
        apply.push(p);
        let i = ni;
        ni += 1;
        Some(i)
    } else {
        None
    };
    let logo_i = if let Some(logo) = &args.logo {
        crate::paths::ensure_input(logo)?;
        apply.push("-i");
        apply.push(logo);
        let i = ni;
        Some(i)
    } else {
        None
    };
    let logo_fg = |li: u32| -> String {
        let lw = (fw * 18 / 100).max(16);
        let m = (fh * 3 / 100).max(8);
        let (x, y) = match args.logo_position.unwrap_or(LogoPos::Br) {
            LogoPos::Tl => (format!("{m}"), format!("{m}")),
            LogoPos::Tr => (format!("W-w-{m}"), format!("{m}")),
            LogoPos::Bl => (format!("{m}"), format!("H-h-{m}")),
            LogoPos::Br => (format!("W-w-{m}"), format!("H-h-{m}")),
        };
        let mut lg = format!("[{li}:v]scale={lw}:-1");
        if let Some(op) = args.logo_opacity {
            lg.push_str(&format!(",format=rgba,colorchannelmixer=aa={op}"));
        }
        lg.push_str(&format!("[lg];[vc][lg]overlay={x}:{y}[vo]"));
        lg
    };
    // wrap: fc held aside so loudnorm folds into the graph — -af conflicts
    // with a complex-feed stream.
    let mut wrap_fc: Option<String> = None;
    let mut wrap_vout = "vc";
    if wrap {
        // Brand wrap: every segment normalized to the platform canvas, then
        // concat — subs/logo ride the whole deliverable (logo on top of all
        // three, captions on the main segment only).
        let segs: Vec<u32> = intro_i.into_iter().chain([0]).chain(outro_i).collect();
        let mut fc = String::new();
        for (k, i) in segs.iter().enumerate() {
            let mut chain = format!(
                "scale={fw}:{fh}:force_original_aspect_ratio=decrease,pad={fw}:{fh}:(ow-iw)/2:(oh-ih)/2:black,setsar=1,fps={fps}",
                fps = args.fps.unwrap_or(30)
            );
            if *i == 0 {
                if let Some(sp) = &subs_part {
                    chain.push_str(sp);
                }
            }
            chain.push_str(",format=yuv420p");
            fc.push_str(&format!("[{i}:v]{chain}[v{k}];"));
            if probe.has_audio {
                fc.push_str(&format!(
                    "[{i}:a]aresample=48000,aformat=channel_layouts=stereo[a{k}];"
                ));
            }
        }
        let ins: String = (0..segs.len())
            .map(|k| {
                if probe.has_audio {
                    format!("[v{k}][a{k}]")
                } else {
                    format!("[v{k}]")
                }
            })
            .collect();
        if probe.has_audio {
            fc.push_str(&format!("{ins}concat=n={}:v=1:a=1[vc][ac];", segs.len()));
        } else {
            fc.push_str(&format!("{ins}concat=n={}:v=1:a=0[vc];", segs.len()));
        }
        if let Some(li) = logo_i {
            fc.push_str(&logo_fg(li));
            wrap_vout = "vo";
        }
        wrap_fc = Some(fc);
    } else if let Some(li) = logo_i {
        let lw = (fw * 18 / 100).max(16);
        let m = (fh * 3 / 100).max(8);
        let (x, y) = match args.logo_position.unwrap_or(LogoPos::Br) {
            LogoPos::Tl => (format!("{m}"), format!("{m}")),
            LogoPos::Tr => (format!("W-w-{m}"), format!("{m}")),
            LogoPos::Bl => (format!("{m}"), format!("H-h-{m}")),
            LogoPos::Br => (format!("W-w-{m}"), format!("H-h-{m}")),
        };
        let mut lg = format!("scale={lw}:-1");
        if let Some(op) = args.logo_opacity {
            lg.push_str(&format!(",format=rgba,colorchannelmixer=aa={op}"));
        }
        let fc = format!("[0:v]{vf}[base];[{li}:v]{lg}[lg];[base][lg]overlay={x}:{y}[vout]");
        apply.extend(["-filter_complex", &fc]);
        apply.extend(["-map", "[vout]"]);
        if probe.has_audio {
            apply.extend(["-map", "0:a:0"]);
        }
    } else {
        apply.extend(["-vf", &vf]);
    }
    apply.extend([
        "-c:v",
        "libx264",
        "-preset",
        "medium",
        "-crf",
        &args.crf.unwrap_or(20).clamp(0, 51).to_string(),
        "-pix_fmt",
        "yuv420p",
    ]);
    if args.to.is_none() {
        apply.extend(["-movflags", "+faststart"]);
    }
    push_metadata(&mut apply, &args);

    let mut measure: Option<Argv> = None;
    let mut measured: Option<serde_json::Value> = None;
    if probe.has_audio {
        let filter = loudnorm::measure_filter(TARGET_I, TARGET_TP, TARGET_LRA);
        let mut m = Argv::ffmpeg();
        m.extend(["-nostats", "-i"]);
        m.push(&args.input);
        m.extend(["-af", &filter, "-vn", "-f", "null", "-"]);
        if !g.dry_run {
            crate::paths::ensure_output_allowed(&args.output, &[&args.input], g.overwrite)?;
            let spawned = spawn::run(&m, g.timeout, false)?;
            let spawned = spawn::require_ok(&m, spawned)?;
            let meas = loudnorm::parse_measured(&spawn::stderr_str(&spawned))?;
            let ln = loudnorm::apply_filter(TARGET_I, TARGET_TP, TARGET_LRA, &meas, false);
            if let Some(fc) = &mut wrap_fc {
                let fcs = fc.trim_end_matches(';').to_string();
                *fc = format!("{fcs};[ac]{ln}[aout]");
            } else {
                apply.extend(["-af", &ln]);
            }
            measured = Some(meas);
        } else if let Some(fc) = &mut wrap_fc {
            let fcs = fc.trim_end_matches(';').to_string();
            *fc = format!("{fcs};[ac]{filter}[aout]");
        } else {
            apply.extend(["-af", &filter]);
        }
        apply.extend(["-c:a", "aac", "-ar", "48000", "-b:a", "192k"]);
        if let Some(ch) = args.channels {
            apply.extend(["-ac", &ch.to_string()]);
        }
        measure = Some(m);
    } else {
        apply.push("-an");
    }
    if let Some(fc) = wrap_fc {
        apply.extend(["-filter_complex", fc.trim_end_matches(';')]);
        apply.extend(["-map", &format!("[{wrap_vout}]")]);
        if probe.has_audio {
            apply.extend(["-map", "[aout]"]);
        }
    }
    // --to: rendered pack pushed straight to ingest — drop faststart (no
    // moov in FLV), add zerolatency, emit -f flv instead of a file
    let to = args.to.clone();
    if let Some(t) = args.preview {
        if !t.is_finite() || t <= 0.0 {
            return Err(Error::input("deliver --preview needs a positive duration"));
        }
        apply.extend(["-t", &t.to_string()]);
    }
    if to.is_none() {
        apply.push(&args.output);
    }

    if let Some(url) = &to {
        apply.extend(["-tune", "zerolatency", "-f", "flv"]);
        apply.push(url);
        let mut c =
            crate::verbs::live::stream_out("deliver", Some(&args.input), url, vec![apply], g)?;
        if let Some(m) = measure {
            let mut commands = engine::commands_of(&[m]);
            commands.extend(c.commands.clone());
            c.commands = commands;
        }
        return Ok(finish(
            c,
            platform,
            (fw, fh),
            measured,
            args.fps.unwrap_or(30),
            &args,
        ));
    }

    let mut argvs = Vec::new();
    if let Some(m) = measure {
        if g.dry_run {
            argvs.push(m);
        } else {
            // measure already ran; keep it in the contract command list
            let mut c = engine::write_job("deliver", &[&args.input], &args.output, vec![apply], g)?;
            let mut commands = engine::commands_of(&[m]);
            commands.extend(c.commands.clone());
            c.commands = commands;
            return Ok(finish(
                c,
                platform,
                (fw, fh),
                measured,
                args.fps.unwrap_or(30),
                &args,
            ));
        }
    }

    argvs.push(apply);
    let c = engine::write_job("deliver", &[&args.input], &args.output, argvs, g)?;
    Ok(finish(
        c,
        platform,
        (fw, fh),
        measured,
        args.fps.unwrap_or(30),
        &args,
    ))
}

fn finish(
    c: Contract,
    platform: &str,
    frame: (u32, u32),
    measured: Option<serde_json::Value>,
    fps: u32,
    args: &DeliverArgs,
) -> Contract {
    c.with_extra(json!({
        "platform": platform,
        "frame": format!("{}x{}", frame.0, frame.1),
        "fps": fps,
        "target_i": TARGET_I,
        "target_tp": TARGET_TP,
        "measured": measured,
        "intro": args.intro.is_some(),
        "outro": args.outro.is_some(),
        "logo": args.logo.is_some(),
    }))
}

fn platform_name(p: DeliverPlatform) -> &'static str {
    match p {
        DeliverPlatform::Social => "social",
        DeliverPlatform::Reels => "reels",
        DeliverPlatform::Tiktok => "tiktok",
        DeliverPlatform::Shorts => "shorts",
        DeliverPlatform::Square => "square",
        DeliverPlatform::Youtube => "youtube",
        DeliverPlatform::Xhs => "xhs",
        DeliverPlatform::Wechat => "wechat",
        DeliverPlatform::Podcast => "podcast",
        DeliverPlatform::Audiobook => "audiobook",
        DeliverPlatform::Douyin => "douyin",
        DeliverPlatform::Kuaishou => "kuaishou",
        DeliverPlatform::Bilibili => "bilibili",
        DeliverPlatform::Pinterest => "pinterest",
        DeliverPlatform::X => "x",
        DeliverPlatform::Linkedin => "linkedin",
    }
}

/// Apple-style container tags — title/author(album artist)/album/genre/
/// comment land on every platform's output.
fn push_metadata(apply: &mut Argv, args: &DeliverArgs) {
    for (k, v) in [
        ("title", &args.title),
        ("artist", &args.author),
        ("album", &args.album),
        ("genre", &args.genre),
        ("comment", &args.comment),
    ] {
        if let Some(v) = v {
            if !v.trim().is_empty() {
                apply.extend(["-metadata", &format!("{k}={v}")]);
            }
        }
    }
}

/// Audio-only feed pack: loudnorm to the podcast spec (−16 LUFS) → m4a AAC.
/// Accepts audio-only inputs — the video platforms require a video track.
fn podcast(args: DeliverArgs, probe: &crate::probe::Probe, g: &Globals) -> Result<Contract, Error> {
    if !probe.has_audio {
        return Err(Error::input(
            "deliver --platform podcast needs an audio stream",
        ));
    }
    let mut apply = ffmpeg_base(g.progress);
    apply.push("-i");
    apply.push(&args.input);
    let mut ni = 1u32;
    if let Some(cover) = &args.cover {
        crate::paths::ensure_input(cover)?;
        apply.push("-i");
        apply.push(cover);
        apply.extend(["-map", "0:a", "-map", "1:v"]);
        ni += 1;
    } else {
        apply.push("-vn");
    }
    // --chapters: YouTube-format marks ("mm:ss title", the file
    // `chapter --yt` writes) become real container chapters — Apple
    // Podcasts/Apple Books turn them into seek stops.
    let mut chap_marks: Vec<(f64, String)> = Vec::new();
    let mut chap_file: Option<std::path::PathBuf> = None;
    if let Some(cf) = &args.chapters {
        crate::paths::ensure_input(cf)?;
        let marks = crate::verbs::chapter::parse_yt_list(cf)?;
        crate::verbs::chapter::check_marks(&marks, probe.duration, "deliver --chapters")?;
        let meta = crate::verbs::chapter::ffmeta_table(&marks, probe.duration);
        let tmp =
            std::env::temp_dir().join(format!("ffkit-deliver-chap-{}.ffmeta", std::process::id()));
        std::fs::write(&tmp, meta).map_err(|e| Error::output(format!("writing chapters: {e}")))?;
        chap_marks = marks;
        apply.extend(["-f", "ffmetadata", "-i"]);
        apply.push(&tmp);
        apply.extend(["-map_metadata", &ni.to_string()]);
        apply.extend(["-map_chapters", &ni.to_string()]);
        chap_file = Some(tmp);
    }
    let mut m = Argv::ffmpeg();
    m.extend(["-nostats", "-i"]);
    m.push(&args.input);
    m.extend([
        "-af",
        &loudnorm::measure_filter(PODCAST_I, TARGET_TP, TARGET_LRA),
        "-vn",
        "-f",
        "null",
        "-",
    ]);
    let mut measured: Option<serde_json::Value> = None;
    if !g.dry_run {
        crate::paths::ensure_output_allowed(&args.output, &[&args.input], g.overwrite)?;
        let spawned = spawn::run(&m, g.timeout, false)?;
        let spawned = spawn::require_ok(&m, spawned)?;
        let meas = loudnorm::parse_measured(&spawn::stderr_str(&spawned))?;
        apply.extend([
            "-af",
            &loudnorm::apply_filter(PODCAST_I, TARGET_TP, TARGET_LRA, &meas, false),
        ]);
        measured = Some(meas);
    } else {
        apply.extend([
            "-af",
            &loudnorm::measure_filter(PODCAST_I, TARGET_TP, TARGET_LRA),
        ]);
    }
    let ab = if matches!(args.platform, DeliverPlatform::Audiobook) {
        "96k"
    } else {
        "128k"
    };
    apply.extend(["-c:a", "aac", "-ar", "48000", "-b:a", ab]);
    if args.cover.is_some() || matches!(args.platform, DeliverPlatform::Audiobook) {
        // .m4a/.m4b resolve to the ipod muxer, which rejects video streams
        // in ffmpeg 4.x — force mp4 (same container) so mjpeg attaches and
        // the chapter table writes.
        if args.cover.is_some() {
            apply.extend(["-c:v", "mjpeg", "-disposition:v:1", "attached_pic"]);
        }
        apply.extend(["-f", "mp4"]);
    }
    push_metadata(&mut apply, &args);
    if let Some(ch) = args.channels {
        apply.extend(["-ac", &ch.to_string()]);
    }
    if let Some(t) = args.preview {
        if !t.is_finite() || t <= 0.0 {
            return Err(Error::input("deliver --preview needs a positive duration"));
        }
        apply.extend(["-t", &t.to_string()]);
    }
    apply.push(&args.output);

    let m_commands = engine::commands_of(std::slice::from_ref(&m));
    let mut argvs = Vec::new();
    if g.dry_run {
        argvs.push(m);
    }
    argvs.push(apply);
    let run = engine::write_job("deliver", &[&args.input], &args.output, argvs, g);
    if let Some(tmp) = &chap_file {
        let _ = std::fs::remove_file(tmp);
    }
    let mut c = run?;
    if !g.dry_run {
        let mut commands = m_commands;
        commands.extend(c.commands.clone());
        c.commands = commands;
    }
    let platform = platform_name(args.platform);
    Ok(c.with_extra(json!({
        "platform": platform,
        "target_i": PODCAST_I,
        "target_tp": TARGET_TP,
        "measured": measured,
        "cover": args.cover.is_some(),
        "chapters": chap_marks.len(),
    })))
}

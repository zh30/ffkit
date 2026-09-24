use serde_json::json;

use crate::cli::{DeliverArgs, DeliverPlatform, Globals};
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
    if matches!(args.platform, DeliverPlatform::Podcast) {
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

    let (fw, fh) = match args.platform {
        DeliverPlatform::Youtube => (1920, 1080),
        DeliverPlatform::Square => (1080, 1080),
        DeliverPlatform::Xhs => (1080, 1440),
        DeliverPlatform::Wechat => (1080, 1260),
        _ => (1080, 1920),
    };
    let mut vf = format!(
        "scale={fw}:{fh}:force_original_aspect_ratio=decrease,pad={fw}:{fh}:(ow-iw)/2:(oh-ih)/2:black,setsar=1,fps={fps}",
        fps = args.fps.unwrap_or(30)
    );
    if let Some(subs) = &args.subs {
        // The subtitles filter parses `:` `'` `,` in filenames — escape them.
        let path = subs
            .canonicalize()
            .map_err(|e| Error::input(format!("{subs:?}: {e}")))?
            .to_string_lossy()
            .replace('\\', "\\\\")
            .replace('\'', "\\'");
        vf.push_str(&format!(",subtitles=filename='{path}'"));
    }
    vf.push_str(",format=yuv420p");
    let platform = platform_name(args.platform);

    let mut apply = ffmpeg_base(g.progress);
    apply.push("-i");
    apply.push(&args.input);
    apply.extend(["-vf", &vf]);
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
            apply.extend([
                "-af",
                &loudnorm::apply_filter(TARGET_I, TARGET_TP, TARGET_LRA, &meas, false),
            ]);
            measured = Some(meas);
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
    // --to: rendered pack pushed straight to ingest — drop faststart (no
    // moov in FLV), add zerolatency, emit -f flv instead of a file
    let to = args.to.clone();
    if to.is_none() {
        apply.push(&args.output);
    }

    if let Some(url) = &to {
        apply.extend(["-tune", "zerolatency", "-f", "flv"]);
        apply.push(url);
        let mut c = crate::verbs::live::stream_out("deliver", &args.input, url, vec![apply], g)?;
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
    ))
}

fn finish(
    c: Contract,
    platform: &str,
    frame: (u32, u32),
    measured: Option<serde_json::Value>,
    fps: u32,
) -> Contract {
    c.with_extra(json!({
        "platform": platform,
        "frame": format!("{}x{}", frame.0, frame.1),
        "fps": fps,
        "target_i": TARGET_I,
        "target_tp": TARGET_TP,
        "measured": measured,
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
    apply.push("-vn");
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
    apply.extend(["-c:a", "aac", "-ar", "48000", "-b:a", "128k"]);
    if let Some(ch) = args.channels {
        apply.extend(["-ac", &ch.to_string()]);
    }
    apply.push(&args.output);

    let m_commands = engine::commands_of(std::slice::from_ref(&m));
    let mut argvs = Vec::new();
    if g.dry_run {
        argvs.push(m);
    }
    argvs.push(apply);
    let mut c = engine::write_job("deliver", &[&args.input], &args.output, argvs, g)?;
    if !g.dry_run {
        let mut commands = m_commands;
        commands.extend(c.commands.clone());
        c.commands = commands;
    }
    Ok(c.with_extra(json!({
        "platform": "podcast",
        "target_i": PODCAST_I,
        "target_tp": TARGET_TP,
        "measured": measured,
    })))
}

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

pub fn run(args: DeliverArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "deliver")?;

    let (fw, fh) = match args.platform {
        DeliverPlatform::Youtube => (1920, 1080),
        DeliverPlatform::Square => (1080, 1080),
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
        "-movflags",
        "+faststart",
    ]);

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
        measure = Some(m);
    } else {
        apply.push("-an");
    }
    apply.push(&args.output);

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
    }
}

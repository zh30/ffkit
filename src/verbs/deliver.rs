use serde_json::json;

use crate::cli::{DeliverArgs, DeliverPlatform, Globals};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::spawn::{self, Argv};
use crate::verbs::loudnorm;

const FRAME_W: u32 = 1080;
const FRAME_H: u32 = 1920;
const TARGET_I: f64 = -14.0;
const TARGET_TP: f64 = -1.5;
const TARGET_LRA: f64 = 11.0;

pub fn run(args: DeliverArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "deliver")?;

    let vf = format!(
        "scale={FRAME_W}:{FRAME_H}:force_original_aspect_ratio=decrease,pad={FRAME_W}:{FRAME_H}:(ow-iw)/2:(oh-ih)/2:black,setsar=1,fps=30,format=yuv420p"
    );
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
        "20",
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
                &loudnorm::apply_filter(TARGET_I, TARGET_TP, TARGET_LRA, &meas),
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
            return Ok(finish(c, platform, measured));
        }
    }

    argvs.push(apply);
    let c = engine::write_job("deliver", &[&args.input], &args.output, argvs, g)?;
    Ok(finish(c, platform, measured))
}

fn finish(c: Contract, platform: &str, measured: Option<serde_json::Value>) -> Contract {
    c.with_extra(json!({
        "platform": platform,
        "frame": format!("{FRAME_W}x{FRAME_H}"),
        "fps": 30,
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
    }
}

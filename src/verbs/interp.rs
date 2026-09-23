use crate::cli::InterpArgs;
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use serde_json::json;

/// Motion-compensated frame interpolation (minterpolate): upres to 60/120fps
/// for high-refresh delivery, or `--slow` for smooth slow-mo from normal-rate
/// footage (it synthesizes the in-between frames).
pub fn run(args: InterpArgs, g: &crate::cli::Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "interp")?;
    if args.fps.is_some() && args.slow.is_some() {
        return Err(Error::input("--fps and --slow are exclusive"));
    }
    let mode = match args.mode {
        crate::cli::InterpMode::Mci => "mci",
        crate::cli::InterpMode::Blend => "blend",
        crate::cli::InterpMode::Dup => "dup",
    };
    let (vf, out_fps) = if let Some(slow) = args.slow {
        let slow = slow.clamp(0.02, 1.0);
        let src_fps = probe
            .fps
            .ok_or_else(|| Error::input("can't read source fps for --slow"))?;
        (
            format!(
                "setpts={:.4}*PTS,minterpolate=fps={src_fps}:mi_mode={mode}",
                1.0 / slow
            ),
            src_fps,
        )
    } else {
        let fps = args.fps.unwrap_or(60.0);
        (format!("minterpolate=fps={fps}:mi_mode={mode}"), fps)
    };

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend([
        "-vf", &vf, "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
    ]);
    if probe.has_audio {
        argv.extend(["-c:a", "copy"]);
    }
    argv.push(&args.output);
    let c = engine::write_job("interp", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "mode": mode,
        "fps": out_fps,
        "slow": args.slow,
        "filter": vf,
    })))
}

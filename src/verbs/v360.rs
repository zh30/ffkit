use serde_json::json;

use crate::cli::{Globals, V360Args};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

/// Reframe a 360/equirectangular source into a flat viewport —
/// pick the look direction (`--yaw`/`--pitch`) and lens (`--fov`).
pub fn run(args: V360Args, g: &Globals) -> Result<Contract, Error> {
    if !(-180.0..=180.0).contains(&args.yaw) || !(-180.0..=180.0).contains(&args.pitch) {
        return Err(Error::input("--yaw/--pitch must be -180..180"));
    }
    if !(10.0..=180.0).contains(&args.fov) {
        return Err(Error::input("--fov must be 10..180"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "v360")?;

    let (w, h) = match &args.size {
        Some(s) => s
            .split_once('x')
            .and_then(|(a, b)| Some((a.parse::<u32>().ok()?, b.parse::<u32>().ok()?)))
            .filter(|(w, h)| *w >= 64 && *h >= 64 && *w <= 8192 && *h <= 8192)
            .ok_or_else(|| Error::input("--size must be WxH (64..8192)"))?,
        None => {
            let w = probe.width.unwrap_or(1920).min(3840);
            (w, (w * 9 / 16) & !1)
        }
    };
    let vf = format!(
        "v360=equirect:flat:yaw={:.2}:pitch={:.2}:h_fov={:.2}:w={w}:h={h}",
        args.yaw, args.pitch, args.fov
    );

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-vf", &vf]);
    argv.extend([
        "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
    ]);
    if probe.has_audio {
        argv.extend(["-c:a", "copy"]);
    }
    argv.push(&args.output);

    let c = engine::write_job("v360", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "yaw": args.yaw,
        "pitch": args.pitch,
        "fov": args.fov,
        "filter": "v360",
    })))
}

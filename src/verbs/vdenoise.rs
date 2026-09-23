use serde_json::json;

use crate::cli::{Globals, VdenoiseArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: VdenoiseArgs, g: &Globals) -> Result<Contract, Error> {
    if !(0.5..=30.0).contains(&args.strength) {
        return Err(Error::input("--strength must be 0.5..=30"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "vdenoise")?;

    let s = args.strength;
    // all three engines accept the timeline `enable` option, so a window is a
    // flag on the filter, not a split graph — cheap to keep the rest untouched.
    let (base, filter_name) = match args.engine.unwrap_or_default() {
        crate::cli::VDenoiseEngine::Nlmeans => (format!("nlmeans=s={s:.1}"), "nlmeans"),
        crate::cli::VDenoiseEngine::Hqdn3d => (
            format!(
                "hqdn3d=luma_spatial={:.1}:chroma_spatial={:.1}:luma_tmp={:.1}",
                s * 2.0,
                s * 1.5,
                s * 2.0
            ),
            "hqdn3d",
        ),
        crate::cli::VDenoiseEngine::Atadenoise => (
            format!("atadenoise=s={:.0}", (5.0 + s * 4.0).min(129.0)),
            "atadenoise",
        ),
        crate::cli::VDenoiseEngine::Vaguedenoise => (
            format!("vaguedenoiser=threshold={:.1}", s * 3.0),
            "vaguedenoiser",
        ),
    };
    let vf = match &args.at {
        Some(raw) => format!(
            "{base}:enable='{}'",
            crate::time::enable_expr(raw, args.dur, probe.duration)?
        ),
        None => {
            if args.dur.is_some() {
                return Err(Error::input("--dur needs --at"));
            }
            base
        }
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

    let c = engine::write_job("vdenoise", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "strength": s,
        "filter": filter_name,
    })))
}

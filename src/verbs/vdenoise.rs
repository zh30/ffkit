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
    // every engine except bm3d accepts the timeline `enable` option, so a
    // window is a flag on the filter, not a split graph.
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
        // bm3d: patch-stack Wiener filtering — the strongest spatial denoiser
        // ffmpeg has; it has no timeline `enable`, so --at is handled below.
        // bm3d's sigma is the assumed noise level — real noise sits far above
        // the 0-5 'mild' band, so strength maps s*8 (6 → 48: flat-region luma
        // stdev halved on a noise=25 fixture)
        crate::cli::VDenoiseEngine::Bm3d => (format!("bm3d=sigma={:.1}", s * 8.0), "bm3d"),
        // dctdnoiz: DCT-domain sigma — aggressive cut that keeps edges sharp
        crate::cli::VDenoiseEngine::Dctdnoiz => {
            (format!("dctdnoiz=sigma={:.1}", s * 5.0), "dctdnoiz")
        }
        // owdenoise: overcomplete wavelet — smoothest output of the set
        crate::cli::VDenoiseEngine::Owdenoise => (
            format!(
                "owdenoise=luma_strength={:.1}:chroma_strength={:.1}",
                s * 4.0,
                s * 4.0
            ),
            "owdenoise",
        ),
    };
    let vf = match &args.at {
        Some(raw) => {
            // bm3d/dctdnoiz/owdenoise have no timeline `enable` on ffmpeg 4.4
            if matches!(filter_name, "bm3d" | "dctdnoiz" | "owdenoise") {
                return Err(Error::input(format!(
                    "--at needs a timeline-capable engine — {filter_name} has none; \n\
                     use nlmeans/hqdn3d/atadenoise/vaguedenoise, or denoise the whole clip"
                )));
            }
            format!(
                "{base}:enable='{}'",
                crate::time::enable_expr(raw, args.dur, probe.duration)?
            )
        }
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

use serde_json::json;

use crate::cli::{GlitchArgs, Globals};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: GlitchArgs, g: &Globals) -> Result<Contract, Error> {
    if !(0.5..=20.0).contains(&args.strength) {
        return Err(Error::input("--strength must be 0.5..=20"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "glitch")?;

    let vf = match args.engine.unwrap_or(crate::cli::GlitchEngine::Shift) {
        crate::cli::GlitchEngine::Shift => {
            // rgb channel offset + temporal noise = datamosh-style glitch
            let s = args.strength.round() as i32;
            let nz = (args.strength * 4.0).round() as i32;
            format!("format=rgba,rgbashift=rh={s}:bh=-{s},noise=alls={nz}:allf=t+u,format=yuv420p")
        }
        crate::cli::GlitchEngine::Planes => {
            // shuffleplanes channel rotation: RGB rotated per frame →
            // psychedelic false color (clean, no noise grain)
            "format=gbrp,shuffleplanes=1:2:0:3,format=yuv420p".to_string()
        }
        crate::cli::GlitchEngine::Swapuv => "swapuv".to_string(),
        crate::cli::GlitchEngine::Stutter => "shuffleframes=0 1 1 2".to_string(),
        crate::cli::GlitchEngine::Swaprect => {
            // swaprect takes literal pixel ints on 4.x (no iw/ih eval)
            let (w, h) = (
                probe.width.unwrap_or(320) / 2,
                probe.height.unwrap_or(240) / 2,
            );
            format!("swaprect=w={w}:h={h}:x1=0:y1=0:x2={w}:y2={h}")
        }
        crate::cli::GlitchEngine::Pixels => {
            // block-scatter shuffle: strength 0.5..20 → block width 64..8 px
            let bw = (64.0 - (args.strength - 0.5) / 19.5 * 56.0).round() as i32;
            format!("shufflepixels=mode=block:width={bw}")
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

    let c = engine::write_job("glitch", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "strength": args.strength,
        "filter": match args.engine.unwrap_or(crate::cli::GlitchEngine::Shift) {
            crate::cli::GlitchEngine::Shift => "rgbashift+noise",
            crate::cli::GlitchEngine::Planes => "shuffleplanes",
            crate::cli::GlitchEngine::Swapuv => "swapuv",
            crate::cli::GlitchEngine::Stutter => "shuffleframes",
            crate::cli::GlitchEngine::Pixels => "shufflepixels",
            crate::cli::GlitchEngine::Swaprect => "swaprect",
        },
    })))
}

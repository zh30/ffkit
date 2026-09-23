use serde_json::json;

use crate::cli::{Globals, HeatArgs, HeatPreset};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::time::enable_expr;

pub fn run(args: HeatArgs, g: &Globals) -> Result<Contract, Error> {
    let preset = match args.preset {
        HeatPreset::Magma => "magma",
        HeatPreset::Inferno => "inferno",
        HeatPreset::Plasma => "plasma",
        HeatPreset::Viridis => "viridis",
        HeatPreset::Turbo => "turbo",
        HeatPreset::Cividis => "cividis",
        HeatPreset::Range1 => "range1",
        HeatPreset::Range2 => "range2",
        HeatPreset::Shadows => "shadows",
        HeatPreset::Highlights => "highlights",
    };
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "heat")?;
    let en = match &args.at {
        Some(a) => format!(":enable='{}'", enable_expr(a, args.dur, probe.duration)?),
        None => {
            if args.dur.is_some() {
                return Err(Error::input("--dur needs --at"));
            }
            String::new()
        }
    };

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    let vf = format!(
        "pseudocolor=preset={preset}:opacity={:.2}{en}",
        args.opacity
    );
    argv.extend(["-vf", &vf]);
    argv.extend([
        "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
    ]);
    if probe.has_audio {
        argv.extend(["-c:a", "copy"]);
    }
    argv.push(&args.output);

    let c2 = engine::write_job("heat", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c2.with_extra(json!({ "preset": preset, "opacity": args.opacity })))
}

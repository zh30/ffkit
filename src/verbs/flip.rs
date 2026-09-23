use serde_json::json;

use crate::cli::{FlipArgs, FlipAxis, Globals};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: FlipArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "flip")?;

    // unmirror selfie footage / flip artwork
    let f = match args.axis {
        FlipAxis::X => "hflip",
        FlipAxis::Y => "vflip",
    };
    let vf = match &args.at {
        Some(s) => format!(
            "{f}=enable='{}'",
            crate::time::enable_expr(s, args.dur, probe.duration)?
        ),
        None => {
            if args.dur.is_some() {
                return Err(Error::input("--dur needs --at"));
            }
            f.to_string()
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

    let c = engine::write_job("flip", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "axis": format!("{:?}", args.axis).to_lowercase(),
        "filter": f,
    })))
}

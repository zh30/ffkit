use serde_json::json;

use crate::cli::{Globals, VignetteArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: VignetteArgs, g: &Globals) -> Result<Contract, Error> {
    if !(0.2..=1.6).contains(&args.angle) {
        return Err(Error::input(
            "--angle must be 0.2..=1.6 radians (smaller is stronger)",
        ));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "vignette")?;

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend([
        "-vf",
        &{
            match &args.at {
                Some(s) => format!(
                    "vignette=angle={}:enable='{}'",
                    args.angle,
                    crate::time::enable_expr(s, args.dur, probe.duration)?
                ),
                None => {
                    if args.dur.is_some() {
                        return Err(Error::input("--dur needs --at"));
                    }
                    format!("vignette=angle={}", args.angle)
                }
            }
        },
        "-c:v",
        "libx264",
        "-preset",
        "fast",
        "-crf",
        "18",
        "-pix_fmt",
        "yuv420p",
    ]);
    if probe.has_audio {
        argv.extend(["-c:a", "copy"]);
    }
    argv.push(&args.output);

    let c = engine::write_job("vignette", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({ "angle": args.angle })))
}

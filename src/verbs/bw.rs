use serde_json::json;

use crate::cli::{BwArgs, Globals};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: BwArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "bw")?;
    let sat = match args.strength {
        Some(s) if (0.0..=1.0).contains(&s) => 1.0 - s,
        Some(_) => return Err(Error::input("--strength must be 0..=1")),
        None => 0.0,
    };

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend([
        "-vf",
        &{
            match &args.at {
                Some(s) => {
                    let at = crate::time::resolve_at(s, args.dur, probe.duration)?;
                    if !(0.0..probe.duration).contains(&at) {
                        return Err(Error::input("--at is outside the input"));
                    }
                    match args.dur {
                        Some(d) if at + d < probe.duration => {
                            format!("hue=s={sat}:enable='between(t,{at:.3},{:.3})'", at + d)
                        }
                        _ => format!("hue=s={sat}:enable='gte(t,{at:.3})'"),
                    }
                }
                None => {
                    if args.dur.is_some() {
                        return Err(Error::input("--dur needs --at"));
                    }
                    format!("hue=s={sat}")
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

    let c = engine::write_job("bw", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({ "filter": "hue=s=0" })))
}

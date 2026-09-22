use serde_json::json;

use crate::cli::{BwArgs, Globals};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: BwArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "bw")?;

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend([
        "-vf",
        &{
            match &args.at {
                Some(s) => {
                    let at = crate::time::parse_time(s)?;
                    if !(0.0..probe.duration).contains(&at) {
                        return Err(Error::input("--at is outside the input"));
                    }
                    match args.dur {
                        Some(d) if at + d < probe.duration => {
                            format!("hue=s=0:enable='between(t,{at:.3},{:.3})'", at + d)
                        }
                        _ => format!("hue=s=0:enable='gte(t,{at:.3})'"),
                    }
                }
                None => {
                    if args.dur.is_some() {
                        return Err(Error::input("--dur needs --at"));
                    }
                    "hue=s=0".to_string()
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

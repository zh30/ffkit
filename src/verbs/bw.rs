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

    let weights = match &args.weights {
        None => None,
        Some(w) => {
            let v: Vec<f64> = w
                .split(',')
                .map(|x| x.trim().parse::<f64>().unwrap_or(f64::NAN))
                .collect();
            if v.len() != 3 || v.iter().any(|x| x.is_nan()) {
                return Err(Error::input("--weights needs three numbers r,g,b"));
            }
            Some(v)
        }
    };
    if weights.is_some() && args.strength.is_some() {
        return Err(Error::input(
            "--weights replaces the luma mix — no --strength",
        ));
    }

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend([
        "-vf",
        &{
            let base = match &weights {
                // colorchannelmixer: same luma mix on all three output
                // channels = true grayscale with custom weights
                Some(v) => format!(
                    "colorchannelmixer=rr={}:rg={}:rb={}:gr={}:gg={}:gb={}:br={}:bg={}:bb={}",
                    v[0], v[1], v[2], v[0], v[1], v[2], v[0], v[1], v[2]
                ),
                None => format!("hue=s={sat}"),
            };
            match &args.at {
                Some(s) => format!(
                    "{base}:enable='{}'",
                    crate::time::enable_expr(s, args.dur, probe.duration)?
                ),
                None => {
                    if args.dur.is_some() {
                        return Err(Error::input("--dur needs --at"));
                    }
                    base
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

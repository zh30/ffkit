use serde_json::json;

use crate::cli::{Globals, ReverbArgs, ReverbSize};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

/// Room ambience via aecho: the size preset picks the delay taps, --wet scales
/// every decay uniformly so one knob goes dry to soaked.
pub fn run(args: ReverbArgs, g: &Globals) -> Result<Contract, Error> {
    if !(0.0..=0.9).contains(&args.wet) || args.wet == 0.0 {
        return Err(Error::input("--wet must be in (0, 0.9]"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_audio {
        return Err(Error::input("reverb needs an audio stream"));
    }
    let (delays, bases): (&str, &[f64]) = match args.size {
        ReverbSize::Room => ("50|80", &[0.8, 0.55]),
        ReverbSize::Hall => ("100|180|250", &[0.9, 0.7, 0.5]),
        ReverbSize::Cave => ("200|400|600", &[0.95, 0.8, 0.6]),
    };
    let decays = bases
        .iter()
        .map(|b| format!("{:.3}", (b * args.wet).min(0.95)))
        .collect::<Vec<_>>()
        .join("|");
    let af = format!("aecho=0.8:0.88:{delays}:{decays}");

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-af", &af, "-map", "0:v?", "-map", "0:a"]);
    if probe.has_video {
        argv.extend(["-c:v", "copy"]);
    }
    argv.extend(["-c:a", "aac"]);
    argv.push(&args.output);

    let c = engine::write_job("reverb", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "size": format!("{:?}", args.size).to_lowercase(),
        "wet": args.wet,
        "filter": af,
    })))
}

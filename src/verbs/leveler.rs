use serde_json::json;

use crate::cli::{Globals, LevelerArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: LevelerArgs, g: &Globals) -> Result<Contract, Error> {
    if !(-60.0..=0.0).contains(&args.threshold) {
        return Err(Error::input("--threshold must be -60..=0 dB"));
    }
    if !(1.0..=20.0).contains(&args.ratio) {
        return Err(Error::input("--ratio must be 1..=20"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_audio {
        return Err(Error::input("leveler: input has no audio stream"));
    }

    // preset fills any knob still at its default value
    let (pt, pr, pa, prel, pm) = match args.preset {
        Some(crate::cli::LevelerPreset::Voice) => (-18.0, 3.0, 8.0, 120.0, 4.0),
        Some(crate::cli::LevelerPreset::Podcast) => (-20.0, 4.0, 10.0, 150.0, 3.0),
        Some(crate::cli::LevelerPreset::Master) => (-12.0, 2.0, 5.0, 80.0, 2.0),
        None => (
            args.threshold,
            args.ratio,
            args.attack,
            args.release,
            args.makeup,
        ),
    };
    let af =
        format!("acompressor=threshold={pt}dB:ratio={pr}:attack={pa}:release={prel}:makeup={pm}dB");
    let fc = match &args.at {
        Some(raw) => {
            let at = crate::time::resolve_at(raw, args.dur, probe.duration)?;
            if !(0.0..probe.duration).contains(&at) {
                return Err(Error::input("--at is outside the input"));
            }
            Some(engine::audio_window(&af, at, args.dur))
        }
        None => {
            if args.dur.is_some() {
                return Err(Error::input("--dur needs --at"));
            }
            None
        }
    };
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    match &fc {
        Some(fc) => {
            argv.extend(["-filter_complex", fc, "-map", "0:v?", "-map", "[aout]"]);
        }
        None => argv.extend(["-af", &af, "-map", "0:v?", "-map", "0:a"]),
    }
    if probe.has_video {
        argv.extend(["-c:v", "copy"]);
    }
    argv.extend(["-c:a", "aac"]);
    argv.push(&args.output);

    let c = engine::write_job("leveler", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "threshold_db": args.threshold,
        "ratio": args.ratio,
        "makeup_db": args.makeup,
    })))
}

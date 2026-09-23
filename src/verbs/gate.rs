use serde_json::json;

use crate::cli::{GateArgs, GatePreset, Globals};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: GateArgs, g: &Globals) -> Result<Contract, Error> {
    let (thr_db, ratio, attack, release) = match args.preset {
        Some(GatePreset::Voice) => (-40.0, 8.0, 10.0, 100.0),
        Some(GatePreset::Podcast) => (-45.0, 10.0, 15.0, 150.0),
        Some(GatePreset::Studio) => (-50.0, 4.0, 5.0, 80.0),
        None => (args.threshold, args.ratio, args.attack, args.release),
    };
    if !(-80.0..=0.0).contains(&thr_db) {
        return Err(Error::input("--threshold must be -80..=0 dB"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_audio {
        return Err(Error::input("gate: input has no audio stream"));
    }

    // agate threshold is linear amplitude, creators think in dB.
    let lin = 10f64.powf(thr_db / 20.0);
    let af = format!(
        "agate=threshold={lin:.6}:ratio={r}:attack={a}:release={rel}:makeup=2",
        r = ratio,
        a = attack,
        rel = release,
    );
    let fc = match &args.at {
        Some(raw) => Some(engine::audio_window_for(
            &af,
            raw,
            args.dur,
            probe.duration,
        )?),
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
        None => argv.extend(["-af", &af]),
    }
    if probe.has_video {
        argv.extend(["-c:v", "copy"]);
    }
    argv.extend(["-c:a", "aac"]);
    argv.push(&args.output);

    let c = engine::write_job("gate", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "threshold_db": thr_db,
        "ratio": ratio,
    })))
}

use serde_json::json;

use crate::cli::{Globals, VoiceArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: VoiceArgs, g: &Globals) -> Result<Contract, Error> {
    if !(-80.0..0.0).contains(&args.threshold) {
        return Err(Error::input("--threshold must be -80..0 dB"));
    }
    if !(-30.0..-5.0).contains(&args.lufs) {
        return Err(Error::input("--lufs must be -30..-5"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_audio {
        return Err(Error::input("voice: input has no audio stream"));
    }

    // One-shot podcast chain: gate the hiss, level the swings, hit platform
    // loudness — the three steps creators otherwise run by hand.
    let lin = 10f64.powf(args.threshold / 20.0);
    let af = format!(
        "agate=threshold={lin:.6}:ratio=6:attack=10:release=120,\
         acompressor=threshold=-18dB:ratio=3:attack=20:release=250:makeup=5dB,\
         loudnorm=I={:.1}:TP=-1.5:LRA=11",
        args.lufs
    );
    // --at/--dur: polish only a window via the shared dry/wet splitter.
    let fc = match &args.at {
        Some(raw) => {
            let at = crate::time::parse_time(raw)?;
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
    if let Some(fc) = &fc {
        argv.extend(["-filter_complex", fc]);
        if probe.has_video {
            argv.extend(["-map", "0:v", "-map", "[aout]", "-c:v", "copy"]);
        } else {
            argv.extend(["-map", "[aout]"]);
        }
    } else {
        argv.extend(["-af", &af]);
        if probe.has_video {
            argv.extend(["-c:v", "copy"]);
        }
    }
    argv.extend(["-c:a", "aac"]);
    argv.push(&args.output);

    let c = engine::write_job("voice", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "gate_threshold_db": args.threshold,
        "target_lufs": args.lufs,
    })))
}

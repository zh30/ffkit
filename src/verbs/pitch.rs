use serde_json::json;

use crate::cli::{Globals, PitchArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: PitchArgs, g: &Globals) -> Result<Contract, Error> {
    if !(-12.0..=12.0).contains(&args.semitones) || args.semitones == 0.0 {
        return Err(Error::input("--semitones must be in -12..=12, not 0"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_audio {
        return Err(Error::input("pitch needs an audio stream"));
    }
    // asetrate shifts pitch AND speed; atempo restores duration.
    let factor = (args.semitones / 12.0).exp2();
    let sr = probe.sample_rate.unwrap_or(48000).max(8000);
    let af = format!(
        "asetrate={sr}*{factor:.6},aresample={sr},atempo={inv:.6}",
        inv = 1.0 / factor
    );
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-af", &af, "-map", "0:v?", "-map", "0:a"]);
    if probe.has_video {
        argv.extend(["-c:v", "copy"]);
    }
    argv.extend(["-c:a", "aac"]);
    argv.push(&args.output);

    let c = engine::write_job("pitch", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "semitones": args.semitones,
        "factor": factor,
    })))
}

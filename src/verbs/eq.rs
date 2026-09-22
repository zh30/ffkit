use serde_json::json;

use crate::cli::{EqArgs, Globals};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: EqArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_audio {
        return Err(Error::input("eq needs an audio stream"));
    }
    // --preset fills whichever bands were left at 0.
    let (mut bass, mut presence, mut treble) = (args.bass, args.presence, args.treble);
    if let Some(preset) = args.preset {
        use crate::cli::EqPreset::*;
        let (b, p, tr) = match preset {
            Voice => (2.0, 3.0, 0.0),
            Podcast => (-1.0, 4.0, 1.0),
            Bright => (0.0, 2.0, 5.0),
            Bass => (8.0, 0.0, 0.0),
        };
        if bass == 0.0 {
            bass = b;
        }
        if presence == 0.0 {
            presence = p;
        }
        if treble == 0.0 {
            treble = tr;
        }
    }
    for (name, v) in [
        ("--bass", bass),
        ("--treble", treble),
        ("--presence", presence),
    ] {
        if !(-20.0..=20.0).contains(&v) {
            return Err(Error::input(format!("{name} must be -20..=20 dB")));
        }
    }
    let mut chain: Vec<String> = Vec::new();
    if bass != 0.0 {
        chain.push(format!("bass=g={}", bass));
    }
    if presence != 0.0 {
        chain.push(format!("equalizer=f=3000:t=q:w=1:g={}", presence));
    }
    if treble != 0.0 {
        chain.push(format!("treble=g={}", treble));
    }
    if chain.is_empty() {
        return Err(Error::input("eq needs at least one nonzero band"));
    }
    let af = chain.join(",");
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-af", &af, "-map", "0:v?", "-map", "0:a"]);
    if probe.has_video {
        argv.extend(["-c:v", "copy"]);
    }
    argv.extend(["-c:a", "aac"]);
    argv.push(&args.output);

    let c = engine::write_job("eq", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "bass": args.bass,
        "treble": args.treble,
        "presence": args.presence,
    })))
}

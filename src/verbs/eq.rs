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
            Warm => (3.0, -2.0, -1.5),
            Air => (0.0, 1.0, 6.0),
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
    if let Some(tilt) = args.tilt {
        if !(-10.0..=10.0).contains(&tilt) {
            return Err(Error::input("--tilt must be -10..=10 dB"));
        }
        if bass == 0.0 {
            bass = tilt;
        }
        if treble == 0.0 {
            treble = -tilt;
        }
    }
    let mut chain: Vec<String> = Vec::new();
    for band in &args.band {
        let mut parts = band.split(':');
        let f: f64 = parts
            .next()
            .and_then(|s| s.trim().parse().ok())
            .ok_or_else(|| Error::input(format!("--band wants FREQ:GAIN[:WIDTH], got '{band}'")))?;
        let g_gain: f64 = parts
            .next()
            .and_then(|s| s.trim().parse().ok())
            .ok_or_else(|| Error::input(format!("--band wants FREQ:GAIN[:WIDTH], got '{band}'")))?;
        let w_oct: f64 = match parts.next() {
            Some(s) => s.trim().parse().map_err(|_| {
                Error::input(format!("--band width must be a number, got '{band}'"))
            })?,
            None => 1.0,
        };
        if !(20.0..=20000.0).contains(&f) {
            return Err(Error::input("--band frequency must be 20..20000 Hz"));
        }
        if !(-20.0..=20.0).contains(&g_gain) {
            return Err(Error::input("--band gain must be -20..=20 dB"));
        }
        if !(0.1..=4.0).contains(&w_oct) {
            return Err(Error::input("--band width must be 0.1..=4 octaves"));
        }
        chain.push(format!("equalizer=f={f:.0}:t=q:w={w_oct:.2}:g={g_gain:.1}"));
    }
    // --curve: arbitrary freehand EQ line through FREQ:GAIN points (firequalizer
    // interpolates between entries — a tilt, a smile, whatever the creator drew)
    if let Some(curve) = &args.curve {
        let mut entries: Vec<String> = Vec::new();
        for pt in curve
            .split(';')
            .flat_map(|s| s.split(','))
            .collect::<Vec<_>>()
            .chunks(2)
        {
            let f: f64 = pt
                .first()
                .and_then(|s| s.trim().parse().ok())
                .ok_or_else(|| {
                    Error::input("--curve wants \"F,G;F,G\" pairs (freq Hz, gain dB)")
                })?;
            let g_gain: f64 = pt
                .get(1)
                .and_then(|s| s.trim().parse().ok())
                .ok_or_else(|| {
                    Error::input("--curve wants \"F,G;F,G\" pairs (freq Hz, gain dB)")
                })?;
            if !(20.0..=20000.0).contains(&f) {
                return Err(Error::input("--curve frequency must be 20..20000 Hz"));
            }
            if !(-60.0..=20.0).contains(&g_gain) {
                return Err(Error::input("--curve gain must be -60..=20 dB"));
            }
            entries.push(format!("entry({f:.0},{g_gain:.1})"));
        }
        if entries.len() < 2 {
            return Err(Error::input("--curve needs at least two F,G points"));
        }
        chain.push(format!("firequalizer=gain_entry='{}'", entries.join(";")));
    }
    // --graphic: the classic 18-slider EQ (65Hz..20kHz, dB per band)
    if let Some(gr) = &args.graphic {
        let gains: Vec<&str> = gr.split(',').collect();
        if gains.len() > 18 {
            return Err(Error::input("--graphic takes at most 18 dB values"));
        }
        let mut parts: Vec<String> = Vec::new();
        for (i, s) in gains.iter().enumerate() {
            let db: f64 = s
                .trim()
                .parse()
                .map_err(|_| Error::input(format!("--graphic wants dB numbers, got '{s}'")))?;
            if !(-60.0..=20.0).contains(&db) {
                return Err(Error::input("--graphic gains must be -60..=20 dB"));
            }
            parts.push(format!("{}b={:.4}", i + 1, 10f64.powf(db / 20.0)));
        }
        chain.push(format!("superequalizer={}", parts.join(":")));
    }
    // --deemph: standard de-emphasis curve (aemphasis reproduction mode) —
    // undoes the pre-emphasis HF boost baked into vinyl rips / FM / CD captures
    if let Some(d) = &args.deemph {
        let ty = match d {
            crate::cli::DeemphType::Riaa => "riaa",
            crate::cli::DeemphType::Cd => "cd",
            crate::cli::DeemphType::Fm50 => "50fm",
            crate::cli::DeemphType::Fm75 => "75fm",
        };
        chain.push(format!("aemphasis=type={ty}"));
    }
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
        None => argv.extend(["-af", &af, "-map", "0:v?", "-map", "0:a"]),
    }
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

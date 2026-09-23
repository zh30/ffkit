use serde_json::json;

use crate::cli::{FxArgs, FxKind, Globals};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

/// Audio FX rack: one filter per effect, --strength scales its depth.
pub fn run(args: FxArgs, g: &Globals) -> Result<Contract, Error> {
    if !(0.0..=1.0).contains(&args.strength) {
        return Err(Error::input("--strength must be 0..1"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_audio {
        return Err(Error::input("fx: input has no audio stream"));
    }
    let s = args.strength;
    let af = match args.kind {
        FxKind::Tremolo => format!("tremolo=f=6:d={:.2}", 0.3 + 0.7 * s),
        FxKind::Vibrato => format!("vibrato=f=5:d={:.2}", 0.3 + 0.7 * s),
        FxKind::Flanger => format!(
            "flanger=delay={:.1}:depth={:.1}",
            2.0 + 8.0 * s,
            1.0 + 3.0 * s
        ),
        FxKind::Phaser => format!(
            "aphaser=in_gain=0.8:out_gain=0.9:delay={:.1}:decay=0.5:speed={:.2}:type=t",
            1.5 + 4.0 * s,
            0.5 + s
        ),
        FxKind::Chorus => format!("chorus=0.7:0.9:55:0.4:0.25:{:.1}", 1.0 + 2.0 * s),
        FxKind::Echo => format!("aecho=0.8:0.9:{:.0}:{:.2}", 40.0 + 80.0 * s, 0.3 + 0.5 * s),
        FxKind::Lofi => format!(
            "acrusher=level_in=1:level_out=1:bits={}:mode=log:aa=0.7,lowpass=f={:.0}",
            (10.0 - 6.0 * s).round() as u32,
            3200.0 + 1800.0 * s
        ),
        FxKind::Radio => String::from(
            "highpass=f=300,lowpass=f=3400,acompressor=threshold=-24dB:ratio=6:attack=5:release=80:makeup=4dB",
        ),
        FxKind::Saturate => {
            let th = (0.95 - 0.55 * s).clamp(0.05, 0.95);
            format!(
                "asoftclip=type=tanh:threshold={th:.2}:output={:.2}",
                (1.0 / th).min(1.6)
            )
        }
        // harmonic exciter — harmonics generated above `freq` add air/presence
        FxKind::Excite => format!("aexciter=amount={:.2}:freq=7500", 1.0 + 3.0 * s),
        FxKind::Bass => format!("bass=g={:.1}:f=110", 2.0 + 10.0 * s),
        FxKind::Muffled => format!("lowpass=f={:.0}", 2400.0 - 1900.0 * s),
        FxKind::Crystal => format!("crystalizer=i={:.1}:c=false", 1.0 + 4.0 * s),
        // sub-bass synthesis: a generated low octave rides under the mix
        FxKind::Sub => format!(
            "asubboost=dry=0.8:wet={:.2}:decay=0.8:cutoff=100:slope=0.5",
            0.3 + 0.6 * s
        ),
        // headphone crossfeed: bleeds each ear slightly into the other so
        // long listening sessions feel speaker-like instead of hard-panned
        FxKind::Crossfeed => format!("crossfeed=strength={:.2}:range=0.5", 0.2 + 0.6 * s),
    };
    // --at/--dur: duck the dry feed to 0 inside the window, add the FX in its place.
    // (on ffmpeg 4.4 none of these filters accept a timeline `enable` option)
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
            argv.extend(["-filter_complex", fc]);
            if probe.has_video {
                argv.extend(["-map", "0:v"]);
            }
            argv.extend(["-map", "[aout]"]);
        }
        None => argv.extend(["-af", &af]),
    }
    if probe.has_video {
        argv.extend(["-c:v", "copy"]);
    }
    argv.extend(["-c:a", "aac"]);
    argv.push(&args.output);

    let c = engine::write_job("fx", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "effect": format!("{:?}", args.kind).to_lowercase(),
        "strength": s,
        "filter": af,
    })))
}

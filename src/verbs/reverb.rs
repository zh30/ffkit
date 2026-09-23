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
    if args.ir.is_some() {
        return convolve(&args, g, &probe);
    }
    if args.tail.is_some() {
        return Err(Error::input("--tail only applies with --ir"));
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

    let c = engine::write_job("reverb", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "size": format!("{:?}", args.size).to_lowercase(),
        "wet": args.wet,
        "filter": af,
    })))
}

/// Convolution reverb: convolve the source with an impulse-response WAV.
/// afir cuts output at the dry input's length, so the dry side is padded by
/// --tail (default: the IR's own duration) to let the tail ring out.
fn convolve(
    args: &ReverbArgs,
    g: &Globals,
    probe: &crate::probe::Probe,
) -> Result<Contract, Error> {
    let ir = args.ir.as_ref().unwrap();
    crate::paths::ensure_input(ir)?;
    let irp = engine::probe_or_err(ir, g)?;
    if !irp.has_audio {
        return Err(Error::input("reverb --ir file has no audio stream"));
    }
    if args.at.is_some() || args.dur.is_some() {
        return Err(Error::input("reverb --ir doesn't support --at/--dur yet"));
    }
    let tail = args.tail.unwrap_or(irp.duration).clamp(0.0, 60.0);
    let wet = args.wet * 10.0;
    // Pad the dry side so the IR tail isn't cut at the source EOF.
    let fc = format!(
        "[0:a]apad=pad_dur={tail:.3}[d];[d][1:a]afir=dry={:.2}:wet={wet:.2}[aout]",
        1.0 - args.wet
    );
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-i"]);
    argv.push(ir);
    argv.extend(["-filter_complex", &fc, "-map", "0:v?", "-map", "[aout]"]);
    if probe.has_video {
        argv.extend(["-c:v", "copy"]);
    }
    argv.extend(["-c:a", "aac"]);
    argv.push(&args.output);

    let c = engine::write_job("reverb", &[&args.input, ir], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "ir": ir,
        "tail": tail,
        "wet": args.wet,
        "filter": "afir",
    })))
}

use serde_json::json;

use crate::cli::{Globals, RiserArgs, ThumpArgs, WhooshArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::probe::Probe;

struct AccentSpec<'a> {
    tool: &'a str,
    input: &'a std::path::Path,
    output: &'a std::path::Path,
    probe: &'a Probe,
    gen: String,
    delay_ms: u64,
    extra: serde_json::Value,
}

fn finish_audio(spec: AccentSpec<'_>, g: &Globals) -> Result<Contract, Error> {
    let AccentSpec {
        tool,
        input,
        output,
        probe,
        gen,
        delay_ms,
        extra,
    } = spec;
    // gen = lavfi source for input 1; delayed stereo, amixed under the dry track
    let fc = if probe.has_audio {
        format!(
            "[1:a]aformat=channel_layouts=stereo,adelay={ms}:all=1[fx];\
             [0:a][fx]amix=inputs=2:duration=first:normalize=0[aout]",
            ms = delay_ms
        )
    } else {
        format!(
            "[1:a]aformat=channel_layouts=stereo,adelay={ms}:all=1[aout]",
            ms = delay_ms
        )
    };
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(input.to_str().unwrap());
    argv.extend(["-f", "lavfi", "-i", &gen]);
    argv.extend([
        "-filter_complex",
        &fc,
        "-map",
        "0:v?",
        "-map",
        "[aout]",
        "-c:a",
        "aac",
    ]);
    if probe.has_video {
        argv.extend(["-c:v", "copy"]);
    }
    argv.push(output.to_str().unwrap());

    let c = engine::write_job(tool, &[input], output, vec![argv], g)?;
    Ok(c.with_extra(extra))
}

/// Sub-bass drop at --at: 55Hz thump with a fast decay envelope.
pub fn thump(args: ThumpArgs, g: &Globals) -> Result<Contract, Error> {
    if !(20..=120).contains(&args.freq) {
        return Err(Error::input("--freq must be 20..=120 Hz"));
    }
    if !(0.05..=1.0).contains(&args.gain) {
        return Err(Error::input("--gain must be 0.05..=1.0"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    let at = args.at.unwrap_or(0.5);
    let gen = format!(
        "sine=frequency={f}:duration={d:.3}:sample_rate=44100,volume=eval=frame:volume='{g:.3}*8.0*exp(-t*10)'",
        f = args.freq,
        d = args.dur,
        g = args.gain
    );
    finish_audio(
        AccentSpec {
            tool: "thump",
            input: &args.input,
            output: &args.output,
            probe: &probe,
            gen,
            delay_ms: (at * 1000.0) as u64,
            extra: json!({ "at": at, "freq": args.freq }),
        },
        g,
    )
}

/// Rising tonal chirp that lands on --at (sweep ends there).
pub fn riser(args: RiserArgs, g: &Globals) -> Result<Contract, Error> {
    if !(0.2..=10.0).contains(&args.dur) {
        return Err(Error::input("--dur must be 0.2..=10 seconds"));
    }
    if !(0.05..=1.0).contains(&args.gain) {
        return Err(Error::input("--gain must be 0.05..=1.0"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    let at = args.at.unwrap_or(0.5);
    let start = (at - args.dur).max(0.0);
    let gen = format!(
        "aevalsrc=exprs='sin(2*PI*(200+1800*t/{d:.3})*t)':duration={d:.3}:sample_rate=44100,\
         volume=eval=frame:volume='{g:.3}*min(t/{d:.3},1)'",
        d = args.dur,
        g = args.gain
    );
    finish_audio(
        AccentSpec {
            tool: "riser",
            input: &args.input,
            output: &args.output,
            probe: &probe,
            gen,
            delay_ms: (start * 1000.0) as u64,
            extra: json!({ "at": at, "dur": args.dur }),
        },
        g,
    )
}

/// Airy noise swell that lands on --at (banded brown noise ramp).
pub fn whoosh(args: WhooshArgs, g: &Globals) -> Result<Contract, Error> {
    if !(0.2..=10.0).contains(&args.dur) {
        return Err(Error::input("--dur must be 0.2..=10 seconds"));
    }
    if !(0.05..=1.0).contains(&args.gain) {
        return Err(Error::input("--gain must be 0.05..=1.0"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    let at = args.at.unwrap_or(0.5);
    let start = (at - args.dur).max(0.0);
    let gen = format!(
        "anoisesrc=color=brown:duration={d:.3}:amplitude=1.0,highpass=f=350,\
         volume=eval=frame:volume='{g:.3}*min(t/{d:.3},1)*exp(-max(t-{c:.3},0)*14)'",
        d = args.dur,
        g = args.gain,
        c = args.dur * 0.85
    );
    finish_audio(
        AccentSpec {
            tool: "whoosh",
            input: &args.input,
            output: &args.output,
            probe: &probe,
            gen,
            delay_ms: (start * 1000.0) as u64,
            extra: json!({ "at": at, "dur": args.dur }),
        },
        g,
    )
}

use serde_json::json;

use crate::cli::{Globals, WaveformArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: WaveformArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_audio {
        return Err(Error::input("waveform: input has no audio stream"));
    }
    let (w, h) = parse_size(&args.size)?;

    let raw = args.color.as_deref().unwrap_or("ffffff");
    // ffmpeg colour spec wants 0xRRGGBB; bare hex is ambiguous.
    let color = crate::color::lavfi(raw);
    let color = color.as_str();
    let sc = match &args.scale {
        Some(s) => {
            if !["lin", "log", "sqrt", "cbrt"].contains(&s.as_str()) {
                return Err(Error::input("--scale must be lin|log|sqrt|cbrt"));
            }
            format!(":scale={s}")
        }
        None => String::new(),
    };
    // --at/--dur: crop the rendered wave to the window, then stretch to --size.
    let flt = if args.peak { ":filter=peak" } else { "" };
    let sp = if args.split { ":split_channels=1" } else { "" };
    let win = match &args.at {
        Some(raw) => {
            let at = crate::time::parse_time(raw)?;
            if !(0.0..probe.duration).contains(&at) {
                return Err(Error::input("--at is outside the input"));
            }
            let end = args
                .dur
                .map(|d| at + d)
                .unwrap_or(probe.duration)
                .min(probe.duration);
            format!(
                ";[w0]crop=w=iw*{fw:.6}:x=iw*{fx:.6}:h=ih,scale={w}:{h}[v]",
                fw = (end - at) / probe.duration,
                fx = at / probe.duration
            )
        }
        None => {
            if args.dur.is_some() {
                return Err(Error::input("--dur needs --at"));
            }
            ";[w0]copy[v]".to_string()
        }
    };
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend([
        "-filter_complex",
        &format!("[0:a]showwavespic=s={w}x{h}:colors={color}{flt}{sp}{sc}[w0]{win}"),
        "-map",
        "[v]",
        "-frames:v",
        "1",
        "-update",
        "1",
    ]);
    argv.push(&args.output);

    let c = engine::write_job("waveform", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "size": format!("{w}x{h}"),
        "color": raw,
    })))
}

fn parse_size(s: &str) -> Result<(u32, u32), Error> {
    let (w, h) = s
        .split_once('x')
        .ok_or_else(|| Error::input("--size must look like WxH (e.g. 1920x540)"))?;
    let w: u32 = w.parse().map_err(|_| Error::input("--size WxH integers"))?;
    let h: u32 = h.parse().map_err(|_| Error::input("--size WxH integers"))?;
    if !(16..=8192).contains(&w) || !(16..=8192).contains(&h) {
        return Err(Error::input("--size sides must be 16..8192"));
    }
    Ok((w, h))
}

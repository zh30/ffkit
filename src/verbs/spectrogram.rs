use serde_json::json;

use crate::cli::{Globals, SpectrogramArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: SpectrogramArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_audio {
        return Err(Error::input("spectrogram: input has no audio stream"));
    }
    let color = match &args.color {
        Some(c) => {
            let ok = [
                "channel",
                "intensity",
                "rainbow",
                "moreland",
                "nebulae",
                "fire",
                "fiery",
                "fruit",
                "cool",
                "magma",
                "green",
                "viridis",
                "plasma",
                "cividis",
                "terrain",
            ];
            if !ok.contains(&c.as_str()) {
                return Err(Error::input(format!(
                    "--color: use one of {}",
                    ok.join("|")
                )));
            }
            format!(":color={c}")
        }
        None => String::new(),
    };
    let (w, h) = args
        .size
        .split_once('x')
        .and_then(|(w, h)| w.parse::<u32>().ok().zip(h.parse::<u32>().ok()))
        .filter(|(w, h)| (16..=8192).contains(w) && (16..=8192).contains(h))
        .ok_or_else(|| Error::input("--size must look like WxH (e.g. 1920x1080)"))?;

    let slice = match &args.at {
        Some(raw) => {
            let at = crate::time::parse_time(raw)?;
            match args.dur {
                Some(d) => format!("atrim={at:.3}:{e:.3},asetpts=PTS-STARTPTS,", e = at + d),
                None => format!("atrim=start={at:.3},asetpts=PTS-STARTPTS,"),
            }
        }
        None => {
            if args.dur.is_some() {
                return Err(Error::input("--dur needs --at"));
            }
            String::new()
        }
    };
    let sc = match &args.scale {
        Some(s) => {
            if !["lin", "sqrt", "cbrt", "log", "4thrt", "5thrt"].contains(&s.as_str()) {
                return Err(Error::input("--scale: lin|sqrt|cbrt|log|4thrt|5thrt"));
            }
            format!(":scale={s}")
        }
        None => String::new(),
    };
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend([
        "-filter_complex",
        &format!("[0:a]{slice}showspectrumpic=s={w}x{h}:legend=1{color}{sc}[v]"),
        "-map",
        "[v]",
        "-frames:v",
        "1",
        "-update",
        "1",
    ]);
    argv.push(&args.output);

    let c = engine::write_job("spectrogram", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({ "size": format!("{w}x{h}") })))
}

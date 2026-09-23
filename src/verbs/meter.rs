use serde_json::json;

use crate::cli::{Globals, MeterArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

/// Live EBU R128 loudness meter as a video (podcast/voice QC: watch I/TP/LRA
/// bars while the audio plays back). `ebur128 video=1` renders the meter UI;
/// the original audio is kept so the clip doubles as a listening check.
pub fn run(args: MeterArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_audio {
        return Err(Error::input("meter: input has no audio stream"));
    }
    let (w, h) = parse_size(&args.size)?;
    if !(9..=18).contains(&args.meter) {
        return Err(Error::input("--meter must be 9..=18 (EBU scale)"));
    }

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend([
        "-filter_complex",
        &format!(
            "[0:a]ebur128=video=1:size={w}x{h}:meter={m}[v]",
            m = args.meter
        ),
        "-map",
        "[v]",
        "-map",
        "0:a",
        "-c:v",
        "libx264",
        "-preset",
        "veryfast",
        "-crf",
        "22",
        "-pix_fmt",
        "yuv420p",
        "-c:a",
        "aac",
        "-b:a",
        "128k",
        "-shortest",
    ]);
    argv.push(&args.output);

    let c = engine::write_job("meter", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "size": format!("{w}x{h}"),
        "meter": args.meter,
        "has_audio": true,
    })))
}

fn parse_size(s: &str) -> Result<(u32, u32), Error> {
    let (w, h) = s
        .split_once('x')
        .ok_or_else(|| Error::input("--size must look like WxH (e.g. 640x480)"))?;
    let w: u32 = w.parse().map_err(|_| Error::input("--size WxH integers"))?;
    let h: u32 = h.parse().map_err(|_| Error::input("--size WxH integers"))?;
    if !(16..=8192).contains(&w) || !(16..=8192).contains(&h) {
        return Err(Error::input("--size sides must be 16..8192"));
    }
    Ok((w, h))
}

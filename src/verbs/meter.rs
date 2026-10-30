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

    // ebur128 draws one meter frame per 100 ms after a 100 ms warm-up, so
    // a window under ~0.3s renders nothing. Hold the window's end and pull
    // its start back instead of writing an empty video.
    const MIN_WINDOW: f64 = 0.3;
    let dur = match args.dur {
        Some(d) if d <= 0.0 => return Err(Error::input("--dur must be > 0")),
        Some(d) => Some(d.max(MIN_WINDOW)),
        None => None,
    };
    let start = match &args.at {
        Some(raw) => {
            let at = crate::time::resolve_frame_at(raw.trim(), probe.duration)?.max(0.0);
            let window = dur.unwrap_or(MIN_WINDOW);
            Some(at.min((probe.duration - window).max(0.0)))
        }
        None => None,
    };

    let mut argv = ffmpeg_base(g.progress);
    if let Some(at) = start {
        argv.extend(["-ss", &crate::time::fmt_time(at)]);
    }
    argv.push("-i");
    argv.push(&args.input);
    let meter = format!("ebur128=video=1:size={w}x{h}:meter={m}", m = args.meter);
    // the window is cut inside the graph: an output -t with -shortest
    // spins ffmpeg 9 forever when ebur128 has emitted no frame yet
    let (graph, amap) = match dur {
        Some(d) => (
            format!("[0:a]atrim=duration={d:.3},asetpts=PTS-STARTPTS,asplit[a][m];[m]{meter}[v]"),
            "[a]",
        ),
        None => (format!("[0:a]{meter}[v]"), "0:a"),
    };
    argv.extend([
        "-filter_complex",
        graph.as_str(),
        "-map",
        "[v]",
        "-map",
        amap,
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
        "start": start,
        "dur": dur,
    })))
}

pub fn parse_size(s: &str) -> Result<(u32, u32), Error> {
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

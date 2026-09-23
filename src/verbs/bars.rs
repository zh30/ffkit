use serde_json::json;

use crate::cli::{BarsArgs, Globals};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: BarsArgs, g: &Globals) -> Result<Contract, Error> {
    if !(0.1..=600.0).contains(&args.dur) {
        return Err(Error::input("--dur must be 0.1..=600 seconds"));
    }
    let (w, h) = args
        .size
        .split_once('x')
        .and_then(|(a, b)| a.parse::<u32>().ok().zip(b.parse::<u32>().ok()))
        .ok_or_else(|| Error::input("--size must be WxH"))?;
    if w == 0 || h == 0 || w > 7680 || h > 4320 {
        return Err(Error::input("--size out of range"));
    }

    let src = if args.hd { "smptehdbars" } else { "smptebars" };
    let mut argv = ffmpeg_base(g.progress);
    argv.extend(["-f", "lavfi", "-i"]);
    argv.push(format!(
        "{src}=size={w}x{h}:duration={:.3}:rate=30",
        args.dur
    ));
    if args.tone {
        argv.extend(["-f", "lavfi", "-i"]);
        argv.push(format!("sine=frequency=1000:duration={:.3}", args.dur));
        argv.extend(["-map", "0:v", "-map", "1:a", "-c:a", "aac", "-b:a", "96k"]);
    } else {
        argv.extend(["-map", "0:v"]);
    }
    argv.extend([
        "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
    ]);
    argv.push(&args.output);

    let c2 = engine::write_job("bars", &[], &args.output, vec![argv], g)?;
    Ok(c2.with_extra(json!({
        "size": args.size,
        "hd": args.hd,
        "tone": args.tone,
    })))
}

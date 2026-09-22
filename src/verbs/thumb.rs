use serde_json::json;

use crate::cli::{Globals, ThumbArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::time::{fmt_time, parse_time};

pub fn run(args: ThumbArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_video {
        return Err(Error::input("thumb: input has no video stream"));
    }
    let ext = args
        .output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !matches!(ext.as_str(), "jpg" | "jpeg" | "png" | "webp") {
        return Err(Error::input(format!(
            "thumb output must be .jpg/.png/.webp, got .{ext}"
        )));
    }
    if args.at.is_some() && args.frame.is_some() {
        return Err(Error::input("pass --at or --frame, not both"));
    }

    let mut argv = ffmpeg_base(g.progress);
    match (args.at.as_deref(), args.frame) {
        (Some(at), None) => {
            let secs = parse_time(at)?;
            // seek before input: fast + frame-accurate enough for a cover grab
            argv.extend(["-ss", &fmt_time(secs)]);
            argv.push("-i");
            argv.push(&args.input);
        }
        (None, Some(f)) => {
            argv.push("-i");
            argv.push(&args.input);
            argv.extend(["-vf", &format!("select='eq(n,{f})'")]);
        }
        (None, None) => {
            let at = probe.duration * 0.1;
            argv.extend(["-ss", &fmt_time(at)]);
            argv.push("-i");
            argv.push(&args.input);
        }
        _ => unreachable!(),
    }
    if let Some(w) = args.width {
        argv.extend(["-vf", &format!("scale={w}:-2")]);
    }
    argv.extend(["-frames:v", "1"]);
    if matches!(ext.as_str(), "jpg" | "jpeg") {
        argv.extend(["-q:v", "2"]);
    }
    argv.push(&args.output);

    let c = engine::write_job("thumb", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "at": args.at.clone(),
        "frame": args.frame,
        "width": probe.width,
        "height": probe.height,
    })))
}

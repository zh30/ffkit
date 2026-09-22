use serde_json::json;

use crate::cli::{CoverArgs, Globals};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::time::{fmt_time, parse_time};

const FRAME_W: u32 = 1080;
const FRAME_H: u32 = 1920;

pub fn run(args: CoverArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "cover")?;
    let at = match &args.at {
        Some(s) => parse_time(s)?,
        None => 0.0,
    };
    if at > probe.duration && probe.duration > 0.0 {
        return Err(Error::input(format!(
            "--at {at} is past duration {:.3}s",
            probe.duration
        )));
    }

    let vf = if args.blur {
        format!(
            "split[a][b];[a]scale={FRAME_W}:{FRAME_H}:force_original_aspect_ratio=increase,crop={FRAME_W}:{FRAME_H},gblur=sigma=40[bg];[b]scale={FRAME_W}:{FRAME_H}:force_original_aspect_ratio=decrease[fg];[bg][fg]overlay=(W-w)/2:(H-h)/2,setsar=1,format=yuv420p"
        )
    } else {
        format!(
            "scale={FRAME_W}:{FRAME_H}:force_original_aspect_ratio=decrease,pad={FRAME_W}:{FRAME_H}:(ow-iw)/2:(oh-ih)/2:black,setsar=1,format=yuv420p"
        )
    };
    let mut argv = ffmpeg_base(g.progress);
    if at > 0.0 {
        argv.extend(["-ss", &fmt_time(at)]);
    }
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-frames:v", "1", "-an", "-vf", &vf]);
    argv.push(&args.output);

    let c = engine::write_job("cover", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "at": at,
        "frame": format!("{FRAME_W}x{FRAME_H}"),
    })))
}

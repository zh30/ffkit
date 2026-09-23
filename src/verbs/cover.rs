use serde_json::json;

use crate::cli::{CoverArgs, Globals};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::time::fmt_time;

pub fn run(args: CoverArgs, g: &Globals) -> Result<Contract, Error> {
    let (frame_w, frame_h) = match &args.size {
        Some(s) => s
            .split_once('x')
            .and_then(|(a, b)| Some((a.parse::<u32>().ok()?, b.parse::<u32>().ok()?)))
            .filter(|(w, h)| *w >= 16 && *h >= 16 && *w % 2 == 0 && *h % 2 == 0)
            .ok_or_else(|| Error::input("--size must be even WxH (min 16x16)"))?,
        None => (1080, 1920),
    };
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "cover")?;
    let at = match &args.at {
        Some(s) => crate::time::resolve_frame_at(s, probe.duration)?,
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
            "split[a][b];[a]scale={frame_w}:{frame_h}:force_original_aspect_ratio=increase,crop={frame_w}:{frame_h},gblur=sigma=40[bg];[b]scale={frame_w}:{frame_h}:force_original_aspect_ratio=decrease[fg];[bg][fg]overlay=(W-w)/2:(H-h)/2,setsar=1,format=yuv420p"
        )
    } else {
        format!(
            "scale={frame_w}:{frame_h}:force_original_aspect_ratio=decrease,pad={frame_w}:{frame_h}:(ow-iw)/2:(oh-ih)/2:black,setsar=1,format=yuv420p"
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
        "frame": format!("{frame_w}x{frame_h}"),
    })))
}

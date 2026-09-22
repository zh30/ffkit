use serde_json::json;

use crate::cli::{DelogoArgs, Globals};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: DelogoArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "delogo")?;
    let (w, h) = (
        probe.width.unwrap_or(0) as i64,
        probe.height.unwrap_or(0) as i64,
    );
    if args.w < 4 || args.h < 4 {
        return Err(Error::input("--w/--h must be >= 4 px"));
    }
    let (x1, y1) = (args.x as i64 + args.w as i64, args.y as i64 + args.h as i64);
    if x1 > w || y1 > h {
        return Err(Error::input(format!(
            "logo box {}x{}@{}+{} exceeds {}x{} frame",
            args.w, args.h, args.x, args.y, w, h
        )));
    }
    let mut vf = format!("delogo=x={}:y={}:w={}:h={}", args.x, args.y, args.w, args.h);
    match (&args.at, args.dur) {
        (Some(at), dur) => {
            let start = crate::time::parse_time(at)?;
            if !(0.0..probe.duration).contains(&start) {
                return Err(Error::input("--at is outside the input"));
            }
            match dur.map(|d| start + d) {
                Some(e) if e < probe.duration => {
                    vf.push_str(&format!(":enable='between(t,{start:.3},{e:.3})'"));
                }
                _ => vf.push_str(&format!(":enable='gte(t,{start:.3})'")),
            }
        }
        (None, Some(_)) => return Err(Error::input("--dur needs --at")),
        (None, None) => {}
    }
    let fc = format!("[0:v]{vf}[vout]");
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-filter_complex", &fc, "-map", "[vout]"]);
    if probe.has_audio {
        argv.extend(["-map", "0:a?", "-c:a", "copy"]);
    }
    argv.extend([
        "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
    ]);
    argv.push(&args.output);

    let c = engine::write_job("delogo", &[&args.input], &args.output, vec![argv], g)?;
    let mut extra = json!({
        "box": {"x": args.x, "y": args.y, "w": args.w, "h": args.h},
    });
    if let Some(at) = &args.at {
        extra["at"] = json!(at);
        if let Some(d) = args.dur {
            extra["dur"] = json!(d);
        }
    }
    Ok(c.with_extra(extra))
}

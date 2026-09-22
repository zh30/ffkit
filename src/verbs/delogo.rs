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
    let vf = format!("delogo=x={}:y={}:w={}:h={}", args.x, args.y, args.w, args.h);
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
    Ok(c.with_extra(json!({
        "box": {"x": args.x, "y": args.y, "w": args.w, "h": args.h},
    })))
}

use serde_json::json;
use std::path::Path;

use crate::cli::{Globals, RepairArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::paths;

/// Swap a stretch of bad/glitched frames for one clean frame from a
/// reference take (`freezeframes`). Times are seconds; the filter's frame
/// indices come from each clip's own fps.
pub fn run(args: RepairArgs, g: &Globals) -> Result<Contract, Error> {
    let p = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&p, "repair")?;
    paths::ensure_input(&args.ref_file)?;
    let r = engine::probe_or_err(&args.ref_file, g)?;
    if !r.has_video {
        return Err(Error::input("repair: --ref has no picture"));
    }
    let fps = p.fps.unwrap_or(25.0);
    let rfps = r.fps.unwrap_or(25.0);

    let at = crate::time::resolve_frame_at(&args.at, p.duration)?;
    if !(0.0..p.duration).contains(&at) {
        return Err(Error::input(format!(
            "repair --at {at} is outside the {:.2}s source",
            p.duration
        )));
    }
    let dur = args.dur.unwrap_or(1.0 / fps);
    let ref_at = match &args.ref_at {
        Some(s) => crate::time::resolve_frame_at(s, r.duration)?,
        None => at,
    };
    if !(0.0..r.duration).contains(&ref_at) {
        return Err(Error::input(format!(
            "repair --ref-at {ref_at} is outside the {:.2}s reference",
            r.duration
        )));
    }

    let first = (at * fps).round() as i64;
    let last = ((at + dur) * fps).round() as i64 - 1;
    let last = last.max(first);
    let replace = (ref_at * rfps).round() as i64;

    let fc = format!("[0:v][1:v]freezeframes=first={first}:last={last}:replace={replace}[v]");
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.push("-i");
    argv.push(&args.ref_file);
    argv.extend(["-filter_complex", &fc, "-map", "[v]", "-map", "0:a?"]);
    argv.extend([
        "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
    ]);
    if p.has_audio {
        argv.extend(["-c:a", "copy"]);
    }
    argv.push(&args.output);

    let inputs: Vec<&Path> = vec![&args.input, &args.ref_file];
    let c = engine::write_job("repair", &inputs, &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "first_frame": first,
        "last_frame": last,
        "replace_frame": replace,
        "fps": fps,
    })))
}

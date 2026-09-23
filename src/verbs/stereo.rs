use serde_json::json;

use crate::cli::{Globals, StereoArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

const FORMATS: &[&str] = &[
    "sbsl", "sbsr", "sbs2l", "sbs2r", "abl", "abr", "ab2l", "ab2r", "tbl", "tbr", "tb2l", "tb2r",
    "al", "ar", "irl", "irr", "icl", "icr", "arcg", "arch", "arcc", "arcd", "arbg", "agmg", "agmh",
    "agmc", "agmd",
];
const OUT_ONLY: &[&str] = &[
    "arcg", "arch", "arcc", "arcd", "arbg", "agmg", "agmh", "agmc", "agmd",
];

pub fn run(args: StereoArgs, g: &Globals) -> Result<Contract, Error> {
    if !FORMATS.contains(&args.in_format.as_str()) || OUT_ONLY.contains(&args.in_format.as_str()) {
        return Err(Error::input(format!(
            "--in '{fmt}' isn't a packed 3D format (anaglyphs are output-only)",
            fmt = args.in_format
        )));
    }
    if !FORMATS.contains(&args.out_format.as_str()) {
        return Err(Error::input(format!(
            "--out '{fmt}' isn't a stereo3d format",
            fmt = args.out_format
        )));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "stereo")?;

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    let vf = format!("stereo3d=in={}:out={}", args.in_format, args.out_format);
    argv.extend([
        "-vf", &vf, "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
    ]);
    if probe.has_audio {
        argv.extend(["-c:a", "copy"]);
    }
    argv.push(&args.output);

    let c = engine::write_job("stereo", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "in": args.in_format,
        "out": args.out_format,
    })))
}

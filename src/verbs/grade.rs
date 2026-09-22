use serde_json::json;

use crate::cli::{Globals, GradeArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::paths;

pub fn run(args: GradeArgs, g: &Globals) -> Result<Contract, Error> {
    if !(0.2..=3.0).contains(&args.contrast) {
        return Err(Error::input("--contrast must be 0.2..=3"));
    }
    if !(0.0..=3.0).contains(&args.saturation) {
        return Err(Error::input("--saturation must be 0..=3"));
    }
    if !(-0.5..=0.5).contains(&args.brightness) {
        return Err(Error::input("--brightness must be -0.5..=0.5"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "grade")?;
    if let Some(lut) = &args.lut {
        paths::ensure_input(lut)?;
    }

    let mut vf = format!(
        "eq=contrast={}:brightness={}:saturation={}",
        args.contrast, args.brightness, args.saturation
    );
    if let Some(lut) = &args.lut {
        // Single quotes group literal path text; escape internal quotes.
        let esc = lut.display().to_string().replace('\'', "\\'");
        vf.push_str(&format!(",lut3d=file='{esc}'"));
    }
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend([
        "-vf", &vf, "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
    ]);
    if probe.has_audio {
        argv.extend(["-c:a", "copy"]);
    }
    argv.push(&args.output);

    let mut inputs: Vec<&std::path::Path> = vec![&args.input];
    if let Some(lut) = &args.lut {
        inputs.push(lut);
    }
    let c = engine::write_job("grade", &inputs, &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "contrast": args.contrast,
        "saturation": args.saturation,
        "brightness": args.brightness,
        "lut": args.lut,
    })))
}

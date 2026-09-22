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
    if args.grain < 0.0 {
        return Err(Error::input("--grain must be >= 0"));
    }
    if !(-1.0..=1.0).contains(&args.warm) {
        return Err(Error::input("--warm must be -1..=1"));
    }

    let mut vf = match args.preset {
        Some(crate::cli::GradePreset::Cinematic) => {
            String::from("eq=contrast=1.12:saturation=0.88,colorbalance=bs=0.1:bm=0.06")
        }
        Some(crate::cli::GradePreset::Vivid) => String::from("eq=saturation=1.45:contrast=1.12"),
        Some(crate::cli::GradePreset::Vintage) => String::from("curves=vintage"),
        Some(crate::cli::GradePreset::Soft) => {
            String::from("eq=contrast=0.92:brightness=0.04:saturation=0.95")
        }
        None => String::new(),
    };
    if !vf.is_empty() {
        vf.push(',');
    }
    vf.push_str(&format!(
        "eq=contrast={}:brightness={}:saturation={}:gamma={}",
        args.contrast, args.brightness, args.saturation, args.gamma
    ));
    if args.hue != 0.0 {
        vf.push_str(&format!(",hue=h={}", args.hue.clamp(-180.0, 180.0)));
    }
    if let Some(lut) = &args.lut {
        // Single quotes group literal path text; escape internal quotes.
        let esc = lut.display().to_string().replace('\'', "\\'");
        vf.push_str(&format!(",lut3d=file='{esc}'"));
    }
    if args.warm != 0.0 {
        let k = 6500.0 - args.warm * 3500.0;
        vf.push_str(&format!(",colortemperature=temperature={k:.0}"));
    }
    if args.grain > 0.0 {
        vf.push_str(&format!(",noise=alls={}:allf=t+u", args.grain.min(30.0)));
    }
    if let Some(s) = &args.at {
        let at = crate::time::parse_time(s)?;
        if !(0.0..probe.duration).contains(&at) {
            return Err(Error::input("--at is outside the input"));
        }
        let win = match args.dur {
            Some(d) => format!("between(t,{at:.3},{:.3})", at + d),
            None => format!("gte(t,{at:.3})"),
        };
        vf = vf
            .split(',')
            .map(|seg| format!("{seg}:enable='{win}'"))
            .collect::<Vec<_>>()
            .join(",");
    } else if args.dur.is_some() {
        return Err(Error::input("--dur needs --at"));
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
        "grain": args.grain,
        "warm": args.warm,
        "hue": args.hue,
    })))
}

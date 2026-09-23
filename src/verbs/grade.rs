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
        Some(crate::cli::GradePreset::Sepia) => {
            String::from("colorchannelmixer=.393:.769:.189:0:.349:.686:.168:0:.272:.534:.131")
        }
        // teal shadows + warm mids (bs/bm only — 4.4 lacks ms)
        Some(crate::cli::GradePreset::Teal) => {
            String::from("eq=contrast=1.08:saturation=1.12,colorbalance=bs=0.10:bm=-0.06")
        }
        // desat lands at the chain tail so the default eq sliders can't
        // re-add saturation
        Some(crate::cli::GradePreset::Noir) => String::from("eq=contrast=1.28:brightness=-0.02"),
        Some(crate::cli::GradePreset::Bleach) => {
            String::from("eq=contrast=1.30:brightness=0.02:saturation=0.55,colorbalance=bs=0.04")
        }
        Some(crate::cli::GradePreset::Crossprocess) => String::from("curves=preset=cross_process"),
        Some(crate::cli::GradePreset::Strongcontrast) => {
            String::from("curves=preset=strong_contrast")
        }
        Some(crate::cli::GradePreset::Linearcontrast) => {
            String::from("curves=preset=linear_contrast")
        }
        Some(crate::cli::GradePreset::Neon) => String::from(
            "eq=contrast=1.15:saturation=1.5,colorbalance=bs=0.20:gs=0.10:rh=0.15:bh=0.15",
        ),
        None => String::new(),
    };
    if !vf.is_empty() {
        vf.push(',');
    }
    vf.push_str(&format!(
        "eq=contrast={}:brightness={}:saturation={}:gamma={}",
        args.contrast, args.brightness, args.saturation, args.gamma
    ));
    if let Some(ev) = args.exposure {
        if !(-3.0..=3.0).contains(&ev) {
            return Err(Error::input("--exposure must be -3..=3 (EV stops)"));
        }
        if ev != 0.0 {
            vf.push_str(&format!(",exposure=exposure={ev}"));
        }
    }
    if args.hue != 0.0 {
        vf.push_str(&format!(",hue=h={}", args.hue.clamp(-180.0, 180.0)));
    }
    if matches!(args.preset, Some(crate::cli::GradePreset::Noir)) {
        vf.push_str(",hue=s=0");
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
        let win = crate::time::enable_expr(s, args.dur, probe.duration)?;
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

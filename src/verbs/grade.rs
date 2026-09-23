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
    if !(-1.0..=1.0).contains(&args.vibrance) {
        return Err(Error::input("--vibrance must be -1..=1"));
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
    if args.vibrance != 0.0 {
        vf.push_str(&format!(",vibrance=intensity={:.3}", args.vibrance * 2.0));
        if let Some(wash) = &args.wash {
            let (hue, sat) = crate::color::hsl(wash)?;
            let amt = args.wash_amount.clamp(0.0, 1.0);
            if !(0.0..=1.0).contains(&args.wash_amount) {
                return Err(Error::input("--wash-amount must be 0..1"));
            }
            vf.push_str(&format!(
                ",colorize=hue={hue:.1}:saturation={:.3}:lightness={:.3}",
                (sat * amt).min(1.0),
                amt * 0.2
            ));
        }
    }
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
    let hald_lut = args
        .lut
        .as_ref()
        .filter(|p| {
            matches!(
                p.extension().and_then(|e| e.to_str()),
                Some("png") | Some("jpg") | Some("jpeg") | Some("webp")
            )
        })
        .cloned();
    if let Some(lut) = &args.lut {
        if hald_lut.is_none() {
            // Single quotes group literal path text; escape internal quotes.
            let esc = lut.display().to_string().replace('\'', "\\'");
            vf.push_str(&format!(",lut3d=file='{esc}'"));
        }
    }
    if args.skin != 0.0 {
        let w = args.skin.clamp(-1.0, 1.0);
        vf.push_str(&format!(
            ",selectivecolor=reds='0 {:.3} {:.3} 0'",
            0.15 * w,
            0.25 * w
        ));
    }
    if args.kelvin.is_some() && args.warm != 0.0 {
        return Err(Error::input(
            "--kelvin and --warm are two dials for the same knob — pick one",
        ));
    }
    if let Some(k) = args.kelvin {
        let k = k.clamp(1000.0, 40000.0);
        vf.push_str(&format!(",colortemperature=temperature={k:.0}"));
    } else if args.warm != 0.0 {
        let k = 6500.0 - args.warm * 3500.0;
        vf.push_str(&format!(",colortemperature=temperature={k:.0}"));
    }
    if let Some(c) = &args.curve {
        let pts = c.replace(['\'', '"', ';', ':', '='], "");
        if pts.is_empty() {
            return Err(Error::input(
                "--curve needs points like \"0/0 0.5/0.7 1/1\"",
            ));
        }
        vf.push_str(&format!(",curves=master='{pts}'"));
    }
    if let Some(sp) = args.split {
        let sp = sp.clamp(-1.0, 1.0);
        // teal shadows + orange highlights; negative flips the pair
        vf.push_str(&format!(",colorcorrect=bl={:.3}:rh={:.3}", sp, sp * 0.7));
    }
    if args.grain > 0.0 {
        vf.push_str(&format!(",noise=alls={}:allf=t+u", args.grain.min(30.0)));
    }
    let win_expr = match &args.at {
        Some(s) => Some(crate::time::enable_expr(s, args.dur, probe.duration)?),
        None => {
            if args.dur.is_some() {
                return Err(Error::input("--dur needs --at"));
            }
            None
        }
    };
    if let Some(win) = &win_expr {
        vf = vf
            .split(',')
            .map(|seg| format!("{seg}:enable='{win}'"))
            .collect::<Vec<_>>()
            .join(",");
    }
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    if let Some(lut) = &hald_lut {
        argv.push("-i");
        argv.push(lut);
        let hald = match &win_expr {
            Some(w) => format!("haldclut:enable='{w}'"),
            None => "haldclut".to_string(),
        };
        let fc = format!("[0:v]{vf}[g];[g][1:v]{hald}[out]");
        argv.extend(["-filter_complex", &fc, "-map", "[out]", "-map", "0:a?"]);
    } else {
        argv.extend(["-vf", &vf, "-map", "0:v", "-map", "0:a?"]);
    }
    argv.extend([
        "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
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
        "lut_engine": if hald_lut.is_some() { "haldclut" } else if args.lut.is_some() { "lut3d" } else { "none" },
        "skin": args.skin,
        "grain": args.grain,
        "warm": args.warm,
        "hue": args.hue,
    })))
}

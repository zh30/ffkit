use serde_json::json;

use crate::cli::{CropArgs, Globals};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

fn parse_region(s: &str) -> Result<(u32, u32, u32, u32), Error> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 4 {
        return Err(Error::input(
            "--region must look like x:y:w:h (e.g. 0:0:1280:720)",
        ));
    }
    let v: Vec<u32> = parts
        .iter()
        .map(|p| {
            p.trim()
                .parse::<u32>()
                .map_err(|_| Error::input("--region takes integers: x:y:w:h"))
        })
        .collect::<Result<_, _>>()?;
    if v[2] < 16 || v[3] < 16 {
        return Err(Error::input("--region w/h must be at least 16px"));
    }
    Ok((v[0], v[1], v[2], v[3]))
}

fn parse_aspect(s: &str) -> Result<f64, Error> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 {
        return Err(Error::input("--aspect must look like W:H (e.g. 1:1, 9:16)"));
    }
    let w: f64 = parts[0]
        .trim()
        .parse()
        .map_err(|_| Error::input("--aspect takes numbers: W:H"))?;
    let h: f64 = parts[1]
        .trim()
        .parse()
        .map_err(|_| Error::input("--aspect takes numbers: W:H"))?;
    if w <= 0.0 || h <= 0.0 {
        return Err(Error::input("--aspect sides must be > 0"));
    }
    Ok(w / h)
}

pub fn run(args: CropArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "crop")?;
    let (iw, ih) = (probe.width.unwrap_or(0), probe.height.unwrap_or(0));
    if iw == 0 || ih == 0 {
        return Err(Error::input("could not read input dimensions"));
    }

    let (w, h, x, y) = if let Some(r) = &args.region {
        let (x, y, w, h) = parse_region(r)?;
        if x + w > iw || y + h > ih {
            return Err(Error::input(format!(
                "--region {x}:{y}:{w}:{h} falls outside the {iw}x{ih} frame"
            )));
        }
        (w, h, x, y)
    } else if let Some(a) = &args.aspect {
        let ar = parse_aspect(a)?;
        // Largest centered box of the target aspect inside the frame.
        let (w, h) = if iw as f64 / ih as f64 > ar {
            ((ih as f64 * ar).round() as u32, ih)
        } else {
            (iw, (iw as f64 / ar).round() as u32)
        };
        let (w, h) = (w & !1, h & !1); // even dims for yuv420p
        let (x, y) = match args.anchor.as_str() {
            "center" => ((iw - w) / 2, (ih - h) / 2),
            "top" => ((iw - w) / 2, 0),
            "bottom" => ((iw - w) / 2, ih - h),
            "left" => (0, (ih - h) / 2),
            "right" => (iw - w, (ih - h) / 2),
            other => {
                return Err(Error::input(format!(
                    "unknown --anchor {other}; use center|top|bottom|left|right"
                )))
            }
        };
        (w, h, x, y)
    } else {
        return Err(Error::input("crop needs --region x:y:w:h or --aspect W:H"));
    };

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend([
        "-vf",
        &format!("crop={w}:{h}:{x}:{y}"),
        "-c:v",
        "libx264",
        "-preset",
        "fast",
        "-crf",
        "18",
        "-pix_fmt",
        "yuv420p",
    ]);
    if probe.has_audio {
        argv.extend(["-c:a", "copy"]);
    }
    argv.push(&args.output);

    let c = engine::write_job("crop", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "region": format!("{x}:{y}:{w}:{h}"),
        "out_size": format!("{w}x{h}"),
    })))
}

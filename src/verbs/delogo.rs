use serde_json::json;

use crate::cli::{DelogoArgs, DelogoShape, Globals};
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
    let regions = match &args.regions {
        Some(list) => {
            if args.x.is_some() || args.y.is_some() || args.w.is_some() || args.h.is_some() {
                return Err(Error::input("--regions conflicts with --x/--y/--w/--h"));
            }
            let mut v = Vec::new();
            for part in list.split(',') {
                let nums: Vec<u32> = part
                    .split(':')
                    .map(|n| {
                        n.parse()
                            .map_err(|_| Error::input(format!("--region '{part}' wants x:y:w:h")))
                    })
                    .collect::<Result<_, _>>()?;
                if nums.len() != 4 {
                    return Err(Error::input(format!("--region '{part}' wants x:y:w:h")));
                }
                v.push((nums[0], nums[1], nums[2], nums[3]));
            }
            v
        }
        None => match (args.x, args.y, args.w, args.h) {
            (Some(x), Some(y), Some(w2), Some(h2)) => vec![(x, y, w2, h2)],
            _ => {
                return Err(Error::input(
                    "need --x --y --w --h or --regions x:y:w:h[,...]",
                ))
            }
        },
    };
    for &(x, y, rw, rh) in &regions {
        if rw < 4 || rh < 4 {
            return Err(Error::input("logo box w/h must be >= 4 px"));
        }
        if x as i64 + rw as i64 > w || y as i64 + rh as i64 > h {
            return Err(Error::input(format!(
                "logo box {}x{}@{}+{} exceeds {}x{} frame",
                rw, rh, x, y, w, h
            )));
        }
    }
    let enable = match (&args.at, args.dur) {
        (Some(at), dur) => Some(format!(
            ":enable='{}'",
            crate::time::enable_expr(at, dur, probe.duration)?
        )),
        (None, Some(_)) => return Err(Error::input("--dur needs --at")),
        (None, None) => None,
    };
    // --soft: removelogo reads a PNG mask (white = remove) and interpolates
    // edges instead of boxing — gentler on gradients/sky.
    let mut mask_tmp = None;
    let circle = matches!(args.shape, DelogoShape::Circle);
    let vf = if args.soft || circle {
        let (fw, fh) = (w as u32, h as u32);
        let mut img = image::RgbaImage::from_pixel(fw, fh, image::Rgba([0, 0, 0, 255]));
        for &(x, y, rw, rh) in &regions {
            if circle {
                let (cx, cy) = (x as f64 + rw as f64 / 2.0, y as f64 + rh as f64 / 2.0);
                let (rx, ry) = (rw as f64 / 2.0, rh as f64 / 2.0);
                for yy in y..(y + rh) {
                    for xx in x..(x + rw) {
                        let dx = (xx as f64 + 0.5 - cx) / rx;
                        let dy = (yy as f64 + 0.5 - cy) / ry;
                        if dx * dx + dy * dy <= 1.0 {
                            img.put_pixel(xx, yy, image::Rgba([255, 255, 255, 255]));
                        }
                    }
                }
            } else {
                for yy in y..(y + rh) {
                    for xx in x..(x + rw) {
                        img.put_pixel(xx, yy, image::Rgba([255, 255, 255, 255]));
                    }
                }
            }
        }
        let tmp = tempfile::tempdir().map_err(|e| Error::output(e.to_string()))?;
        let mask = tmp.path().join("mask.png");
        img.save(&mask)
            .map_err(|e| Error::output(format!("write mask png: {e}")))?;
        mask_tmp = Some(tmp);
        let m = mask
            .to_string_lossy()
            .replace('\\', "\\\\")
            .replace(':', "\\:")
            .replace('\'', "\\'");
        format!(
            "removelogo=filename='{m}'{}",
            enable.as_deref().unwrap_or("")
        )
    } else {
        regions
            .iter()
            .map(|&(x, y, rw, rh)| {
                format!(
                    "delogo=x={x}:y={y}:w={rw}:h={rh}{}",
                    enable.as_deref().unwrap_or("")
                )
            })
            .collect::<Vec<_>>()
            .join(",")
    };
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
    drop(mask_tmp);
    let mut extra = json!({
        "box": {"x": regions[0].0, "y": regions[0].1, "w": regions[0].2, "h": regions[0].3},
        "regions": regions,
    });
    if let Some(at) = &args.at {
        extra["at"] = json!(at);
        if let Some(d) = args.dur {
            extra["dur"] = json!(d);
        }
    }
    Ok(c.with_extra(extra))
}

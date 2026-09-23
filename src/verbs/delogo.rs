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
    let image_mask = args.image.as_ref().map(|p| {
        let m = p
            .to_string_lossy()
            .replace('\\', "\\\\")
            .replace(':', "\\:")
            .replace('\'', "\\'");
        m
    });
    if image_mask.is_some() {
        if args.x.is_some()
            || args.y.is_some()
            || args.w.is_some()
            || args.h.is_some()
            || args.regions.is_some()
        {
            return Err(Error::input(
                "--image conflicts with --x/--y/--w/--h/--regions",
            ));
        }
        if !args.image.as_ref().unwrap().exists() {
            return Err(Error::input("--image file not found"));
        }
    }
    if let Some(f) = &args.find {
        if args.x.is_some()
            || args.y.is_some()
            || args.w.is_some()
            || args.h.is_some()
            || args.regions.is_some()
            || args.image.is_some()
        {
            return Err(Error::input(
                "--find conflicts with --x/--y/--w/--h/--regions/--image",
            ));
        }
        if !f.exists() {
            return Err(Error::input("--find file not found"));
        }
    }
    // --find: find_rect hunts the reference bitmap through the first 15s and
    // reports its box in lavfi.rect.* frame metadata — that becomes the
    // removal region, no manual coordinates needed
    let found = if let Some(f) = &args.find {
        let esc = |p: &std::path::Path| {
            p.to_string_lossy()
                .replace('\\', "\\\\")
                .replace(':', "\\:")
                .replace('\'', "\\'")
        };
        let graph = format!(
            "movie='{}'[v];[v]trim=duration=15,find_rect=object='{}':threshold=0.5",
            esc(&args.input),
            esc(f)
        );
        let mut pv = crate::spawn::Argv::ffprobe();
        pv.push("-f");
        pv.push("lavfi");
        pv.push("-i");
        pv.push(&graph);
        pv.extend(["-show_frames", "-of", "default=nw=1"]);
        let sp = crate::spawn::run(&pv, g.timeout, false)?;
        let sp = crate::spawn::require_ok(&pv, sp)?;
        let out = crate::spawn::stdout_str(&sp).unwrap_or_default();
        // scan frame blocks until one reports all four lavfi.rect.* keys —
        // mixing coords across frames would corrupt the box
        let mut hit: Option<(u32, u32, u32, u32)> = None;
        let mut vals = [None::<u32>; 4];
        let mut seen = false;
        for l in out.lines() {
            if l.starts_with("frame:") {
                if seen {
                    if let [Some(x), Some(y), Some(w2), Some(h2)] = vals {
                        hit = Some((x, y, w2, h2));
                        break;
                    }
                    vals = [None; 4];
                }
                seen = true;
                continue;
            }
            for (k, i) in [
                ("lavfi.rect.x=", 0usize),
                ("lavfi.rect.y=", 1usize),
                ("lavfi.rect.w=", 2usize),
                ("lavfi.rect.h=", 3usize),
            ] {
                if let Some(v) = l
                    .split(k)
                    .nth(1)
                    .and_then(|s| s.trim().split(' ').next())
                    .and_then(|s| s.parse().ok())
                {
                    vals[i] = Some(v);
                }
            }
        }
        if hit.is_none() {
            if let [Some(x), Some(y), Some(w2), Some(h2)] = vals {
                hit = Some((x, y, w2, h2));
            }
        }
        hit.map(|(x, y, w2, h2)| vec![(x, y, w2, h2)])
    } else {
        None
    };
    let regions = if let Some(r) = found {
        if args.find.is_some() && r.is_empty() {
            return Err(Error::input(
                "object not found in the first 15s — pass --x/--y/--w/--h or --regions",
            ));
        }
        r
    } else if args.find.is_some() {
        return Err(Error::input(
            "object not found in the first 15s — pass --x/--y/--w/--h or --regions",
        ));
    } else if image_mask.is_some() {
        Vec::new()
    } else {
        match &args.regions {
            Some(list) => {
                if args.x.is_some() || args.y.is_some() || args.w.is_some() || args.h.is_some() {
                    return Err(Error::input("--regions conflicts with --x/--y/--w/--h"));
                }
                let mut v = Vec::new();
                for part in list.split(',') {
                    let nums: Vec<u32> = part
                        .split(':')
                        .map(|n| {
                            n.parse().map_err(|_| {
                                Error::input(format!("--region '{part}' wants x:y:w:h"))
                            })
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
                        "need --x --y --w --h, --regions x:y:w:h[,...], --image mask.png, or --find ref.png",
                    ))
                }
            },
        }
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
    let vf = if let Some(m) = &image_mask {
        format!(
            "removelogo=filename='{m}'{}",
            enable.as_deref().unwrap_or("")
        )
    } else if args.soft || circle {
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
    let mut extra = if image_mask.is_some() {
        json!({ "mask_image": args.image })
    } else if args.find.is_some() {
        json!({ "find": args.find, "found_box": regions[0] })
    } else {
        json!({
            "box": {"x": regions[0].0, "y": regions[0].1, "w": regions[0].2, "h": regions[0].3},
            "regions": regions,
        })
    };
    if let Some(at) = &args.at {
        extra["at"] = json!(at);
        if let Some(d) = args.dur {
            extra["dur"] = json!(d);
        }
    }
    Ok(c.with_extra(extra))
}

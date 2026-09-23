use serde_json::json;

use crate::cli::{CensorArgs, CensorMode, CensorShape, Globals};
use crate::contract::{Contract, Status};
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

fn parse_regions(s: &str) -> Result<Vec<(u32, u32, u32, u32)>, Error> {
    s.split(',')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(parse_region)
        .collect()
}

fn parse_region(s: &str) -> Result<(u32, u32, u32, u32), Error> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 4 {
        return Err(Error::input(
            "--region must look like x:y:w:h (e.g. 100:80:64:64)",
        ));
    }
    let mut vals = [0u32; 4];
    for (i, p) in parts.iter().enumerate() {
        vals[i] = p
            .trim()
            .parse()
            .map_err(|_| Error::input("--region takes integers: x:y:w:h"))?;
    }
    if vals[2] < 8 || vals[3] < 8 {
        return Err(Error::input("--region w/h must be at least 8px"));
    }
    Ok((vals[0], vals[1], vals[2], vals[3]))
}

pub fn run(args: CensorArgs, g: &Globals) -> Result<Contract, Error> {
    let regions = parse_regions(&args.region)?;
    if regions.is_empty() {
        return Err(Error::input("--region must look like x:y:w:h"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "censor")?;
    let (iw, ih) = (probe.width.unwrap_or(0), probe.height.unwrap_or(0));
    for (x, y, w, h) in &regions {
        if iw > 0 && (x + w > iw || y + h > ih) {
            return Err(Error::input(format!(
                "--region {x}:{y}:{w}:{h} falls outside the {iw}x{ih} frame"
            )));
        }
    }

    // pixelize is ffmpeg 5+; downscale/upscale-nearest mosaics everywhere.
    let effect_of = |w: u32, h: u32| match args.mode {
        CensorMode::Pixel => format!(
            "scale=w={bw}:h={bh}:flags=neighbor,scale={w}:{h}:flags=neighbor",
            bw = (w as f64 / args.strength.max(2.0)).max(2.0) as u32,
            bh = (h as f64 / args.strength.max(2.0)).max(2.0) as u32,
        ),
        CensorMode::Blur => format!("gblur=sigma={:.0}", args.strength.max(1.0)),
    };
    let enable = match (&args.at, args.dur) {
        (Some(at), dur) => {
            if at.contains(',') && dur.is_none() {
                return Err(Error::input("a comma list of --at times needs --dur"));
            }
            let mut starts = Vec::new();
            for part in at.split(',') {
                let start = crate::time::resolve_at(part.trim(), dur, probe.duration)?;
                if !(0.0..probe.duration).contains(&start) {
                    return Err(Error::input("--at is outside the input"));
                }
                starts.push(start);
            }
            if starts.len() == 1 {
                let start = starts[0];
                match dur.map(|d| start + d) {
                    Some(e) if e < probe.duration => {
                        format!(":enable='between(t,{start:.3},{e:.3})'")
                    }
                    _ => format!(":enable='gte(t,{start:.3})'"),
                }
            } else {
                let d = dur.unwrap();
                let expr = starts
                    .iter()
                    .map(|s| {
                        let e = (s + d).min(probe.duration);
                        format!("between(t,{s:.3},{e:.3})")
                    })
                    .collect::<Vec<_>>()
                    .join("+");
                format!(":enable='{expr}'")
            }
        }
        (None, Some(_)) => return Err(Error::input("--dur needs --at")),
        (None, None) => String::new(),
    };
    // One crop+effect+overlay arm per region, chained on the main video.
    let mut fc = String::new();
    let mut prev = "0:v".to_string();
    for (i, (x, y, w, h)) in regions.iter().enumerate() {
        let last = i + 1 == regions.len();
        let out = if last {
            "vout".to_string()
        } else {
            format!("v{i}")
        };
        let mask = match args.shape {
            CensorShape::Box => String::new(),
            // elliptical alpha mask so pixelated faces read as circles
            CensorShape::Circle => ",format=rgba,geq=r='r(X,Y)':g='g(X,Y)':b='b(X,Y)':a='if(lte(hypot(X-W/2,Y-H/2),min(W\\,H)/2),255,0)'".to_string(),
        };
        fc.push_str(&format!(
            "[{prev}]split[b{i}][t{i}];             [t{i}]crop={w}:{h}:{x}:{y},{eff}{mask}[c{i}];             [b{i}][c{i}]overlay={x}:{y}:shortest=1{en}[{out}];",
            eff = effect_of(*w, *h),
            mask = mask,
            en = enable,
        ));
        prev = out;
    }
    fc.pop();

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-filter_complex", &fc, "-map", "[vout]"]);
    if probe.has_audio {
        argv.extend(["-map", "0:a", "-c:a", "aac"]);
    }
    argv.extend([
        "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
    ]);
    argv.push(&args.output);

    let c = engine::write_job("censor", &[&args.input], &args.output, vec![argv], g)?;
    let mut extra = json!({
        "regions": regions.iter().map(|(x,y,w,h)| format!("{x}:{y}:{w}:{h}")).collect::<Vec<_>>(),
        "mode": format!("{:?}", args.mode).to_lowercase(),
    });
    if let Some(at) = &args.at {
        extra["at"] = json!(at);
        if let Some(d) = args.dur {
            extra["dur"] = json!(d);
        }
    }
    if matches!(c.status, Status::Ok) {
        if let Ok(p) = engine::probe_or_err(&args.output, g) {
            extra["probe"] = json!(p);
        }
    }
    Ok(c.with_extra(extra))
}

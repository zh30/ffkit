use serde_json::json;

use crate::cli::{CensorArgs, CensorMode, Globals};
use crate::contract::{Contract, Status};
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

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
    let (x, y, w, h) = parse_region(&args.region)?;
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "censor")?;
    let (iw, ih) = (probe.width.unwrap_or(0), probe.height.unwrap_or(0));
    if iw > 0 && (x + w > iw || y + h > ih) {
        return Err(Error::input(format!(
            "--region {x}:{y}:{w}:{h} falls outside the {iw}x{ih} frame"
        )));
    }

    // pixelize is ffmpeg 5+; downscale/upscale-nearest mosaics everywhere.
    let effect = match args.mode {
        CensorMode::Pixel => format!(
            "scale=w={bw}:h={bh}:flags=neighbor,scale={w}:{h}:flags=neighbor",
            bw = (w / 16).max(2),
            bh = (h / 16).max(2),
        ),
        CensorMode::Blur => "gblur=sigma=30".to_string(),
    };
    let enable = match (&args.at, args.dur) {
        (Some(at), dur) => {
            let start = crate::time::parse_time(at)?;
            if !(0.0..probe.duration).contains(&start) {
                return Err(Error::input("--at is outside the input"));
            }
            match dur.map(|d| start + d) {
                Some(e) if e < probe.duration => {
                    format!(":enable='between(t,{start:.3},{e:.3})'")
                }
                _ => format!(":enable='gte(t,{start:.3})'"),
            }
        }
        (None, Some(_)) => return Err(Error::input("--dur needs --at")),
        (None, None) => String::new(),
    };
    let fc = format!(
        "[0:v]split[base][top];\
         [top]crop={w}:{h}:{x}:{y},{effect}[cens];\
         [base][cens]overlay={x}:{y}:shortest=1{enable}[vout]"
    );

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
        "region": args.region,
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

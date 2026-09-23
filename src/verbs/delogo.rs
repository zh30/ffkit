use serde_json::json;

use crate::cli::{DelogoArgs, Globals};
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
    if args.w < 4 || args.h < 4 {
        return Err(Error::input("--w/--h must be >= 4 px"));
    }
    let (x1, y1) = (args.x as i64 + args.w as i64, args.y as i64 + args.h as i64);
    if x1 > w || y1 > h {
        return Err(Error::input(format!(
            "logo box {}x{}@{}+{} exceeds {}x{} frame",
            args.w, args.h, args.x, args.y, w, h
        )));
    }
    // --soft: removelogo reads a PNG mask (white = remove) and interpolates
    // edges instead of boxing — gentler on gradients/sky.
    let mut mask_tmp = None;
    let mut vf = if args.soft {
        let (fw, fh) = (w as u32, h as u32);
        let mut img = image::RgbaImage::from_pixel(fw, fh, image::Rgba([0, 0, 0, 255]));
        for y in args.y..(args.y + args.h) {
            for x in args.x..(args.x + args.w) {
                img.put_pixel(x, y, image::Rgba([255, 255, 255, 255]));
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
        format!("removelogo=filename='{m}'")
    } else {
        format!("delogo=x={}:y={}:w={}:h={}", args.x, args.y, args.w, args.h)
    };
    match (&args.at, args.dur) {
        (Some(at), dur) => {
            vf.push_str(&format!(
                ":enable='{}'",
                crate::time::enable_expr(at, dur, probe.duration)?
            ));
        }
        (None, Some(_)) => return Err(Error::input("--dur needs --at")),
        (None, None) => {}
    }
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
        "box": {"x": args.x, "y": args.y, "w": args.w, "h": args.h},
    });
    if let Some(at) = &args.at {
        extra["at"] = json!(at);
        if let Some(d) = args.dur {
            extra["dur"] = json!(d);
        }
    }
    Ok(c.with_extra(extra))
}

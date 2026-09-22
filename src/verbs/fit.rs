use crate::cli::{FitArgs, FitMode, FlipMode, Globals};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::paths;

pub fn run(args: FitArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "fit")?;

    let (tw, th) = target_frame(
        &args,
        probe.width.unwrap_or(1280),
        probe.height.unwrap_or(720),
    )?;
    let tw = paths::even(tw).max(2);
    let th = paths::even(th).max(2);

    let mut vf = Vec::new();
    match args.rotate {
        Some(90) => vf.push("transpose=1".into()),
        Some(180) => vf.push("transpose=1,transpose=1".into()),
        Some(270) => vf.push("transpose=2".into()),
        Some(0) | None => {}
        Some(r) => {
            return Err(Error::input(format!(
                "--rotate must be 90, 180, or 270 (got {r})"
            )));
        }
    }
    match args.flip {
        Some(FlipMode::H) => vf.push("hflip".into()),
        Some(FlipMode::V) => vf.push("vflip".into()),
        None => {}
    }

    if let Some(fps) = args.fps {
        if fps <= 0.0 {
            return Err(Error::input("--fps must be positive"));
        }
    }

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    if args.fit == FitMode::Blur {
        // Repurpose look: a zoomed, blurred copy fills the frame and the scaled
        // foreground sits centered on it. Needs split + overlay → filter_complex.
        let mut tail = vec!["setsar=1".to_string(), "format=yuv420p".to_string()];
        if let Some(fps) = args.fps {
            tail.push(format!("fps={fps}"));
        }
        let pre = if vf.is_empty() {
            String::new()
        } else {
            format!("{},", vf.join(","))
        };
        let fc = format!(
            "[0:v]{pre}split[bg0][fg0];\
             [bg0]scale={tw}:{th}:force_original_aspect_ratio=increase,crop={tw}:{th},gblur=sigma=30[bg];\
             [fg0]scale={tw}:{th}:force_original_aspect_ratio=decrease[fg];\
             [bg][fg]overlay=(W-w)/2:(H-h)/2,{}[vout]",
            tail.join(","),
        );
        argv.extend(["-filter_complex", &fc, "-map", "[vout]"]);
        if probe.has_audio {
            argv.extend(["-map", "0:a?"]);
        }
    } else {
        let scale_pad = match args.fit {
            FitMode::Pad => {
                let pad_color = match &args.color {
                    Some(c) => crate::color::lavfi(c),
                    None => "black".to_string(),
                };
                format!(
                    "scale={tw}:{th}:force_original_aspect_ratio=decrease,pad={tw}:{th}:(ow-iw)/2:(oh-ih)/2:{pad_color}"
                )
            }
            FitMode::Crop => {
                format!("scale={tw}:{th}:force_original_aspect_ratio=increase,crop={tw}:{th}")
            }
            FitMode::Blur => unreachable!(),
        };
        vf.push(scale_pad);
        vf.push("setsar=1".into());
        vf.push("format=yuv420p".into());
        if let Some(fps) = args.fps {
            vf.push(format!("fps={fps}"));
        }
        argv.extend(["-vf", &vf.join(",")]);
    }
    argv.extend(["-c:v", "libx264", "-preset", "fast", "-crf", "18"]);
    if probe.has_audio {
        argv.extend(["-c:a", "aac"]);
    } else {
        argv.push("-an");
    }
    argv.push(&args.output);
    engine::write_job("fit", &[&args.input], &args.output, vec![argv], g)
}

fn target_frame(args: &FitArgs, src_w: u32, src_h: u32) -> Result<(u32, u32), Error> {
    if let (Some(w), Some(h)) = (args.width, args.height) {
        return Ok((w, h));
    }
    if let Some(aspect) = &args.aspect {
        let (aw, ah) = parse_aspect(aspect)?;
        let (w, h) = if let Some(width) = args.width {
            (width, ((width as u64 * ah as u64) / aw as u64) as u32)
        } else if let Some(height) = args.height {
            ((((height as u64) * aw as u64) / ah as u64) as u32, height)
        } else if aw <= ah {
            (1080, ((1080u64 * ah as u64) / aw as u64) as u32)
        } else {
            ((((1080u64) * aw as u64) / ah as u64) as u32, 1080)
        };
        return Ok((w, h));
    }
    if let Some(w) = args.width {
        let h = ((src_h as u64 * w as u64) / src_w as u64) as u32;
        return Ok((w, h));
    }
    if let Some(h) = args.height {
        let w = ((src_w as u64 * h as u64) / src_h as u64) as u32;
        return Ok((w, h));
    }
    Err(Error::input("fit needs --aspect and/or --width/--height"))
}

fn parse_aspect(s: &str) -> Result<(u32, u32), Error> {
    let (a, b) = s
        .split_once(':')
        .ok_or_else(|| Error::input(format!("invalid aspect {s}; use 9:16")))?;
    let aw: u32 = a
        .parse()
        .map_err(|_| Error::input(format!("invalid aspect {s}")))?;
    let ah: u32 = b
        .parse()
        .map_err(|_| Error::input(format!("invalid aspect {s}")))?;
    if aw == 0 || ah == 0 {
        return Err(Error::input("aspect sides must be > 0"));
    }
    Ok((aw, ah))
}

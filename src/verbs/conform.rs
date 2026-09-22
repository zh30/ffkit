use std::path::Path;

use serde_json::json;

use crate::cli::{ConformArgs, Globals};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: ConformArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    if args.size.is_none() && args.fps.is_none() && args.lufs.is_none() {
        return Err(Error::input(
            "nothing to conform — pass --size WxH, --fps N, and/or --lufs L",
        ));
    }

    let mut vf: Vec<String> = Vec::new();
    if let Some(sz) = &args.size {
        let (w, h) = sz
            .split_once(['x', 'X'])
            .and_then(|(w, h)| Some((w.parse::<u32>().ok()?, h.parse::<u32>().ok()?)))
            .filter(|(w, h)| *w > 0 && *h > 0)
            .map(|(w, h)| (w & !1, h & !1))
            .ok_or_else(|| Error::input("--size must be WxH, e.g. 1920x1080"))?;
        // Fit inside WxH without upscaling beyond: even dims, letterbox-safe.
        vf.push(format!(
            "scale=w={w}:h={h}:force_original_aspect_ratio=decrease:force_divisible_by=2"
        ));
    }
    if let Some(fps) = args.fps {
        if !(1.0..=240.0).contains(&fps) {
            return Err(Error::input("--fps must be 1..=240"));
        }
        vf.push(format!("fps={fps}"));
    }
    vf.push("format=yuv420p".into());

    let mut argv = ffmpeg_base(g.progress);
    argv.extend(["-i".to_string(), args.input.display().to_string()]);
    if probe.has_video {
        argv.extend(["-vf".to_string(), vf.join(",")]);
        argv.extend([
            "-c:v".to_string(),
            "libx264".to_string(),
            "-preset".to_string(),
            "fast".to_string(),
            "-crf".to_string(),
            "18".to_string(),
        ]);
    }
    if probe.has_audio {
        // loudnorm upsamples internally — resample back AFTER it.
        let mut af = String::new();
        if let Some(l) = args.lufs {
            if !(-70.0..=-5.0).contains(&l) {
                return Err(Error::input("--lufs must be -70..=-5 (e.g. -14)"));
            }
            af.push_str(&format!("loudnorm=I={l:.1}:TP=-1.5:LRA=11,"));
        }
        af.push_str("aresample=48000,aformat=channel_layouts=stereo");
        argv.extend(["-af".to_string(), af]);
        argv.extend([
            "-c:a".to_string(),
            "aac".to_string(),
            "-b:a".to_string(),
            "192k".to_string(),
        ]);
    }
    argv.push(args.output.display().to_string());

    let inputs: Vec<&Path> = vec![&args.input];
    let mut c = engine::write_job("conform", &inputs, &args.output, vec![argv], g)?;
    c = c.with_extra(json!({
        "size": args.size,
        "fps": args.fps,
        "lufs": args.lufs,
    }));
    Ok(c)
}

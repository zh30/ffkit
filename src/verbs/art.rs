use std::path::Path;

use serde_json::json;

use crate::cli::{ArtArgs, Globals};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::paths;

pub fn run(args: ArtArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_audio && !probe.has_video {
        return Err(Error::input("art: input has no media streams"));
    }
    paths::ensure_input(&args.input)?;
    if args.extract {
        // Pull the embedded cover stream out to -o (jpg/png by container tag).
        let probe = engine::probe_or_err(&args.input, g)?;
        if !probe.has_video {
            return Err(Error::input("art --extract: no embedded picture found"));
        }
        let mut argv = ffmpeg_base(g.progress);
        argv.extend(["-i".to_string(), args.input.display().to_string()]);
        argv.extend([
            "-map".to_string(),
            "0:v:0".to_string(),
            "-c".to_string(),
            "copy".to_string(),
        ]);
        argv.push(args.output.display().to_string());
        let inputs: Vec<&Path> = vec![&args.input];
        return engine::write_job("art", &inputs, &args.output, vec![argv], g);
    }
    let image = args.image.as_ref().unwrap();
    paths::ensure_input(image)?;
    let ext = args
        .output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let mut argv = ffmpeg_base(g.progress);
    argv.extend(["-i".to_string(), args.input.display().to_string()]);
    argv.extend(["-i".to_string(), image.display().to_string()]);
    argv.extend(["-map".to_string(), "0".to_string()]);
    argv.extend(["-map".to_string(), "1:v".to_string()]);
    // Cover art rides as an mjpeg frame everywhere (mp3 APIC / mp4 attached_pic).
    argv.extend(["-c:v".to_string(), "mjpeg".to_string()]);
    match ext.as_str() {
        "mp3" => {
            argv.extend([
                "-c:a".to_string(),
                "libmp3lame".to_string(),
                "-q:a".to_string(),
                "4".to_string(),
                "-id3v2_version".to_string(),
                "3".to_string(),
                "-metadata:s:v".to_string(),
                "comment=cover (front)".to_string(),
            ]);
        }
        "m4a" | "aac" => {
            argv.extend([
                "-c:a".to_string(),
                "aac".to_string(),
                "-disposition:v:1".to_string(),
                "attached_pic".to_string(),
            ]);
        }
        "mp4" | "mov" | "m4v" | "mkv" => {
            argv.extend([
                "-c:a".to_string(),
                "copy".to_string(),
                "-disposition:v:1".to_string(),
                "attached_pic".to_string(),
            ]);
        }
        other => {
            return Err(Error::input(format!(
                ".{other} can't hold cover art — output .mp3, .m4a, .mp4, or .mkv"
            )));
        }
    }
    argv.push(args.output.display().to_string());

    let inputs: Vec<&Path> = vec![&args.input, image];
    let mut c = engine::write_job("art", &inputs, &args.output, vec![argv], g)?;
    c = c.with_extra(json!({ "art": paths::display(image) }));
    Ok(c)
}

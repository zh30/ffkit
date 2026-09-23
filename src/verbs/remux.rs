use serde_json::json;

use crate::cli::{Globals, RemuxArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: RemuxArgs, g: &Globals) -> Result<Contract, Error> {
    let ext = args
        .output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if ext.is_empty() {
        return Err(Error::input(
            "output needs an extension (mp4, mkv, mov, m4a…)",
        ));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_video && !probe.has_audio {
        return Err(Error::input("remux: input has no media streams"));
    }

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    if args.audio && args.video {
        return Err(Error::input("remux: --audio and --video are exclusive"));
    }
    if args.audio {
        if !probe.has_audio {
            return Err(Error::input("remux --audio: input has no audio"));
        }
        // Re-encode only when the container can't hold the source codec.
        let src = probe.acodec.as_deref().unwrap_or("");
        let fits = match ext.as_str() {
            "m4a" | "mp4" | "mov" => matches!(src, "aac" | "alac" | "mp3"),
            "mp3" => src == "mp3",
            "ogg" | "oga" => matches!(src, "vorbis" | "opus" | "flac"),
            "wav" | "aif" | "aiff" | "caf" => src.starts_with("pcm"),
            _ => true,
        };
        argv.extend(["-map", "0:a"]);
        if fits {
            argv.extend(["-c:a", "copy"]);
        } else {
            let codec = match ext.as_str() {
                "mp3" => "libmp3lame",
                "ogg" | "oga" => "libvorbis",
                "wav" | "aif" | "aiff" | "caf" => "pcm_s16le",
                _ => "aac",
            };
            argv.extend(["-c:a", codec]);
        }
    } else if args.video {
        if !probe.has_video {
            return Err(Error::input("remux --video: input has no video"));
        }
        argv.extend(["-map", "0:v", "-c:v", "copy"]);
    } else {
        argv.extend(["-map", "0", "-c", "copy"]);
    }
    if matches!(ext.as_str(), "mp4" | "m4a" | "mov") {
        argv.extend(["-movflags", "+faststart"]);
    }
    argv.push(&args.output);

    let c = engine::write_job("remux", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(
        json!({ "container": ext, "audio_only": args.audio, "video_only": args.video }),
    ))
}

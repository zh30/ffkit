use std::path::Path;

use serde_json::json;

use crate::cli::{AudiogramArgs, Globals};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::paths;

/// Podcast clip → 1080x1920 video: cover still (or flat colour) with a
/// showwaves strip keyed over it. Audio is re-encoded to aac.
pub fn run(args: AudiogramArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_audio {
        return Err(Error::input("audiogram: input has no audio stream"));
    }
    if let Some(img) = &args.image {
        paths::ensure_input(img)?;
    }

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    if let Some(img) = &args.image {
        argv.extend(["-loop", "1", "-i"]);
        argv.push(img);
    } else {
        argv.extend(["-f", "lavfi", "-i", "color=c=0x101418:s=1080x1920:r=30"]);
    }

    // Waveform sits in the lower-middle band — clear of Reels/TikTok top and
    // bottom chrome — and the black showwaves floor is keyed out so the cover
    // shows through. overlay shortest=1 ends [vout] with the waveform: -shortest
    // alone overshoots because the encoder queue keeps the infinite cover
    // going past audio EOF.
    let fc =
        "[1:v]scale=1080:1920:force_original_aspect_ratio=increase,crop=1080:1920,setsar=1[bg];\
              [0:a]showwaves=s=940x320:mode=cline:rate=30:colors=white:draw=full[wv];\
              [wv]colorkey=0x000000:0.12:0.1[wvk];\
              [bg][wvk]overlay=(W-w)/2:(H-h)*0.62:shortest=1[vout]";
    argv.extend(["-filter_complex", fc, "-map", "[vout]", "-map", "0:a"]);
    argv.extend([
        "-c:v",
        "libx264",
        "-preset",
        "fast",
        "-crf",
        "20",
        "-pix_fmt",
        "yuv420p",
        "-c:a",
        "aac",
        "-shortest",
    ]);
    argv.push(&args.output);

    let mut inputs: Vec<&Path> = vec![&args.input];
    if let Some(img) = &args.image {
        inputs.push(img);
    }
    let mut c = engine::write_job("audiogram", &inputs, &args.output, vec![argv], g)?;
    c = c.with_extra(json!({
        "frame": "1080x1920",
        "waveform": "showwaves",
    }));
    Ok(c)
}

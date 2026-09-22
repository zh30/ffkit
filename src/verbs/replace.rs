use std::path::Path;

use serde_json::json;

use crate::cli::{Globals, ReplaceArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::paths;

/// Swap a video's audio track for a new file (lav mic, clean voice, new
/// music). Picture is stream-copied; the new audio is padded or trimmed to
/// the video length so the output keeps the input duration.
pub fn run(args: ReplaceArgs, g: &Globals) -> Result<Contract, Error> {
    if !(-600.0..=600.0).contains(&args.audio_offset) {
        return Err(Error::input("--audio-offset must be -600..=600 s"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "replace")?;
    paths::ensure_input(&args.audio)?;
    let audio_probe = engine::probe_or_err(&args.audio, g)?;
    if !audio_probe.has_audio {
        return Err(Error::input("replace: --audio file has no audio stream"));
    }

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    if args.audio_offset < 0.0 {
        // Input-side seek trims the head of the replacement audio.
        argv.extend(["-ss", &format!("{:.3}", -args.audio_offset)]);
    }
    argv.extend(["-i"]);
    argv.push(&args.audio);

    if args.mix != 0.0 && !(0.0..=1.0).contains(&args.mix) {
        return Err(Error::input("--mix must be a linear gain 0..=1"));
    }
    if args.mix > 0.0 && !probe.has_audio {
        return Err(Error::input(
            "replace --mix needs an audio stream on the input",
        ));
    }

    let mut chain = String::new();
    if args.audio_offset > 0.0 {
        chain = format!("adelay={:.0}:all=1,", args.audio_offset * 1000.0);
    }
    // apad then atrim: short beds get silence to the credits, long beds are cut.
    let fc = if args.mix > 0.0 {
        format!(
            "[1:a]{chain}apad,atrim=duration={:.3},aresample=48000,aformat=channel_layouts=stereo[new];[0:a]aresample=48000,aformat=channel_layouts=stereo,volume={:.3},atrim=duration={:.3}[old];[old][new]amix=inputs=2:normalize=0[aout]",
            probe.duration, args.mix, probe.duration
        )
    } else {
        format!(
            "[1:a]{chain}apad,atrim=duration={:.3},aresample=48000,aformat=channel_layouts=stereo[aout]",
            probe.duration
        )
    };
    argv.extend([
        "-filter_complex",
        &fc,
        "-map",
        "0:v",
        "-map",
        "[aout]",
        "-c:v",
        "copy",
        "-c:a",
        "aac",
    ]);
    argv.push(&args.output);

    let inputs: Vec<&Path> = vec![&args.input, &args.audio];
    let mut c = engine::write_job("replace", &inputs, &args.output, vec![argv], g)?;
    c = c.with_extra(json!({
        "audio": args.audio,
        "audio_offset": args.audio_offset,
        "mix": args.mix,
        "video_copy": true,
    }));
    Ok(c)
}

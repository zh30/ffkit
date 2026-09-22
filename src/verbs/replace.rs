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
    if args.loop_track {
        argv.extend(["-stream_loop", "-1"]);
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
    if args.dur.is_some() && args.at.is_none() {
        return Err(Error::input("replace --dur needs --at"));
    }
    if let Some(at) = args.at {
        if !probe.has_audio {
            return Err(Error::input(
                "replace --at needs an audio stream on the input",
            ));
        }
        if args.mix > 0.0 || args.duck {
            return Err(Error::input(
                "replace --at replaces wholesale — drop --mix/--duck",
            ));
        }
        let end = match args.dur {
            Some(d) => (at + d).min(probe.duration),
            None => (at + (audio_probe.duration - args.audio_offset.max(0.0))).min(probe.duration),
        };
        if at < 0.0 || end <= at || end > probe.duration + 0.01 {
            return Err(Error::input(
                "replace --at/--dur window is empty or outside the source",
            ));
        }
    }

    let mut chain = String::new();
    if args.audio_offset > 0.0 {
        chain = format!("adelay={:.0}:all=1,", args.audio_offset * 1000.0);
    }
    // --fade: afade in/out on the new audio, fade-out ending at the video edge.
    let fade = if args.fade > 0.0 {
        let f = args.fade.min(probe.duration / 2.0).max(0.02);
        format!(
            ",afade=t=in:st=0:d={f:.3},afade=t=out:st={:.3}:d={f:.3}",
            probe.duration - f
        )
    } else {
        String::new()
    };
    // apad then atrim: short beds get silence to the credits, long beds are cut.
    const AF: &str = "aformat=sample_fmts=dbl:sample_rates=48000:channel_layouts=stereo";
    let fc = if args.mix > 0.0 && args.duck {
        // sidechaincompress needs packed dbl on both pads (same pinning as music.rs)
        format!(
            "[1:a]{chain}apad,atrim=duration={:.3}{fade},{AF}[new];[new]asplit=2[n1][n2];             [0:a]aresample=48000,aformat=channel_layouts=stereo,volume={:.3},atrim=duration={:.3},{AF}[old];             [old][n1]sidechaincompress=threshold=0.05:ratio=6:attack=20:release=250[dk];             [dk][n2]amix=inputs=2:duration=first:dropout_transition=0:normalize=0[aout]",
            probe.duration, args.mix, probe.duration
        )
    } else if args.mix > 0.0 {
        format!(
            "[1:a]{chain}apad,atrim=duration={:.3}{fade},aresample=48000,aformat=channel_layouts=stereo[new];[0:a]aresample=48000,aformat=channel_layouts=stereo,volume={:.3},atrim=duration={:.3}[old];[old][new]amix=inputs=2:normalize=0[aout]",
            probe.duration, args.mix, probe.duration
        )
    } else if let Some(at) = args.at {
        // Windowed replacement: original track outside [at, end), new audio inside.
        let end = match args.dur {
            Some(d) => (at + d).min(probe.duration),
            None => (at + (audio_probe.duration - args.audio_offset.max(0.0))).min(probe.duration),
        };
        let len = end - at;
        let wf = if args.fade > 0.0 {
            let f = args.fade.min(len / 2.0).max(0.02);
            format!(
                ",afade=t=in:st=0:d={f:.3},afade=t=out:st={:.3}:d={f:.3}",
                len - f
            )
        } else {
            String::new()
        };
        let mut segs = String::new();
        let mut pads: Vec<&str> = Vec::new();
        if at > 0.001 {
            segs.push_str(&format!("[0:a]atrim=0:{at:.3},asetpts=PTS-STARTPTS[a0];"));
            pads.push("a0");
        }
        segs.push_str(&format!(
            "[1:a]{chain}atrim=0:{len:.3},asetpts=PTS-STARTPTS{wf},aresample=48000,aformat=channel_layouts=stereo[a1];"
        ));
        pads.push("a1");
        if end < probe.duration - 0.001 {
            segs.push_str(&format!("[0:a]atrim={end:.3},asetpts=PTS-STARTPTS[a2];"));
            pads.push("a2");
        }
        let n = pads.len();
        let ins: String = pads.iter().map(|p| format!("[{p}]")).collect();
        format!("{segs}{ins}concat=n={n}:v=0:a=1[aout]")
    } else {
        format!(
            "[1:a]{chain}apad,atrim=duration={:.3}{fade},aresample=48000,aformat=channel_layouts=stereo[aout]",
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
        "window": args.at.map(|at| {
            let end = match args.dur {
                Some(d) => (at + d).min(probe.duration),
                None => (at + (audio_probe.duration - args.audio_offset.max(0.0)))
                    .min(probe.duration),
            };
            json!({"at": at, "end": end})
        }),
        "mix": args.mix,
        "duck": args.duck,
        "video_copy": true,
        "loop": args.loop_track,
    }));
    Ok(c)
}

use std::path::{Path, PathBuf};

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
    if let Some(video) = &args.video {
        return replace_video(&args, g, &probe, video.clone());
    }
    let audio = args
        .audio
        .as_ref()
        .ok_or_else(|| Error::input("replace needs --audio or --video"))?;
    paths::ensure_input(audio)?;
    let audio_probe = engine::probe_or_err(audio, g)?;
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
    argv.push(audio);

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
    let windows: Vec<(f64, f64)> = if let Some(raw) = &args.at {
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
        let mut ws = Vec::new();
        for part in raw.split(',') {
            let at = crate::time::parse_time(part.trim())?;
            let end = match args.dur {
                Some(d) => (at + d).min(probe.duration),
                None => {
                    (at + (audio_probe.duration - args.audio_offset.max(0.0))).min(probe.duration)
                }
            };
            if at < 0.0 || end <= at || end > probe.duration + 0.01 {
                return Err(Error::input(
                    "replace --at/--dur window is empty or outside the source",
                ));
            }
            ws.push((at, end));
        }
        ws.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        for w in ws.windows(2) {
            if w[1].0 < w[0].1 - 1e-6 {
                return Err(Error::input("replace: --at windows overlap"));
            }
        }
        ws
    } else {
        Vec::new()
    };

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
    } else if !windows.is_empty() {
        // Windowed replacement: original track outside each window, the new
        // audio laid across the windows in order (comma --at = several).

        let mut segs = String::new();
        let mut pads: Vec<String> = Vec::new();
        let mut prev = 0.0f64;
        let mut off = 0.0f64;
        let mut k = 0usize;
        for &(s, e) in &windows {
            if s > prev + 0.001 {
                segs.push_str(&format!(
                    "[0:a]atrim={prev:.3}:{s:.3},asetpts=PTS-STARTPTS[a{k}];"
                ));
                pads.push(format!("a{k}"));
                k += 1;
            }
            let wlen = e - s;
            let wf2 = if args.fade > 0.0 {
                let f = args.fade.min(wlen / 2.0).max(0.02);
                format!(
                    ",afade=t=in:st=0:d={f:.3},afade=t=out:st={:.3}:d={f:.3}",
                    wlen - f
                )
            } else {
                String::new()
            };
            segs.push_str(&format!(
                "[1:a]{chain}atrim={off:.3}:{oend:.3},asetpts=PTS-STARTPTS{wf2},aresample=48000,aformat=channel_layouts=stereo[a{k}];",
                oend = off + wlen
            ));
            pads.push(format!("a{k}"));
            k += 1;
            off += wlen;
            prev = e;
        }
        if prev < probe.duration - 0.001 {
            segs.push_str(&format!("[0:a]atrim={prev:.3},asetpts=PTS-STARTPTS[a{k}];"));
            pads.push(format!("a{k}"));
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

    let inputs: Vec<&Path> = vec![&args.input, audio];
    let mut c = engine::write_job("replace", &inputs, &args.output, vec![argv], g)?;
    c = c.with_extra(json!({
        "audio": audio,
        "audio_offset": args.audio_offset,
        "window": if windows.is_empty() {
            json!(null)
        } else if windows.len() == 1 {
            json!({"at": windows[0].0, "end": windows[0].1})
        } else {
            json!(windows.iter().map(|(s, e)| json!({"at": s, "end": e})).collect::<Vec<_>>())
        },
        "mix": args.mix,
        "duck": args.duck,
        "video_copy": true,
        "loop": args.loop_track,
    }));
    Ok(c)
}

/// The converse swap: keep this video's audio, show another file's frames.
/// The audio is the master clock — output length follows the input. A
/// shorter replacement picture needs `--loop`; a longer one is trimmed.
fn replace_video(
    args: &ReplaceArgs,
    g: &Globals,
    probe: &crate::probe::Probe,
    video: PathBuf,
) -> Result<Contract, Error> {
    if args.mix != 0.0
        || args.duck
        || args.at.is_some()
        || args.dur.is_some()
        || args.fade != 0.0
        || args.audio_offset != 0.0
    {
        return Err(Error::input(
            "replace --video: --audio-offset/--fade/--mix/--duck/--at/--dur are audio-swap options",
        ));
    }
    paths::ensure_input(&video)?;
    let vp = engine::probe_or_err(&video, g)?;
    engine::need_video(&vp, "replace --video")?;
    if !args.loop_track && vp.duration < probe.duration - 0.1 {
        return Err(Error::input(
            "replace --video: the new picture is shorter than the audio — pass --loop to repeat it",
        ));
    }

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    if args.loop_track {
        argv.extend(["-stream_loop", "-1"]);
    }
    argv.extend(["-i"]);
    argv.push(&video);
    argv.extend(["-map", "1:v"]);
    if probe.has_audio {
        argv.extend(["-map", "0:a"]);
    }
    argv.extend([
        "-t",
        &format!("{:.3}", probe.duration),
        "-c:v",
        "libx264",
        "-preset",
        "fast",
        "-crf",
        "18",
        "-pix_fmt",
        "yuv420p",
    ]);
    if probe.has_audio {
        argv.extend(["-c:a", "copy"]);
    }
    argv.push(&args.output);

    let c = engine::write_job(
        "replace",
        &[&args.input, &video],
        &args.output,
        vec![argv],
        g,
    )?;
    Ok(c.with_extra(json!({
        "video": video,
        "audio_copy": probe.has_audio,
        "loop": args.loop_track,
    })))
}

use std::path::Path;

use serde_json::json;

use crate::cli::{ConformArgs, Globals};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: ConformArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    if args.size.is_none()
        && args.fps.is_none()
        && args.lufs.is_none()
        && args.crf.is_none()
        && args.hold.is_none()
        && args.hold_start.is_none()
        && !args.even
        && args.ar.is_none()
        && args.channels.is_none()
    {
        return Err(Error::input(
            "nothing to conform — pass --size WxH, --fps N, --lufs L, --hold SEC, --even, --ar HZ, --channels N",
        ));
    }
    if let Some(r) = args.ar {
        if !(8000..=192000).contains(&r) {
            return Err(Error::input("--ar must be 8000..=192000 Hz"));
        }
    }

    if args.anchor.is_some() && args.pad.is_none() {
        return Err(Error::input("--anchor needs --pad"));
    }
    if args.blur && args.pad.is_some() {
        return Err(Error::input("--blur fills the letterbox — drop --pad"));
    }
    if args.blur && args.size.is_none() {
        return Err(Error::input("--blur needs --size (the target canvas)"));
    }
    let mut vf: Vec<String> = Vec::new();
    let mut blur_fc: Option<String> = None;
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
        if let Some(raw) = &args.pad {
            let c = crate::color::lavfi(raw);
            let (px, py) = match args.anchor.as_deref() {
                None => ("(ow-iw)/2", "(oh-ih)/2"),
                Some("top") => ("(ow-iw)/2", "0"),
                Some("bottom") => ("(ow-iw)/2", "oh-ih"),
                Some("left") => ("0", "(oh-ih)/2"),
                Some("right") => ("ow-iw", "(oh-ih)/2"),
                Some(a) => {
                    return Err(Error::input(format!(
                        "--anchor must be top|bottom|left|right, got '{a}'"
                    )))
                }
            };
            vf.push(format!("pad={w}:{h}:{px}:{py}:{c}"));
        }
        if args.blur {
            let fps_tail = match args.fps {
                Some(f) if (1.0..=240.0).contains(&f) => format!(",fps={f}"),
                Some(_) => return Err(Error::input("--fps must be 1..=240")),
                None => String::new(),
            };
            blur_fc = Some(format!(
                "[0:v]split[cm][cb];[cb]scale={w}:{h}:force_original_aspect_ratio=increase:force_divisible_by=2,crop={w}:{h},gblur=sigma=40[bg];[cm]scale=w={w}:h={h}:force_original_aspect_ratio=decrease:force_divisible_by=2[fg];[bg][fg]overlay=(W-w)/2:(H-h)/2{fps_tail},format=yuv420p[vout]"
            ));
        }
    }
    // --even: floor odd dims — phone/screen captures at odd px can't take
    // yuv420p; --size/--blur already force even dims so skip there
    if args.even && args.size.is_none() {
        vf.push("scale=trunc(iw/2)*2:trunc(ih/2)*2".into());
    }
    if let Some(fps) = args.fps {
        if !(1.0..=240.0).contains(&fps) {
            return Err(Error::input("--fps must be 1..=240"));
        }
        vf.push(format!("fps={fps}"));
    }
    // --hold/--hold-start: tpad clones the edge frame for N seconds —
    // end-card hold or pre-roll without burning a title card
    let mut tpad = String::new();
    let mut held_secs = 0.0f64;
    let src_fps = probe.fps.unwrap_or(30.0).max(1.0);
    for (opt, mode, val) in [
        ("start", "start_mode=clone", args.hold_start),
        ("stop", "stop_mode=clone", args.hold),
    ] {
        if let Some(s) = val {
            if !(0.0..=600.0).contains(&s) || s == 0.0 {
                return Err(Error::input("--hold SEC must be 0..600"));
            }
            held_secs += s;
            let frames = (s * src_fps).round().max(1.0) as u32;
            if !tpad.is_empty() {
                tpad.push(':');
            }
            tpad.push_str(&format!("{opt}={frames}:{mode}"));
        }
    }
    if !tpad.is_empty() {
        if !probe.has_video {
            return Err(Error::input("--hold needs a video stream"));
        }
        vf.push(format!("tpad={tpad}"));
    }
    vf.push("format=yuv420p".into());

    let mut argv = ffmpeg_base(g.progress);
    argv.extend(["-i".to_string(), args.input.display().to_string()]);
    if probe.has_video {
        if let Some(fc) = blur_fc {
            argv.extend(["-filter_complex".to_string(), fc]);
            argv.extend(["-map".to_string(), "[vout]".to_string()]);
        } else {
            argv.extend(["-vf".to_string(), vf.join(",")]);
        }
        argv.extend([
            "-c:v".to_string(),
            "libx264".to_string(),
            "-preset".to_string(),
            "fast".to_string(),
            "-crf".to_string(),
            args.crf.unwrap_or(18).to_string(),
        ]);
    }
    if let Some(c) = args.crf {
        if c > 51 {
            return Err(Error::input("--crf must be 0..=51"));
        }
    }
    if probe.has_audio {
        // loudnorm upsamples internally — resample back AFTER it.
        // --ar overrides the broadcast-48k default (44100 podcasts, 96000 masters)
        let rate = args.ar.unwrap_or(48000);
        let mut af = String::new();
        if let Some(l) = args.lufs {
            if !(-70.0..=-5.0).contains(&l) {
                return Err(Error::input("--lufs must be -70..=-5 (e.g. -14)"));
            }
            af.push_str(&format!("loudnorm=I={l:.1}:TP=-1.5:LRA=11,"));
        }
        let layout = match args.channels.unwrap_or(2) {
            1 => "mono",
            2 => "stereo",
            n => return Err(Error::input(format!("--channels is 1 or 2, got {n}"))),
        };
        af.push_str(&format!(
            "aresample={rate},aformat=channel_layouts={layout}"
        ));
        // silence-pad the tail so audio length matches a frame hold
        if held_secs > 0.0 {
            let whole = ((probe.duration + held_secs) * rate as f64).round() as u64;
            af.push_str(&format!(",apad=whole_len={whole}"));
        }
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
        "blur": args.blur,
        "anchor": args.anchor,
        "fps": args.fps,
        "lufs": args.lufs,
        "hold": args.hold,
        "hold_start": args.hold_start,
        "ar": args.ar,
        "channels": args.channels,
    }));
    Ok(c)
}

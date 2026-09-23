use std::path::Path;

use crate::cli::{BrollArgs, FitMode, Globals};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::paths;
use crate::time;

pub fn run(args: BrollArgs, g: &Globals) -> Result<Contract, Error> {
    if args.duration <= 0.0 || !args.duration.is_finite() {
        return Err(Error::input("--duration must be > 0"));
    }
    let a = engine::probe_or_err(&args.input, g)?;
    // `end` = cutaway over the tail (--duration back from the last frame);
    // a comma list flashes the same insert at several points.
    let mut windows: Vec<f64> = Vec::new();
    for part in args.at.split(',') {
        let p = part.trim();
        let at = if p == "end" {
            (a.duration - args.duration).max(0.0)
        } else {
            time::parse_time(p)?
        };
        windows.push(at);
    }
    windows.sort_by(|x, y| x.partial_cmp(y).unwrap());
    let at = windows[0];
    engine::need_video(&a, "broll")?;
    let b = engine::probe_or_err(&args.insert, g)?;
    if args.volume.is_some() && !args.audio {
        return Err(Error::input("--volume needs --audio"));
    }
    if args.audio && (args.still || !b.has_audio) {
        return Err(Error::input(
            "--audio needs a video insert with an audio stream",
        ));
    }
    engine::need_video(&b, "broll")?;
    if (args.scale.is_some() || args.margin.is_some()) && args.position.is_none() {
        return Err(Error::input("--scale/--margin need --position"));
    }
    if let Some(s) = args.scale {
        if !(0.05..=0.8).contains(&s) {
            return Err(Error::input("--scale must be 0.05..=0.8 of the frame"));
        }
    }
    if let Some(s) = windows.iter().find(|s| **s >= a.duration) {
        return Err(Error::input(format!(
            "--at {s} is past A-roll duration {:.3}s",
            a.duration
        )));
    }
    let ends: Vec<f64> = windows
        .iter()
        .map(|s| (s + args.duration).min(a.duration))
        .collect();
    let end = ends[0];
    let w = paths::even(a.width.unwrap_or(1280)).max(2);
    let h = paths::even(a.height.unwrap_or(720)).max(2);

    let pip = args.position.is_some();
    let (ox, oy) = match &args.position {
        Some(p) => crate::verbs::overlay::overlay_xy(p, args.margin.unwrap_or(24))?,
        None => ("0".to_string(), "0".to_string()),
    };
    let prep = if pip {
        let sw = (w as f64 * args.scale.unwrap_or(0.30)).round() as u32;
        let sw = paths::even(sw).max(16);
        format!("scale={sw}:-2")
    } else {
        match args.fit {
            FitMode::Crop => format!(
                "scale={w}:{h}:force_original_aspect_ratio=increase,crop={w}:{h}"
            ),
            FitMode::Pad => format!(
                "scale={w}:{h}:force_original_aspect_ratio=decrease,pad={w}:{h}:(ow-iw)/2:(oh-ih)/2:black"
            ),
            // Blurred copy of B fills the A frame; the insert sits centered on it.
            FitMode::Blur => format!(
                "split[bb0][bf0];[bb0]scale={w}:{h}:force_original_aspect_ratio=increase,crop={w}:{h},gblur=sigma=30[bgb];[bf0]scale={w}:{h}:force_original_aspect_ratio=decrease[bfg];[bgb][bfg]overlay=(W-w)/2:(H-h)/2"
            ),
        }
    };

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.push("-i");
    argv.push(&args.insert);
    // Shift B so its first frame lands at --at on A's timeline. Without this,
    // a short insert (e.g. 0.5s B at --at 1) has already EOF'd and overlay
    // freezes the last frame for the whole window.
    let still_pre = if args.still {
        format!(
            "loop=loop=-1:size=1,fps={:.3},",
            a.fps.unwrap_or(30.0).max(1.0)
        )
    } else if args.loop_insert {
        // Buffer the whole insert and replay it: without this a short insert
        // hits EOF mid-window and overlay freezes the last frame.
        let b = engine::probe_or_err(&args.insert, g)?;
        let bfps = b.fps.unwrap_or(30.0).max(1.0);
        let frames = (b.duration * bfps).ceil().max(1.0) as u64;
        format!("loop=loop=-1:size={frames},setpts=N/({bfps}*TB),fps={bfps:.3},")
    } else {
        String::new()
    };
    let win = end - at;
    let pix = if args.fade > 0.0 || args.opacity.is_some() {
        "rgba"
    } else {
        "yuv420p"
    };
    let alpha = match args.opacity {
        Some(op) => {
            if !(1.0..=100.0).contains(&op) {
                return Err(Error::input("--opacity must be 1..=100"));
            }
            format!(",colorchannelmixer=aa={:.3}", op / 100.0)
        }
        None => String::new(),
    };
    let fade_chain_of = |s: f64, e: f64| -> String {
        if args.fade > 0.0 {
            let f = args.fade.min((e - s) / 2.0).max(0.02);
            format!(
                ",fade=t=in:st={s:.3}:d={f:.3}:alpha=1,fade=t=out:st={e3:.3}:d={f:.3}:alpha=1",
                e3 = e - f
            )
        } else {
            String::new()
        }
    };
    let fade_chain = fade_chain_of(at, end);
    let border_pre = match args.border {
        Some(b) if (1..=200).contains(&b) => format!(
            ",pad=iw+{d}:ih+{d}:{b}:{b}:{col}",
            d = b * 2,
            col = match args.border_color.as_deref() {
                Some(c) => crate::color::lavfi(c),
                None => "white".to_string(),
            }
        ),
        Some(_) => return Err(Error::input("--border must be 1..=200 px")),
        None => String::new(),
    };
    if args.border.is_some() && !pip {
        return Err(Error::input("--border needs --position (PiP mode)"));
    }
    let mut fc = String::new();
    if windows.len() == 1 {
        fc.push_str(&format!(
            "[1:v]{still_pre}{prep},setsar=1{border_pre},format={pix}{alpha},setpts=PTS-STARTPTS+{at:.3}/TB{fade_chain}[br];[0:v][br]overlay={ox}:{oy}:eof_action=repeat:enable='between(t,{at:.3},{end:.3})'[vout]"
        ));
    } else {
        // One overlay branch per window so the insert restarts at every point.
        let n = windows.len();
        let mut split = String::from("[1:v]split=");
        split.push_str(&n.to_string());
        for i in 0..n {
            split.push_str(&format!("[bs{i}]"));
        }
        fc.push_str(&split);
        for (i, (s, e)) in windows.iter().zip(&ends).enumerate() {
            fc.push_str(&format!(
                ";[bs{i}]{still_pre}{prep},setsar=1{border_pre},format={pix}{alpha},setpts=PTS-STARTPTS+{s:.3}/TB{}[br{i}]",
                fade_chain_of(*s, *e)
            ));
        }
        for (i, (s, e)) in windows.iter().zip(&ends).enumerate() {
            let src = if i == 0 {
                "0:v".to_string()
            } else {
                format!("x{}", i - 1)
            };
            let out = if i + 1 == n {
                "vout".to_string()
            } else {
                format!("x{i}")
            };
            fc.push_str(&format!(
                ";[{src}][br{i}]overlay={ox}:{oy}:eof_action=repeat:enable='between(t,{s:.3},{e:.3})'[{out}]"
            ));
        }
    }
    if args.audio {
        if let Some(v) = args.volume {
            if !(0.0..=4.0).contains(&v) {
                return Err(Error::input("--volume: use 0..=4 (linear)"));
            }
        }
        let vol = match args.volume {
            Some(v) => format!(",volume={v:.4}"),
            None => String::new(),
        };
        if windows.len() == 1 {
            let at_ms = (at * 1000.0) as u64;
            fc.push_str(&format!(
                ";[1:a]atrim=duration={win:.3},asetpts=PTS-STARTPTS{vol},adelay={at_ms}:all=1[ba]"
            ));
            if a.has_audio {
                fc.push_str(";[0:a][ba]amix=inputs=2:duration=first:normalize=0[aout]");
            } else {
                fc.push_str(";[ba]anull[aout]");
            }
        } else {
            let mut wet = String::new();
            for (i, (s, e)) in windows.iter().zip(&ends).enumerate() {
                let w = *e - *s;
                let ms = (*s * 1000.0) as u64;
                wet.push_str(&format!(
                    ";[1:a]atrim=duration={w:.3},asetpts=PTS-STARTPTS{vol},adelay={ms}:all=1[ba{i}]"
                ));
            }
            fc.push_str(&wet);
            let mut mix = String::new();
            for i in 0..windows.len() {
                mix.push_str(&format!("[ba{i}]"));
            }
            if a.has_audio {
                fc.push_str(&format!(
                    ";[0:a]{mix}amix=inputs={}:duration=first:normalize=0[aout]",
                    windows.len() + 1
                ));
            } else {
                fc.push_str(&format!(
                    ";{mix}amix=inputs={}:duration=longest:normalize=0[aout]",
                    windows.len()
                ));
            }
        }
    }
    argv.extend(["-filter_complex", &fc, "-map", "[vout]"]);
    if args.audio {
        argv.extend(["-map", "[aout]", "-c:a", "aac"]);
    } else if a.has_audio {
        argv.extend(["-map", "0:a", "-c:a", "copy"]);
    }
    argv.extend(["-c:v", "libx264", "-preset", "fast", "-crf", "18"]);
    // Delayed B can outlast A on the overlay timeline; pin output to A-roll length.
    argv.extend(["-t", &format!("{:.3}", a.duration)]);
    argv.push(&args.output);

    let inputs: Vec<&Path> = vec![&args.input, &args.insert];
    let mut c = engine::write_job("broll", &inputs, &args.output, vec![argv], g)?;
    c = c.with_extra(serde_json::json!({
        "at": at,
        "end": end,
        "insert": paths::display(&args.insert),
        "still": args.still,
        "motion": args.motion.map(|_| "kenburns"),
        "fit": match args.fit {
            FitMode::Crop => "crop",
            FitMode::Pad => "pad",
            FitMode::Blur => "blur",
        },
    }));
    Ok(c)
}

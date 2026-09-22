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
    let at = time::parse_time(&args.at)?;
    let a = engine::probe_or_err(&args.input, g)?;
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
    if at >= a.duration {
        return Err(Error::input(format!(
            "--at {at} is past A-roll duration {:.3}s",
            a.duration
        )));
    }
    let end = (at + args.duration).min(a.duration);
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
    } else {
        String::new()
    };
    let win = end - at;
    let (pix, fade_chain) = if args.fade > 0.0 {
        let f = args.fade.min(win / 2.0).max(0.02);
        (
            "rgba",
            format!(
                ",fade=t=in:st={at:.3}:d={f:.3}:alpha=1,fade=t=out:st={:.3}:d={f:.3}:alpha=1",
                end - f
            ),
        )
    } else {
        ("yuv420p", String::new())
    };
    let mut fc = format!(
        "[1:v]{still_pre}{prep},setsar=1,format={pix},setpts=PTS-STARTPTS+{at:.3}/TB{fade_chain}[br];[0:v][br]overlay={ox}:{oy}:eof_action=repeat:enable='between(t,{at:.3},{end:.3})'[vout]"
    );
    if args.audio {
        if let Some(v) = args.volume {
            if !(0.0..=4.0).contains(&v) {
                return Err(Error::input("--volume: use 0..=4 (linear)"));
            }
        }
        let at_ms = (at * 1000.0) as u64;
        let vol = match args.volume {
            Some(v) => format!(",volume={v:.4}"),
            None => String::new(),
        };
        fc.push_str(&format!(
            ";[1:a]atrim=duration={win:.3},asetpts=PTS-STARTPTS{vol},adelay={at_ms}:all=1[ba]"
        ));
        if a.has_audio {
            fc.push_str(";[0:a][ba]amix=inputs=2:duration=first:normalize=0[aout]");
        } else {
            fc.push_str(";[ba]anull[aout]");
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

use std::path::Path;

use serde_json::json;

use crate::cli::{Globals, KeyArgs, KeyMode};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::paths;

/// Green-screen composite: remove `--color` from the foreground and lay it
/// over a background image or video (sized to the foreground canvas). The
/// foreground's own audio is kept; duration follows the foreground.
pub fn run(args: KeyArgs, g: &Globals) -> Result<Contract, Error> {
    if matches!(args.mode, Some(KeyMode::Matte)) {
        return matte(&args, g);
    }
    if !(0.0..=1.0).contains(&args.similarity) {
        return Err(Error::input("--similarity must be 0..=1"));
    }
    if !(0.0..=1.0).contains(&args.blend) {
        return Err(Error::input("--blend must be 0..=1"));
    }
    let color = parse_color(&args.color)?;

    let fg = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&fg, "key")?;
    let bg_path = args
        .bg
        .as_ref()
        .ok_or_else(|| Error::input("key needs --bg FILE (unless --mode matte)"))?;
    paths::ensure_input(bg_path)?;
    let bg = engine::probe_or_err(bg_path, g)?;
    if !bg.has_video {
        return Err(Error::input("key: --bg has no picture"));
    }

    let w = paths::even(fg.width.unwrap_or(1280));
    let h = paths::even(fg.height.unwrap_or(720));
    let fps = fg.fps.unwrap_or(30.0);
    // A still or too-short background gets looped to the foreground length.
    let bg_is_still = bg.duration < fg.duration - 0.1;

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    if bg_is_still {
        argv.extend(["-loop", "1", "-t"]);
        argv.push(format!("{:.3}", fg.duration));
    }
    argv.extend(["-i"]);
    argv.push(bg_path);

    let despill = if args.despill {
        ",despill=type=green"
    } else {
        ""
    };
    // colorkey removes a hue; lumakey removes the luma band
    // [threshold±similarity] (bright sky, whiteboard paper, dark backdrops)
    let keyer = match args.mode.unwrap_or(KeyMode::Color) {
        KeyMode::Color => format!(
            "colorkey={color}:{:.3}:{:.3}{despill}",
            args.similarity, args.blend
        ),
        KeyMode::Luma => {
            let t = args.threshold.unwrap_or(0.5);
            if !(0.0..=1.0).contains(&t) {
                return Err(Error::input("--threshold must be 0..=1"));
            }
            format!(
                "lumakey=threshold={t:.3}:tolerance={:.3}:softness={:.3}",
                args.similarity, args.blend
            )
        }
        KeyMode::Matte => unreachable!("matte returns early"),
    };
    let enable = match &args.at {
        Some(s) => format!(
            ":enable='{}'",
            crate::time::enable_expr(s, args.dur, fg.duration)?
        ),
        None => String::new(),
    };
    let fc = format!(
        "[0:v]fps={fps:.3},format=yuv420p,{keyer}[keyed];\
         [1:v]scale={w}:{h}:force_original_aspect_ratio=increase,crop={w}:{h},setsar=1,fps={fps:.3},format=yuv420p[bg];\
         [bg][keyed]overlay=0:0:shortest=1{enable}[vout]",
    );
    argv.extend(["-filter_complex", &fc, "-map", "[vout]"]);
    if fg.has_audio {
        argv.extend(["-map", "0:a", "-c:a", "aac"]);
    }
    argv.extend([
        "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
    ]);
    argv.push(&args.output);

    let inputs: Vec<&Path> = vec![&args.input, bg_path];
    let mut c = engine::write_job("key", &inputs, &args.output, vec![argv], g)?;
    c = c.with_extra(json!({
        "mode": match args.mode.unwrap_or(KeyMode::Color) { KeyMode::Color => "color", KeyMode::Luma => "luma", KeyMode::Matte => "matte" },
        "color": color,
        "threshold": args.threshold,
        "similarity": args.similarity,
        "blend": args.blend,
        "bg_is_still": bg_is_still,
    }));
    Ok(c)
}

/// External matte: the mask's luma becomes the foreground's alpha
/// (alphamerge). Encodes prores 4444 so the channel survives to an
/// editor — use `premult` next for straight-alpha handoffs.
fn matte(args: &KeyArgs, g: &Globals) -> Result<Contract, Error> {
    let mask = args
        .mask
        .as_ref()
        .ok_or_else(|| Error::input("key --mode matte needs --mask FILE (grayscale matte)"))?;
    let fg = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&fg, "key")?;
    paths::ensure_input(mask)?;
    let m = engine::probe_or_err(mask, g)?;
    if !m.has_video {
        return Err(Error::input("key --mode matte: --mask has no picture"));
    }
    let w = paths::even(fg.width.unwrap_or(1280));
    let h = paths::even(fg.height.unwrap_or(720));
    let fps = fg.fps.unwrap_or(30.0);

    let fc = format!(
        "[0:v]fps={fps:.3},format=rgba[c];[1:v]fps={fps:.3},format=gray,scale={w}:{h}[m];[c][m]alphamerge[v]",
    );
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.push("-i");
    argv.push(mask);
    argv.extend(["-filter_complex", &fc, "-map", "[v]", "-map", "0:a?"]);
    argv.extend([
        "-c:v",
        "prores_ks",
        "-profile:v",
        "4444",
        "-pix_fmt",
        "yuva444p10le",
    ]);
    if fg.has_audio {
        argv.extend(["-c:a", "copy"]);
    }
    argv.push(&args.output);

    let inputs: Vec<&Path> = vec![&args.input, mask];
    let c = engine::write_job("key", &inputs, &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "mode": "matte",
        "mask": paths::display(mask),
        "width": w,
        "height": h,
    })))
}

fn parse_color(s: &str) -> Result<String, Error> {
    let hex = s
        .strip_prefix("0x")
        .or_else(|| s.strip_prefix('#'))
        .unwrap_or(s);
    if hex.len() == 6 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
        Ok(format!("0x{hex}"))
    } else {
        Err(Error::input(format!(
            "--color must be RRGGBB hex (e.g. 0x00ff00), got {s:?}"
        )))
    }
}

use std::path::Path;

use serde_json::json;

use crate::cli::{Globals, KeyArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::paths;

/// Green-screen composite: remove `--color` from the foreground and lay it
/// over a background image or video (sized to the foreground canvas). The
/// foreground's own audio is kept; duration follows the foreground.
pub fn run(args: KeyArgs, g: &Globals) -> Result<Contract, Error> {
    if !(0.0..=1.0).contains(&args.similarity) {
        return Err(Error::input("--similarity must be 0..=1"));
    }
    if !(0.0..=1.0).contains(&args.blend) {
        return Err(Error::input("--blend must be 0..=1"));
    }
    let color = parse_color(&args.color)?;

    let fg = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&fg, "key")?;
    paths::ensure_input(&args.bg)?;
    let bg = engine::probe_or_err(&args.bg, g)?;
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
    argv.push(&args.bg);

    let despill = if args.despill {
        ",despill=type=green"
    } else {
        ""
    };
    let enable = match &args.at {
        Some(s) => format!(
            ":enable='{}'",
            crate::time::enable_expr(s, args.dur, fg.duration)?
        ),
        None => String::new(),
    };
    let fc = format!(
        "[0:v]fps={fps:.3},format=yuv420p,colorkey={color}:{:.3}:{:.3}{despill}[keyed];\
         [1:v]scale={w}:{h}:force_original_aspect_ratio=increase,crop={w}:{h},setsar=1,fps={fps:.3},format=yuv420p[bg];\
         [bg][keyed]overlay=0:0:shortest=1{enable}[vout]",
        args.similarity, args.blend
    );
    argv.extend(["-filter_complex", &fc, "-map", "[vout]"]);
    if fg.has_audio {
        argv.extend(["-map", "0:a", "-c:a", "aac"]);
    }
    argv.extend([
        "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
    ]);
    argv.push(&args.output);

    let inputs: Vec<&Path> = vec![&args.input, &args.bg];
    let mut c = engine::write_job("key", &inputs, &args.output, vec![argv], g)?;
    c = c.with_extra(json!({
        "color": color,
        "similarity": args.similarity,
        "blend": args.blend,
        "bg_is_still": bg_is_still,
    }));
    Ok(c)
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

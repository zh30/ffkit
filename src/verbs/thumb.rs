use serde_json::json;

use std::path::Path;

use crate::cli::{Globals, ThumbArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::time::{fmt_time, parse_time};

pub fn run(args: ThumbArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_video {
        return Err(Error::input("thumb: input has no video stream"));
    }
    let ext = args
        .output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !matches!(ext.as_str(), "jpg" | "jpeg" | "png" | "webp") {
        return Err(Error::input(format!(
            "thumb output must be .jpg/.png/.webp, got .{ext}"
        )));
    }
    if args.at.is_some() && args.frame.is_some() {
        return Err(Error::input("pass --at or --frame, not both"));
    }
    if args.scenes {
        if args.at.is_some() || args.frame.is_some() || args.count.is_some() {
            return Err(Error::input(
                "--scenes finds its own frames; drop --at/--frame/--count",
            ));
        }
        let stem = args
            .output
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("thumb");
        let parent = args.output.parent().filter(|p| !p.as_os_str().is_empty());
        let pattern = format!("{stem}_%02d.{ext}");
        let out = match parent {
            Some(d) => d.join(&pattern).display().to_string(),
            None => pattern.clone(),
        };
        let scale = args
            .width
            .map(|w| format!(",scale={w}:-2"))
            .unwrap_or_default();
        let mut argv = ffmpeg_base(g.progress);
        argv.push("-i");
        argv.push(&args.input);
        // eq(n,0) keeps frame 0 — a no-cut clip still yields one still.
        argv.extend(["-vf", &format!("select='eq(n,0)+gt(scene,0.35)'{scale}")]);
        // -fps_mode appeared in ffmpeg 5; 4.x spells it -vsync.
        if engine::ffmpeg_major().unwrap_or(6) >= 5 {
            argv.extend(["-fps_mode", "passthrough"]);
        } else {
            argv.extend(["-vsync", "passthrough"]);
        }
        argv.push(&out);
        let first = Path::new(&out.replace("%02d", "01")).to_path_buf();
        let c = engine::write_job("thumb", &[&args.input], &first, vec![argv], g)?;
        return Ok(c.with_extra(json!({ "scenes": true })));
    }

    if let Some(n) = args.count {
        if args.at.is_some() || args.frame.is_some() {
            return Err(Error::input("--count spreads frames; drop --at/--frame"));
        }
        if !(1..=50).contains(&n) {
            return Err(Error::input("--count must be 1..=50"));
        }
        let stem = args
            .output
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("thumb");
        let parent = args.output.parent().filter(|p| !p.as_os_str().is_empty());
        // fps=(n-0.5)/dur spaces the picks across ~95% of the clip; a higher
        // rate lands the last pts past EOF and drops a frame.
        let fps = (n as f64 - 0.5).max(0.5) / probe.duration.max(0.05);
        let files: Vec<String> = (1..=n)
            .map(|k| {
                let name = format!("{stem}_{k:02}.{ext}");
                match parent {
                    Some(d) => d.join(name).display().to_string(),
                    None => name,
                }
            })
            .collect();
        let mut argv = ffmpeg_base(g.progress);
        argv.push("-i");
        argv.push(&args.input);
        let scale = args
            .width
            .map(|w| format!(",scale={w}:-2"))
            .unwrap_or_default();
        argv.extend(["-vf", &format!("fps={fps:.6}{scale}")]);
        argv.extend(["-frames:v", &n.to_string()]);
        argv.push(files[0].replacen("_01.", "_%02d.", 1));
        // write_job verifies files[0] (the first still); extras lists them all.
        let c = engine::write_job("thumb", &[&args.input], Path::new(&files[0]), vec![argv], g)?;
        return Ok(c.with_extra(json!({ "count": n, "files": files })));
    }
    let mut argv = ffmpeg_base(g.progress);
    match (args.at.as_deref(), args.frame) {
        (Some(at), None) => {
            let secs = parse_time(at)?;
            // seek before input: fast + frame-accurate enough for a cover grab
            argv.extend(["-ss", &fmt_time(secs)]);
            argv.push("-i");
            argv.push(&args.input);
        }
        (None, Some(f)) => {
            argv.push("-i");
            argv.push(&args.input);
            argv.extend(["-vf", &format!("select='eq(n,{f})'")]);
        }
        (None, None) => {
            let at = probe.duration * 0.1;
            argv.extend(["-ss", &fmt_time(at)]);
            argv.push("-i");
            argv.push(&args.input);
        }
        _ => unreachable!(),
    }
    if let Some(w) = args.width {
        argv.extend(["-vf", &format!("scale={w}:-2")]);
    }
    argv.extend(["-frames:v", "1"]);
    if matches!(ext.as_str(), "jpg" | "jpeg") {
        argv.extend(["-q:v", "2"]);
    }
    argv.push(&args.output);

    let c = engine::write_job("thumb", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "at": args.at.clone(),
        "frame": args.frame,
        "width": probe.width,
        "height": probe.height,
    })))
}

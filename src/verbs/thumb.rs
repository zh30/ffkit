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
    if args.best {
        if args.at.is_some() || args.frame.is_some() || args.count.is_some() || args.scenes {
            return Err(Error::input(
                "--best picks its own frame; drop other selectors",
            ));
        }
        // thumbnail scores each batch of 100 frames by average similarity and
        // emits the most typical one — lands on a clean still even when the
        // footage shakes or a subject blinks through the intro.
        let scale = args
            .width
            .map(|w| format!(",scale={w}:-2"))
            .unwrap_or_default();
        let mut argv = ffmpeg_base(g.progress);
        argv.push("-i");
        argv.push(&args.input);
        argv.extend(["-vf", &format!("thumbnail{scale}"), "-frames:v", "1"]);
        argv.push(&args.output);
        let c = engine::write_job("thumb", &[&args.input], &args.output, vec![argv], g)?;
        return Ok(c.with_extra(json!({ "best": true })));
    }

    if let Some(n) = args.count {
        if args.at.is_some() || args.frame.is_some() {
            return Err(Error::input("--count spreads frames; drop --at/--frame"));
        }
        if !(1..=50).contains(&n) {
            return Err(Error::input("--count must be 1..=50"));
        }
        let t0 = match &args.from {
            Some(s) if s.trim().eq_ignore_ascii_case("end") => probe.duration,
            Some(s) if s.trim().to_ascii_lowercase().starts_with("end-") => {
                probe.duration - parse_time(&s.trim()[4..])?
            }
            Some(s) => parse_time(s)?,
            None => 0.0,
        };
        let t1 = match &args.to {
            Some(s) if s.trim().eq_ignore_ascii_case("end") => probe.duration,
            Some(s) => parse_time(s)?,
            None => probe.duration,
        };
        if t0 < 0.0 || t1 <= t0 || t0 >= probe.duration {
            return Err(Error::input(
                "--from/--to need 0 <= from < to within the input",
            ));
        }
        let span = t1.min(probe.duration) - t0;
        let stem = args
            .output
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("thumb");
        let parent = args.output.parent().filter(|p| !p.as_os_str().is_empty());
        // fps=(n-0.5)/span spaces the picks across ~95% of the window; a higher
        // rate lands the last pts past EOF and drops a frame.
        let fps = (n as f64 - 0.5).max(0.5) / span.max(0.05);
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
        if t0 > 0.0 {
            argv.extend(["-ss", &fmt_time(t0)]);
        }
        argv.push("-i");
        argv.push(&args.input);
        if t1 < probe.duration {
            argv.extend(["-t", &fmt_time(span)]);
        }
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
    // comma --at: one still per timepoint → `<stem>_N.<ext>`
    if let Some(raw) = &args.at {
        if raw.split(',').count() > 1 {
            let mut argv = ffmpeg_base(g.progress);
            let mut files = Vec::new();
            for (i, part) in raw.split(',').enumerate() {
                let secs = crate::time::resolve_frame_at(part.trim(), probe.duration)?;
                let inp = args.input.display().to_string();
                argv.extend(["-ss", &fmt_time(secs), "-i", inp.as_str()]);
                files.push(derive_output(&args.output, i + 1));
            }
            for (i, f) in files.iter().enumerate() {
                argv.extend(["-map", &format!("{i}:v"), "-frames:v", "1"]);
                if matches!(ext.as_str(), "jpg" | "jpeg") {
                    argv.extend(["-q:v", "2"]);
                }
                if let Some(w) = args.width {
                    argv.extend(["-vf", &format!("scale={w}:-2")]);
                }
                argv.push(f.as_str());
            }
            let c = engine::write_job(
                "thumb",
                &[&args.input],
                std::path::Path::new(&files[0]),
                vec![argv],
                g,
            )?;
            let missing: Vec<_> = files
                .iter()
                .skip(1)
                .filter(|f| !std::path::Path::new(f).exists())
                .collect();
            if !missing.is_empty() {
                return Err(Error::output(format!(
                    "thumb: expected outputs missing: {}",
                    missing
                        .iter()
                        .map(|f| f.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                )));
            }
            return Ok(c.with_extra(json!({
                "at": args.at.clone(),
                "files": files,
                "width": probe.width,
                "height": probe.height,
            })));
        }
    }
    let mut argv = ffmpeg_base(g.progress);
    match (args.at.as_deref(), args.frame) {
        (Some(at), None) => {
            let secs = crate::time::resolve_frame_at(at, probe.duration)?;
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

fn derive_output(base: &std::path::Path, i: usize) -> String {
    let stem = base.file_stem().and_then(|s| s.to_str()).unwrap_or("thumb");
    let ext = base.extension().and_then(|e| e.to_str()).unwrap_or("jpg");
    base.with_file_name(format!("{stem}_{i}.{ext}"))
        .to_string_lossy()
        .to_string()
}

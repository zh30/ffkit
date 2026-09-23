use serde_json::json;

use crate::cli::{CoverArgs, Globals};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::time::fmt_time;

pub fn run(args: CoverArgs, g: &Globals) -> Result<Contract, Error> {
    let (frame_w, frame_h) = match &args.size {
        Some(s) => s
            .split_once('x')
            .and_then(|(a, b)| Some((a.parse::<u32>().ok()?, b.parse::<u32>().ok()?)))
            .filter(|(w, h)| *w >= 16 && *h >= 16 && *w % 2 == 0 && *h % 2 == 0)
            .ok_or_else(|| Error::input("--size must be even WxH (min 16x16)"))?,
        None => (1080, 1920),
    };
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "cover")?;
    let vf = if args.blur {
        format!(
            "split[a][b];[a]scale={frame_w}:{frame_h}:force_original_aspect_ratio=increase,crop={frame_w}:{frame_h},gblur=sigma=40[bg];[b]scale={frame_w}:{frame_h}:force_original_aspect_ratio=decrease[fg];[bg][fg]overlay=(W-w)/2:(H-h)/2,setsar=1,format=yuv420p"
        )
    } else {
        format!(
            "scale={frame_w}:{frame_h}:force_original_aspect_ratio=decrease,pad={frame_w}:{frame_h}:(ow-iw)/2:(oh-ih)/2:black,setsar=1,format=yuv420p"
        )
    };
    // comma --at: one cover per timepoint → `<stem>_N.<ext>`
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
                argv.extend([
                    "-map",
                    &format!("{i}:v"),
                    "-frames:v",
                    "1",
                    "-an",
                    "-vf",
                    vf.as_str(),
                ]);
                argv.push(f.as_str());
            }
            let c = engine::write_job(
                "cover",
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
                    "cover: expected outputs missing: {}",
                    missing
                        .iter()
                        .map(|f| f.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                )));
            }
            return Ok(c.with_extra(json!({
                "files": files,
                "frame": format!("{frame_w}x{frame_h}"),
            })));
        }
    }

    let at = match &args.at {
        Some(s) => crate::time::resolve_frame_at(s, probe.duration)?,
        None => 0.0,
    };
    if at > probe.duration && probe.duration > 0.0 {
        return Err(Error::input(format!(
            "--at {at} is past duration {:.3}s",
            probe.duration
        )));
    }

    let mut argv = ffmpeg_base(g.progress);
    if at > 0.0 {
        argv.extend(["-ss", &fmt_time(at)]);
    }
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-frames:v", "1", "-an", "-vf", &vf]);
    argv.push(&args.output);

    let c = engine::write_job("cover", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "at": at,
        "frame": format!("{frame_w}x{frame_h}"),
    })))
}

fn derive_output(base: &std::path::Path, i: usize) -> String {
    let stem = base.file_stem().and_then(|s| s.to_str()).unwrap_or("cover");
    let ext = base.extension().and_then(|e| e.to_str()).unwrap_or("jpg");
    base.with_file_name(format!("{stem}_{i}.{ext}"))
        .to_string_lossy()
        .to_string()
}

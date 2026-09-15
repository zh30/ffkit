use std::path::Path;

use serde_json::json;

use crate::cli::{Globals, TitleArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: TitleArgs, g: &Globals) -> Result<Contract, Error> {
    let text = args.text.trim();
    if text.is_empty() {
        return Err(Error::input("--text is empty"));
    }
    if args.duration <= 0.0 {
        return Err(Error::input("--duration must be > 0"));
    }
    let (x, y) = match args.position.as_str() {
        "center" => ("(W-w)/2", "(H-h)/2"),
        "top" => ("(W-w)/2", "trunc(H*0.18)"),
        other => {
            return Err(Error::input(format!(
                "--position {other}: use center or top"
            )));
        }
    };
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "title")?;
    let until = args.duration.min(probe.duration.max(0.05));
    let font_path = crate::font::resolve(args.font.as_deref().map(Path::new))?;
    let font_bytes = std::fs::read(&font_path)?;
    let vw = probe.width.unwrap_or(1280);
    let img = crate::raster::render_title(text, &font_bytes, vw)?;
    let tmp = tempfile::tempdir().map_err(|e| Error::output(e.to_string()))?;
    let png = tmp.path().join("title.png");
    img.save(&png)
        .map_err(|e| Error::output(format!("write title png: {e}")))?;

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.push("-i");
    argv.push(&png);
    let fc = format!("[0:v][1:v]overlay=x={x}:y={y}:enable='between(t,0,{until:.3})'[vout]");
    argv.extend(["-filter_complex", &fc, "-map", "[vout]"]);
    if probe.has_audio {
        argv.extend(["-map", "0:a", "-c:a", "copy"]);
    }
    argv.extend([
        "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
    ]);
    argv.push(&args.output);

    let c = engine::write_job("title", &[&args.input], &args.output, vec![argv], g)?;
    drop(tmp);
    Ok(c.with_extra(json!({
        "text": text,
        "duration": until,
        "position": args.position,
        "font": font_path.display().to_string(),
    })))
}

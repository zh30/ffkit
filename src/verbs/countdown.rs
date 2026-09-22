use serde_json::json;
use std::path::Path;

use crate::cli::{CountdownArgs, Globals};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

fn parse_hex(c: &str) -> Result<[u8; 3], Error> {
    let c = c.trim_start_matches('#');
    if c.len() != 6 {
        return Err(Error::input("--color must be RRGGBB hex"));
    }
    let b = |i: usize| -> Result<u8, Error> {
        u8::from_str_radix(&c[i..i + 2], 16).map_err(|_| Error::input("--color must be RRGGBB hex"))
    };
    Ok([b(0)?, b(2)?, b(4)?])
}

/// Rasterized 3-2-1(-GO) intro overlay: one PNG input per run, each shown
/// `--each` seconds via `enable='between(t,a,b)'`.
pub fn run(args: CountdownArgs, g: &Globals) -> Result<Contract, Error> {
    if !(1..=10).contains(&args.from) {
        return Err(Error::input("--from must be 1..=10"));
    }
    if args.each <= 0.0 {
        return Err(Error::input("--each must be > 0 seconds"));
    }
    if !(0.25..=8.0).contains(&args.size) {
        return Err(Error::input("--size must be 0.25..8"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "countdown")?;
    let at = args.at.unwrap_or(0.0);
    if at < 0.0 {
        return Err(Error::input("--at must be >= 0"));
    }

    let font_path = crate::font::resolve(args.font.as_deref().map(Path::new))?;
    let font_bytes = std::fs::read(&font_path)?;
    let vw = probe.width.unwrap_or(1280);
    let fg = match &args.color {
        Some(c) => parse_hex(c)?,
        None => [255, 255, 255],
    };

    // Text runs: countdown digits, then optional GO.
    let mut runs: Vec<String> = (1..=args.from).rev().map(|n| n.to_string()).collect();
    if let Some(go) = &args.go {
        runs.push(go.clone());
    }

    let tmp = tempfile::tempdir().map_err(|e| Error::output(e.to_string()))?;
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);

    let mut segs = Vec::new();
    let mut prev = "[0:v]".to_string();
    for (i, text) in runs.iter().enumerate() {
        let img = crate::raster::render_title_styled(text, &font_bytes, vw, fg, args.size as f32)?;
        let png = tmp.path().join(format!("n{i}.png"));
        img.save(&png)
            .map_err(|e| Error::output(format!("write countdown png: {e}")))?;
        argv.push("-i");
        argv.push(png);
        let a = at + i as f64 * args.each;
        let b = a + args.each;
        let label = format!("c{i}");
        segs.push(format!(
            "{prev}[{}:v]overlay=x=(W-w)/2:y=(H-h)/2:enable='between(t,{a:.3},{b:.3})'[{label}]",
            i + 1
        ));
        prev = format!("[{label}]");
    }
    let mut fc = segs.join(";");
    if args.beep {
        let win = runs.len() as f64 * args.each;
        // aevalsrc: 880Hz sine gated to the first 120ms of each tick window
        argv.extend([
            "-f",
            "lavfi",
            "-i",
            &format!(
                "aevalsrc='sin(2*PI*880*t)*lt(mod(t-{at:.3},{ea:.3}),0.12)':d={dd:.3}:s=44100",
                ea = args.each,
                dd = at + win
            ),
        ]);
        let beep_idx = runs.len() + 1;
        if probe.has_audio {
            fc.push_str(&format!(
                ";[0:a][{beep_idx}:a]amix=inputs=2:duration=first[aout]"
            ));
        } else {
            fc.push_str(&format!(";[{beep_idx}:a]anull[aout]"));
        }
    }
    argv.extend(["-filter_complex", &fc, "-map", &prev]);
    if args.beep {
        argv.extend(["-map", "[aout]", "-c:a", "aac"]);
    } else if probe.has_audio {
        argv.extend(["-map", "0:a?", "-c:a", "copy"]);
    }
    argv.extend([
        "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
    ]);
    argv.push(&args.output);

    let c = engine::write_job("countdown", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "from": args.from,
        "each": args.each,
        "at": at,
        "go": args.go,
    })))
}

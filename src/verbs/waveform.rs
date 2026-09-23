use serde_json::json;

use crate::cli::{Globals, WaveformArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: WaveformArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_audio {
        return Err(Error::input("waveform: input has no audio stream"));
    }
    let (w, h) = parse_size(&args.size)?;

    let raw = args.color.as_deref().unwrap_or("ffffff");
    // ffmpeg colour spec wants 0xRRGGBB; bare hex is ambiguous.
    let color = crate::color::lavfi(raw);
    let color = color.as_str();
    let sc = match &args.scale {
        Some(s) => {
            if !["lin", "log", "sqrt", "cbrt"].contains(&s.as_str()) {
                return Err(Error::input("--scale must be lin|log|sqrt|cbrt"));
            }
            format!(":scale={s}")
        }
        None => String::new(),
    };
    // --at/--dur: crop the rendered wave to the window, then stretch to --size.
    let flt = if args.peak { ":filter=peak" } else { "" };
    let dr = if args.full { ":draw=full" } else { "" };
    let (bg_pre, bg_post) = match &args.bg {
        Some(b) => (
            format!("color=c={c}:s={w}x{h}[bgr];", c = crate::color::lavfi(b)),
            ";[bgr][v]overlay=0:0[wout]".to_string(),
        ),
        None => (String::new(), String::new()),
    };
    let sp = if args.split { ":split_channels=1" } else { "" };
    // Multi-window: comma --at renders one PNG per window (`<stem>_N.png`).
    let multi = args
        .at
        .as_deref()
        .map(|s| s.split(',').count() > 1)
        .unwrap_or(false);
    let (win, outs) = match &args.at {
        Some(raw) if !multi => {
            let at = crate::time::resolve_at(raw, args.dur, probe.duration)?;
            if !(0.0..probe.duration).contains(&at) {
                return Err(Error::input("--at is outside the input"));
            }
            let end = args
                .dur
                .map(|d| at + d)
                .unwrap_or(probe.duration)
                .min(probe.duration);
            (
                format!(
                    ";[w0]crop=w=iw*{fw:.6}:x=iw*{fx:.6}:h=ih,scale={w}:{h}[v]",
                    fw = (end - at) / probe.duration,
                    fx = at / probe.duration
                ),
                vec![crate::paths::display(&args.output)],
            )
        }
        Some(raw) => {
            let ws = crate::time::enable_windows(raw, args.dur, probe.duration)?;
            let n = ws.len();
            let mut tail = String::new();
            if args.bg.is_some() {
                // [v] feeds bg_post's overlay, then the composited [wout] splits
                tail.push_str(";[w0]copy[v]");
            }
            let head = if args.bg.is_some() {
                format!(";[wout]split={n}")
            } else {
                format!(";[w0]split={n}")
            };
            tail.push_str(&head);
            for i in 0..n {
                tail.push_str(&format!("[sp{i}]"));
            }
            let mut files = Vec::new();
            for (i, &(s, e)) in ws.iter().enumerate() {
                tail.push_str(&format!(
                    ";[sp{i}]crop=w=iw*{fw:.6}:x=iw*{fx:.6}:h=ih,scale={w}:{h}[o{i}]",
                    fw = (e - s) / probe.duration,
                    fx = s / probe.duration
                ));
                files.push(derive_output(&args.output, i + 1));
            }
            (tail, files)
        }
        None => {
            if args.dur.is_some() {
                return Err(Error::input("--dur needs --at"));
            }
            (
                ";[w0]copy[v]".to_string(),
                vec![crate::paths::display(&args.output)],
            )
        }
    };
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend([
        "-filter_complex",
        &format!(
            "{bg_pre}[0:a]showwavespic=s={w}x{h}:colors={color}{flt}{sp}{sc}{dr}[w0]{win}{bg_post}"
        ),
        "-map",
        if outs.len() > 1 {
            "[o0]"
        } else if args.bg.is_some() {
            "[wout]"
        } else {
            "[v]"
        },
        "-frames:v",
        "1",
        "-update",
        "1",
    ]);
    if outs.len() > 1 {
        // first output already mapped above; map the rest
    }
    argv.push(outs[0].as_str());
    for (i, f) in outs.iter().enumerate().skip(1) {
        argv.extend(["-map", &format!("[o{i}]"), "-frames:v", "1", "-update", "1"]);
        argv.push(f.as_str());
    }

    let out0 = std::path::PathBuf::from(&outs[0]);
    let mut c = engine::write_job("waveform", &[&args.input], &out0, vec![argv], g)?;
    if outs.len() > 1 {
        let missing: Vec<_> = outs
            .iter()
            .skip(1)
            .filter(|f| !std::path::Path::new(f).exists())
            .collect();
        if !missing.is_empty() {
            return Err(Error::output(format!(
                "waveform: expected outputs missing: {}",
                missing
                    .iter()
                    .map(|f| f.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )));
        }
    }
    c = c.with_extra(json!({
        "size": format!("{w}x{h}"),
        "color": raw,
    }));
    if outs.len() > 1 {
        let mut m = serde_json::Map::new();
        m.insert("outputs".to_string(), json!(outs));
        c = c.with_extra(serde_json::Value::Object(m));
    }
    Ok(c)
}

fn derive_output(base: &std::path::Path, i: usize) -> String {
    let stem = base
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("waveform");
    let ext = base.extension().and_then(|e| e.to_str()).unwrap_or("png");
    base.with_file_name(format!("{stem}_{i}.{ext}"))
        .to_string_lossy()
        .to_string()
}

fn parse_size(s: &str) -> Result<(u32, u32), Error> {
    let (w, h) = s
        .split_once('x')
        .ok_or_else(|| Error::input("--size must look like WxH (e.g. 1920x540)"))?;
    let w: u32 = w.parse().map_err(|_| Error::input("--size WxH integers"))?;
    let h: u32 = h.parse().map_err(|_| Error::input("--size WxH integers"))?;
    if !(16..=8192).contains(&w) || !(16..=8192).contains(&h) {
        return Err(Error::input("--size sides must be 16..8192"));
    }
    Ok((w, h))
}

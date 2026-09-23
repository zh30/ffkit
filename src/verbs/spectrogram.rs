use serde_json::json;

use crate::cli::{Globals, SpectrogramArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: SpectrogramArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_audio {
        return Err(Error::input("spectrogram: input has no audio stream"));
    }
    let color = match &args.color {
        Some(c) => {
            let ok = [
                "channel",
                "intensity",
                "rainbow",
                "moreland",
                "nebulae",
                "fire",
                "fiery",
                "fruit",
                "cool",
                "magma",
                "green",
                "viridis",
                "plasma",
                "cividis",
                "terrain",
            ];
            if !ok.contains(&c.as_str()) {
                return Err(Error::input(format!(
                    "--color: use one of {}",
                    ok.join("|")
                )));
            }
            format!(":color={c}")
        }
        None => String::new(),
    };
    let sep = if args.separate { ":mode=separate" } else { "" };
    let (w, h) = args
        .size
        .split_once('x')
        .and_then(|(w, h)| w.parse::<u32>().ok().zip(h.parse::<u32>().ok()))
        .filter(|(w, h)| (16..=8192).contains(w) && (16..=8192).contains(h))
        .ok_or_else(|| Error::input("--size must look like WxH (e.g. 1920x1080)"))?;

    // Multi-window: comma --at renders one PNG per window (`<stem>_N.png`).
    let multi = args
        .at
        .as_deref()
        .map(|s| s.split(',').count() > 1)
        .unwrap_or(false);
    let (slice, outs) = match &args.at {
        Some(raw) if !multi => {
            let at = crate::time::resolve_at(raw, args.dur, probe.duration)?;
            let s = match args.dur {
                Some(d) => format!("atrim={at:.3}:{e:.3},asetpts=PTS-STARTPTS,", e = at + d),
                None => format!("atrim=start={at:.3},asetpts=PTS-STARTPTS,"),
            };
            (s, vec![crate::paths::display(&args.output)])
        }
        Some(raw) => {
            let ws = crate::time::enable_windows(raw, args.dur, probe.duration)?;
            let n = ws.len();
            let mut s = format!("asplit={n}");
            let mut files = Vec::new();
            for i in 0..n {
                s.push_str(&format!("[as{i}]"));
            }
            for (i, &(a, b)) in ws.iter().enumerate() {
                s.push_str(&format!(
                    ";[as{i}]atrim={a:.3}:{b:.3},asetpts=PTS-STARTPTS[a{i}]"
                ));
                files.push(derive_output(&args.output, i + 1));
            }
            (s, files)
        }
        None => {
            if args.dur.is_some() {
                return Err(Error::input("--dur needs --at"));
            }
            (String::new(), vec![crate::paths::display(&args.output)])
        }
    };
    let lg = if args.no_legend { "0" } else { "1" };
    let sc = match &args.scale {
        Some(s) => {
            if !["lin", "sqrt", "cbrt", "log", "4thrt", "5thrt"].contains(&s.as_str()) {
                return Err(Error::input("--scale: lin|sqrt|cbrt|log|4thrt|5thrt"));
            }
            format!(":scale={s}")
        }
        None => String::new(),
    };
    let multi_tail = if multi {
        let mut t = String::new();
        for i in 0..outs.len() {
            t.push_str(&format!(
                ";[a{i}]showspectrumpic=s={w}x{h}:legend={lg}{color}{sc}{sep}[v{i}]"
            ));
        }
        t
    } else {
        String::new()
    };
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend([
        "-filter_complex",
        &format!(
            "[0:a]{slice}{core}{multi_tail}",
            core = if multi {
                String::new()
            } else {
                format!("showspectrumpic=s={w}x{h}:legend={lg}{color}{sc}{sep}[v]")
            },
            multi_tail = multi_tail
        ),
        "-map",
        if multi { "[v0]" } else { "[v]" },
        "-frames:v",
        "1",
        "-update",
        "1",
    ]);
    argv.push(outs[0].as_str());
    for (i, f) in outs.iter().enumerate().skip(1) {
        argv.extend(["-map", &format!("[v{i}]"), "-frames:v", "1", "-update", "1"]);
        argv.push(f.as_str());
    }

    let out0 = std::path::PathBuf::from(&outs[0]);
    let mut c = engine::write_job("spectrogram", &[&args.input], &out0, vec![argv], g)?;
    if outs.len() > 1 {
        let missing: Vec<_> = outs
            .iter()
            .skip(1)
            .filter(|f| !std::path::Path::new(f).exists())
            .collect();
        if !missing.is_empty() {
            return Err(Error::output(format!(
                "spectrogram: expected outputs missing: {}",
                missing
                    .iter()
                    .map(|f| f.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )));
        }
    }
    c = c.with_extra(json!({ "size": format!("{w}x{h}") }));
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
        .unwrap_or("spectrogram");
    let ext = base.extension().and_then(|e| e.to_str()).unwrap_or("png");
    base.with_file_name(format!("{stem}_{i}.{ext}"))
        .to_string_lossy()
        .to_string()
}

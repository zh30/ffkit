use serde_json::json;

use crate::cli::{Globals, ScopeArgs, ScopeMode};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::time::enable_expr;

pub fn run(args: ScopeArgs, g: &Globals) -> Result<Contract, Error> {
    if !(0.1..=0.6).contains(&args.size) {
        return Err(Error::input(
            "--size must be 0.1..=0.6 (fraction of frame width)",
        ));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "scope")?;
    if matches!(args.mode, ScopeMode::Loud) && !probe.has_audio {
        return Err(Error::input("scope --mode loud needs an audio track"));
    }
    let w = probe.width.unwrap_or(0) as f64;
    let sw = (w * args.size) as u32 & !1;
    let sh = sw;

    if matches!(args.mode, ScopeMode::Data) {
        let mut argv = ffmpeg_base(g.progress);
        argv.push("-i");
        argv.push(&args.input);
        let (pw, ph) = (probe.width.unwrap_or(1280), probe.height.unwrap_or(720));
        let sx = args.x.unwrap_or(pw / 2);
        let sy = args.y.unwrap_or(ph / 2);
        let en = match &args.at {
            Some(a) => format!(":enable='{}'", enable_expr(a, args.dur, probe.duration)?),
            None => {
                if args.dur.is_some() {
                    return Err(Error::input("--dur needs --at"));
                }
                String::new()
            }
        };
        let vf = format!("datascope=size={pw}x{ph}:x={sx}:y={sy}:mode=color2{en}");
        argv.extend(["-vf", &vf, "-map", "0:v", "-map", "0:a?"]);
        argv.extend([
            "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
        ]);
        if probe.has_audio {
            argv.extend(["-c:a", "copy"]);
        }
        argv.push(&args.output);
        let c2 = engine::write_job("scope", &[&args.input], &args.output, vec![argv], g)?;
        return Ok(c2.with_extra(json!({"mode": "data", "x": sx, "y": sy})));
    }
    if matches!(args.mode, ScopeMode::Mvs) {
        let mut argv = ffmpeg_base(g.progress);
        argv.push("-i");
        argv.push(&args.input);
        let en = match &args.at {
            Some(a) => format!(":enable='{}'", enable_expr(a, args.dur, probe.duration)?),
            None => {
                if args.dur.is_some() {
                    return Err(Error::input("--dur needs --at"));
                }
                String::new()
            }
        };
        let vf = format!("codecview=mv=pf+bf+bb{en}");
        argv.extend(["-vf", &vf, "-map", "0:v", "-map", "0:a?"]);
        argv.extend([
            "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
        ]);
        if probe.has_audio {
            argv.extend(["-c:a", "copy"]);
        }
        argv.push(&args.output);
        let c2 = engine::write_job("scope", &[&args.input], &args.output, vec![argv], g)?;
        return Ok(c2.with_extra(json!({"mode": "mvs"})));
    }
    let filt = match args.mode {
        ScopeMode::Loud => String::new(),
        ScopeMode::Vector => "vectorscope=m=color2".to_string(),
        ScopeMode::Wave => "waveform=mode=column:display=parade:intensity=0.5".to_string(),
        ScopeMode::Hist => "thistogram=display_mode=overlay".to_string(),
        ScopeMode::Qp => "qp".to_string(),
        ScopeMode::Pix => {
            let fw = probe.width.unwrap_or(1280) as f64;
            let fh = probe.height.unwrap_or(720) as f64;
            let fx = args.x.map(|px| px as f64 / fw).unwrap_or(0.5);
            let fy = args.y.map(|px| px as f64 / fh).unwrap_or(0.5);
            format!("pixscope=x={fx:.3}:y={fy:.3}:w=17:h=17:o=0.9")
        }
        ScopeMode::Osc => "oscilloscope=x=0.5:y=0.5:s=0.85:t=0.5:o=1:g=1:st=1".to_string(),
        // drift: signalstats feeds YAVG per frame; drawgraph plots it in the
        // corner tile — flat = locked exposure, slope = ramp/flicker source
        ScopeMode::Drift => format!(
            "signalstats,drawgraph=m1=lavfi.signalstats.YAVG:fg1=0xFF2020:min=0:max=255:bg=0x202020@0.7:slide=scroll:size={sw}x{sh}"
        ),
        ScopeMode::Mvs | ScopeMode::Data => unreachable!(),
    };
    let (x, y) = match args.position.as_str() {
        "top-left" => ("8", "8"),
        "top-right" => ("W-w-8", "8"),
        "bottom-left" => ("8", "H-h-8"),
        _ => ("W-w-8", "H-h-8"),
    };
    let en = match &args.at {
        Some(a) => format!(":enable='{}'", enable_expr(a, args.dur, probe.duration)?),
        None => {
            if args.dur.is_some() {
                return Err(Error::input("--dur needs --at"));
            }
            String::new()
        }
    };

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    // pixscope needs ≥640x480: upsample (nearest — keep pixel edges crisp),
    // run it there, then shrink the viz to the tile
    let fc = if matches!(args.mode, ScopeMode::Loud) {
        // loudness-over-time tile: ebur128 tags each frame's momentary LUFS,
        // adrawgraph renders it — dips mark quiet stretches, a pinned top
        // means the track is driving into the ceiling
        format!(
            "[0:a]ebur128=metadata=1,adrawgraph=m1=lavfi.r128.M:fg1=0xFFFF00:min=-70:max=0:bg=0x202020@0.7:slide=scroll:size={sw}x{sh}[sc];[0:v][sc]overlay={x}:{y}{en}[v]",
            sw = sw, sh = sh, x = x, y = y, en = en
        )
    } else if matches!(args.mode, ScopeMode::Pix) {
        format!(
            "[0:v]split[a][b];[b]scale='max(iw,640)':'max(ih,480)':flags=neighbor,format=rgb24,{filt},scale={sw}:{sh}[sc];[a][sc]overlay={x}:{y}{en}[v]",
            filt = filt, x = x, y = y, en = en
        )
    } else {
        format!(
        "[0:v]split[a][b];[b]scale={sw}:{sh},format=rgb24,{filt}[sc];[a][sc]overlay={x}:{y}{en}[v]",
        filt = filt, x = x, y = y, en = en
        )
    };
    argv.extend(["-filter_complex", &fc, "-map", "[v]", "-map", "0:a?"]);
    argv.extend([
        "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
    ]);
    if probe.has_audio {
        argv.extend(["-c:a", "copy"]);
    }
    argv.push(&args.output);

    let c2 = engine::write_job("scope", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c2.with_extra(json!({
        "mode": format!("{:?}", args.mode),
        "size": args.size,
        "position": args.position,
    })))
}

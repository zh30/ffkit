use serde_json::json;

use crate::cli::{BarEdge, Globals, ProgressArgs};
use crate::contract::{Contract, Status};
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::spawn::Argv;
use crate::time;

pub fn run(args: ProgressArgs, g: &Globals) -> Result<Contract, Error> {
    if args.height == 0 || args.height > 200 {
        return Err(Error::input("--height must be 1..200 px"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "progress")?;

    // drawbox's w is init-only, so the bar is a strip sliding in:
    // tip lands at W*t/duration linearly.
    let vertical = matches!(args.edge, BarEdge::Left | BarEdge::Right);
    let x = match args.edge {
        BarEdge::Right => "main_w-overlay_w".to_string(),
        _ => "0".to_string(),
    };
    let y = match args.edge {
        BarEdge::Bottom => "main_h-overlay_h".to_string(),
        _ => "0".to_string(),
    };
    let enable = match &args.at {
        Some(s) => format!(
            ":enable='{}'",
            time::enable_expr(s, args.dur, probe.duration)?
        ),
        None => {
            if args.dur.is_some() {
                return Err(Error::input("--dur needs --at"));
            }
            String::new()
        }
    };
    // Vertical bars fill bottom-up: the strip slides down from y=-H.
    let slide = if args.reverse {
        if vertical {
            format!("y='main_h*t/{:.3}'", probe.duration)
        } else {
            format!("x='-main_w*t/{:.3}'", probe.duration)
        }
    } else if vertical {
        format!("y='main_h-main_h*t/{:.3}'", probe.duration)
    } else {
        format!("x='-main_w+main_w*t/{:.3}'", probe.duration)
    };
    let (tx, ty) = if vertical {
        (x.as_str(), "0")
    } else {
        ("0", y.as_str())
    };
    // fill_pos = the moving overlay arg: `x=...` for horizontal, `y=...` for vertical;
    // track sits at the fixed edge (tx, ty) behind it.
    let fill = if vertical {
        format!("overlay=x={x}:{slide}:shortest=1{enable}")
    } else {
        format!("overlay={slide}:y={y}:shortest=1{enable}")
    };
    let fc = if args.bg.is_some() {
        // Static full-length track bar, then the fill bar sliding across it.
        format!("[0:v][2:v]overlay=x={tx}:y={ty}:shortest=1[tb];[tb][1:v]{fill}[vout]")
    } else {
        format!("[0:v][1:v]{fill}[vout]")
    };

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.push("-f");
    argv.push("lavfi");
    argv.push("-i");
    let (sw, sh) = if vertical {
        (args.height, probe.height.unwrap_or(180))
    } else {
        (probe.width.unwrap_or(320), args.height)
    };
    let bar_src = format!(
        "color=c={c}:size={w}x{h}:d={d:.3}",
        c = crate::color::lavfi(&args.color),
        w = sw,
        h = sh,
        d = probe.duration,
    );
    argv.push(&bar_src);
    if let Some(bg) = &args.bg {
        argv.push("-f");
        argv.push("lavfi");
        argv.push("-i");
        argv.push(bar_src.replacen(
            &format!("color=c={}", crate::color::lavfi(&args.color)),
            &format!("color=c={}", crate::color::lavfi(bg)),
            1,
        ));
    }
    argv.extend(["-filter_complex", &fc, "-map", "[vout]"]);
    if probe.has_audio {
        argv.extend(["-map", "0:a", "-c:a", "aac"]);
    }
    argv.extend([
        "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
    ]);
    argv.push(&args.output);

    let argvs: Vec<Argv> = vec![argv];
    let c = engine::write_job("progress", &[&args.input], &args.output, argvs, g)?;
    let mut extra = json!({
        "color": args.color,
        "height": args.height,
        "edge": format!("{:?}", args.edge).to_lowercase(),
    });
    if matches!(c.status, Status::Ok) {
        if let Ok(p) = engine::probe_or_err(&args.output, g) {
            extra["probe"] = json!(p);
        }
    }
    Ok(c.with_extra(extra))
}

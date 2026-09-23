use serde_json::json;

use crate::cli::{Globals, TrailArgs, TrailMode};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: TrailArgs, g: &Globals) -> Result<Contract, Error> {
    if !(2..=16).contains(&args.frames) {
        return Err(Error::input("--frames must be 2..=16"));
    }
    if !(0.5..=0.99).contains(&args.decay) {
        return Err(Error::input("--decay must be 0.5..=0.99"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "trail")?;

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    match args.mode {
        TrailMode::Echo => {
            // tmix smears FORWARD; delay the smear back by (frames-1)/fps and
            // overlay it on the live picture — a real trailing echo.
            let fps = probe.fps.unwrap_or(30.0).max(1.0);
            let d = (args.frames - 1) as f64 / fps;
            let w: Vec<String> = (1..=args.frames).map(|i| i.to_string()).collect();
            let enable = match &args.at {
                Some(s) => format!(
                    ":enable='{}'",
                    crate::time::enable_expr(s, args.dur, probe.duration)?
                ),
                None => {
                    if args.dur.is_some() {
                        return Err(Error::input("--dur needs --at"));
                    }
                    String::new()
                }
            };
            let fc = format!(
                "[0:v]split[m][t];[t]tmix=frames={}:weights='{}',setpts=PTS+{d:.4}/TB[t];[m][t]overlay=eof_action=pass{enable}[v]",
                args.frames,
                w.join(" ")
            );
            argv.extend(["-filter_complex", &fc, "-map", "[v]", "-map", "0:a?"]);
            if probe.has_audio {
                argv.extend(["-c:a", "copy"]);
            }
        }
        TrailMode::Diff => {
            let enable = match &args.at {
                Some(s) => format!(
                    ":enable='{}'",
                    crate::time::enable_expr(s, args.dur, probe.duration)?
                ),
                None => {
                    if args.dur.is_some() {
                        return Err(Error::input("--dur needs --at"));
                    }
                    String::new()
                }
            };
            argv.extend(["-vf", &format!("tblend=all_mode=difference{enable}")]);
            if probe.has_audio {
                argv.extend(["-c:a", "copy"]);
            }
        }
        TrailMode::Light => {
            if args.at.is_some() || args.dur.is_some() {
                return Err(Error::input("--at/--dur only apply to --mode echo"));
            }
            argv.extend(["-vf", &format!("lagfun=decay={:.3}", args.decay)]);
            if probe.has_audio {
                argv.extend(["-c:a", "copy"]);
            }
        }
    }
    argv.extend([
        "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
    ]);
    argv.push(&args.output);

    let c = engine::write_job("trail", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "mode": match args.mode {
            TrailMode::Echo => "echo",
            TrailMode::Light => "light",
            TrailMode::Diff => "diff",
        },
        "frames": args.frames,
        "decay": args.decay,
    })))
}

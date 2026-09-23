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
    let w = probe.width.unwrap_or(0) as f64;
    let sw = (w * args.size) as u32 & !1;
    let sh = sw;

    let filt = match args.mode {
        ScopeMode::Vector => "vectorscope=m=color2",
        ScopeMode::Wave => "waveform=mode=column:display=parade:intensity=0.5",
        ScopeMode::Hist => "thistogram=display_mode=overlay",
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
    let fc =
        format!(
        "[0:v]split[a][b];[b]scale={sw}:{sh},format=rgb24,{filt}[sc];[a][sc]overlay={x}:{y}{en}[v]",
        filt = filt, x = x, y = y, en = en
    );
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

use serde_json::json;

use crate::cli::{Globals, TiltArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::time::enable_expr;

pub fn run(args: TiltArgs, g: &Globals) -> Result<Contract, Error> {
    if !(0.1..=0.45).contains(&args.band) {
        return Err(Error::input(
            "--band must be 0.1..=0.45 (sharp band fraction)",
        ));
    }
    if !(1.0..=40.0).contains(&args.blur) {
        return Err(Error::input("--blur must be 1..=40 (gblur sigma)"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "tilt")?;
    let (w, h) = (probe.width.unwrap_or(0), probe.height.unwrap_or(0));
    let strip = ((h as f64 * (1.0 - args.band)) / 2.0) as u32 & !1;
    if strip == 0 {
        return Err(Error::input("--band too wide for this height"));
    }
    let rest_y = h - strip;

    let en = match &args.at {
        Some(a) => format!(":enable='{}'", enable_expr(a, args.dur, probe.duration)?),
        None => {
            if args.dur.is_some() {
                return Err(Error::input("--dur needs --at"));
            }
            String::new()
        }
    };

    // tilt-shift: blur the top and bottom strips, keep the middle band sharp
    let fc = format!(
        "[0:v]split[a][t];[t]crop={w}:{strip}:0:0,gblur=sigma={b:.1}[tb];\
         [a][tb]overlay=0:0{en}[m];[m]split[m2][c];[c]crop={w}:{strip}:0:{rest_y},gblur=sigma={b:.1}[bb];\
         [m2][bb]overlay=0:{rest_y}{en}[v]",
        b = args.blur
    );

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-filter_complex", &fc, "-map", "[v]", "-map", "0:a?"]);
    argv.extend([
        "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
    ]);
    if probe.has_audio {
        argv.extend(["-c:a", "copy"]);
    }
    argv.push(&args.output);

    let c2 = engine::write_job("tilt", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c2.with_extra(json!({ "band": args.band, "blur": args.blur })))
}

use serde_json::json;

use crate::cli::{Globals, StrobeArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: StrobeArgs, g: &Globals) -> Result<Contract, Error> {
    if !(0.5..=30.0).contains(&args.rate) {
        return Err(Error::input("--rate must be 0.5..=30 flashes/sec"));
    }
    if !(0.05..=0.8).contains(&args.duty) {
        return Err(Error::input("--duty must be 0.05..=0.8"));
    }
    let c = crate::color::lavfi(&args.color);
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "strobe")?;
    let (w, h) = (probe.width.unwrap_or(0), probe.height.unwrap_or(0));

    // music-video flash: opaque color frame replaces the picture periodically
    let period = 1.0 / args.rate;
    let flash = period * args.duty;
    let win = match &args.at {
        Some(a) => crate::time::enable_windows(a, args.dur, probe.duration)?,
        None => {
            if args.dur.is_some() {
                return Err(Error::input("--dur needs --at"));
            }
            vec![(0.0, probe.duration)]
        }
    };
    let en = format!(
        "{}*lt(mod(t,{period:.4}),{flash:.4})",
        win.iter()
            .map(|(s, e)| format!("between(t,{s:.3},{e:.3})"))
            .collect::<Vec<_>>()
            .join("+")
    );

    let mut argv = ffmpeg_base(g.progress);
    argv.extend(["-f", "lavfi", "-i"]);
    argv.push(format!("color=c={c}:size={w}x{h}:d={:.3}", probe.duration));
    argv.push("-i");
    argv.push(&args.input);
    let fc = format!("[1:v][0:v]overlay=0:0:enable='{en}'[v]");
    argv.extend(["-filter_complex", &fc, "-map", "[v]", "-map", "1:a?"]);
    argv.extend([
        "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
    ]);
    if probe.has_audio {
        argv.extend(["-c:a", "copy"]);
    }
    argv.push(&args.output);

    let c2 = engine::write_job("strobe", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c2.with_extra(json!({
        "rate": args.rate,
        "duty": args.duty,
        "color": args.color,
    })))
}

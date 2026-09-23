use serde_json::json;

use crate::cli::{Globals, SpinArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::time::enable_expr;

pub fn run(args: SpinArgs, g: &Globals) -> Result<Contract, Error> {
    if !(0.5..=45.0).contains(&args.deg) {
        return Err(Error::input("--deg must be 0.5..=45 swing amplitude"));
    }
    if !(0.05..=10.0).contains(&args.rate) {
        return Err(Error::input("--rate must be 0.05..=10 swings/sec"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "spin")?;

    let en = match &args.at {
        Some(a) => format!(":enable='{}'", enable_expr(a, args.dur, probe.duration)?),
        None => {
            if args.dur.is_some() {
                return Err(Error::input("--dur needs --at"));
            }
            String::new()
        }
    };

    // pendulum sway: rotate by a slow sine, keep canvas, black corners
    let vf = format!(
        "rotate=a='{d:.3}*PI/180*sin(2*PI*t*{r:.3})':fillcolor=black{en}",
        d = args.deg,
        r = args.rate
    );

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-vf", &vf]);
    argv.extend([
        "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
    ]);
    if probe.has_audio {
        argv.extend(["-c:a", "copy"]);
    }
    argv.push(&args.output);

    let c2 = engine::write_job("spin", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c2.with_extra(json!({ "deg": args.deg, "rate": args.rate })))
}

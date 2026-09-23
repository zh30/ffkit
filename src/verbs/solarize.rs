use serde_json::json;

use crate::cli::{Globals, SolarizeArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::time::enable_expr;

pub fn run(args: SolarizeArgs, g: &Globals) -> Result<Contract, Error> {
    if args.threshold > 255 {
        return Err(Error::input("--threshold must be 0..=255"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "solarize")?;
    let en = match &args.at {
        Some(a) => format!(":enable='{}'", enable_expr(a, args.dur, probe.duration)?),
        None => {
            if args.dur.is_some() {
                return Err(Error::input("--dur needs --at"));
            }
            String::new()
        }
    };

    let t = args.threshold;
    let e = format!("if(gt(val,{t}),255-val,val)");
    let vf = format!("lutrgb=r='{e}':g='{e}':b='{e}'{en}");

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

    let c2 = engine::write_job("solarize", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c2.with_extra(json!({ "threshold": args.threshold })))
}

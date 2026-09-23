use serde_json::json;

use crate::cli::{Globals, LensArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::time::enable_expr;

pub fn run(args: LensArgs, g: &Globals) -> Result<Contract, Error> {
    if !(-1.0..=1.0).contains(&args.k1) || !(-1.0..=1.0).contains(&args.k2) {
        return Err(Error::input("--k1/--k2 must be -1.0..=1.0"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "lens")?;
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
    let vf = format!(
        "lenscorrection=k1={}:k2={}:i=bilinear{en}",
        args.k1, args.k2
    );
    argv.extend(["-vf", &vf]);
    argv.extend([
        "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
    ]);
    if probe.has_audio {
        argv.extend(["-c:a", "copy"]);
    }
    argv.push(&args.output);

    let c2 = engine::write_job("lens", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c2.with_extra(json!({ "k1": args.k1, "k2": args.k2 })))
}

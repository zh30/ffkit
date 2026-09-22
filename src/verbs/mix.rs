use serde_json::json;
use std::path::Path;

use crate::cli::{Globals, MixArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::paths;

pub fn run(args: MixArgs, g: &Globals) -> Result<Contract, Error> {
    let pa = engine::probe_or_err(&args.a, g)?;
    let pb = engine::probe_or_err(&args.b, g)?;
    for (p, path) in [(&pa, &args.a), (&pb, &args.b)] {
        if !p.has_audio {
            return Err(Error::input(format!(
                "`{}` has no audio stream — mix merges audio",
                paths::display(path)
            )));
        }
    }
    if !(0.0..=4.0).contains(&args.vol_a) || !(0.0..=4.0).contains(&args.vol_b) {
        return Err(Error::input("--vol must be 0..=4 (linear scale)"));
    }
    let dur = if args.longest { "longest" } else { "first" };

    // --at/--dur: gate the B track into the window (outside it, A alone).
    let gate = match &args.at {
        Some(raw) => {
            let at = crate::time::parse_time(raw)?;
            if !(0.0..pa.duration).contains(&at) {
                return Err(Error::input("--at is outside the A input"));
            }
            let end = args.dur.map(|d| at + d).unwrap_or(pa.duration);
            format!(",volume='between(t,{at:.3},{end:.3})':eval=frame")
        }
        None => {
            if args.dur.is_some() {
                return Err(Error::input("--dur needs --at"));
            }
            String::new()
        }
    };

    let fc = format!(
        "[0:a]aresample=48000,volume={:.4}[a0];\
         [1:a]aresample=48000,volume={:.4}{gate}[a1];\
         [a0][a1]amix=inputs=2:duration={dur}:normalize=0[aout]",
        args.vol_a, args.vol_b
    );

    let mut argv = ffmpeg_base(g.progress);
    argv.extend(["-i".to_string(), args.a.display().to_string()]);
    argv.extend(["-i".to_string(), args.b.display().to_string()]);
    argv.extend(["-filter_complex".to_string(), fc]);
    if pa.has_video {
        argv.extend(["-map".to_string(), "0:v".to_string()]);
        argv.extend(["-c:v".to_string(), "copy".to_string()]);
    }
    argv.extend(["-map".to_string(), "[aout]".to_string()]);
    if pa.has_video {
        argv.extend([
            "-c:a".to_string(),
            "aac".to_string(),
            "-b:a".to_string(),
            "192k".to_string(),
        ]);
    }
    argv.push(args.output.display().to_string());

    let inputs: Vec<&Path> = vec![&args.a, &args.b];
    let mut c = engine::write_job("mix", &inputs, &args.output, vec![argv], g)?;
    c = c.with_extra(json!({
        "duration_mode": dur,
        "vol_a": args.vol_a,
        "vol_b": args.vol_b,
        "kept_video": pa.has_video,
    }));
    Ok(c)
}

use std::io::Write;

use serde_json::json;

use crate::cli::{Globals, LoopArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::paths;

pub fn run(args: LoopArgs, g: &Globals) -> Result<Contract, Error> {
    let times = match args.until {
        Some(secs) => {
            let probe = engine::probe_or_err(&args.input, g)?;
            if probe.duration <= 0.0 {
                return Err(Error::input("loop --until: input has no duration"));
            }
            let n = (secs / probe.duration).ceil() as u32;
            if !(2..=500).contains(&n) {
                return Err(Error::input(format!(
                    "--until {secs}s needs {n} loops (allowed 2..=500)"
                )));
            }
            n
        }
        None if !(2..=12).contains(&args.times) => {
            return Err(Error::input("--times must be 2..=12"));
        }
        None => args.times,
    };
    paths::ensure_input(&args.input)?;
    let mut list = tempfile::NamedTempFile::new().map_err(|e| Error::output(e.to_string()))?;
    let abs = paths::abs(&args.input);
    let escaped = abs.to_string_lossy().replace('\'', "'\\''");
    for _ in 0..times {
        writeln!(list, "file '{escaped}'").map_err(|e| Error::output(e.to_string()))?;
    }
    list.flush().ok();
    let list_path = list.path().to_path_buf();

    let mut argv = ffmpeg_base(g.progress);
    argv.extend(["-f", "concat", "-safe", "0", "-i"]);
    argv.push(&list_path);
    argv.extend(["-c", "copy"]);
    if let Some(secs) = args.until {
        argv.extend(["-t".to_string(), format!("{secs:.3}")]);
    }
    argv.push(&args.output);

    let result = engine::write_job("loop", &[&args.input], &args.output, vec![argv], g);
    drop(list);
    let c = result?;
    Ok(c.with_extra(json!({ "times": times,
        "until": args.until })))
}

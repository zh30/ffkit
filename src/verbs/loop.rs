use std::io::Write;

use serde_json::json;

use crate::cli::{Globals, LoopArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::paths;

pub fn run(args: LoopArgs, g: &Globals) -> Result<Contract, Error> {
    if !(2..=12).contains(&args.times) {
        return Err(Error::input("--times must be 2..=12"));
    }
    paths::ensure_input(&args.input)?;
    let mut list = tempfile::NamedTempFile::new().map_err(|e| Error::output(e.to_string()))?;
    let abs = paths::abs(&args.input);
    let escaped = abs.to_string_lossy().replace('\'', "'\\''");
    for _ in 0..args.times {
        writeln!(list, "file '{escaped}'").map_err(|e| Error::output(e.to_string()))?;
    }
    list.flush().ok();
    let list_path = list.path().to_path_buf();

    let mut argv = ffmpeg_base(g.progress);
    argv.extend(["-f", "concat", "-safe", "0", "-i"]);
    argv.push(&list_path);
    argv.extend(["-c", "copy"]);
    argv.push(&args.output);

    let result = engine::write_job("loop", &[&args.input], &args.output, vec![argv], g);
    drop(list);
    let c = result?;
    Ok(c.with_extra(json!({ "times": args.times })))
}

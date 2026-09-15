use std::path::{Path, PathBuf};

use serde_json::json;

use crate::cli::Globals;
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(because: String, args: Vec<String>, g: &Globals) -> Result<Contract, Error> {
    let reason = because.trim();
    if reason.is_empty() {
        return Err(Error::input(
            "--because must name the verb or graph field that cannot express this job",
        ));
    }
    if args.is_empty() {
        return Err(Error::input(
            "ffkit ffmpeg --because REASON -- <ffmpeg args>; last argument is the output file",
        ));
    }
    let output = PathBuf::from(args.last().unwrap());
    if args.last().unwrap().starts_with('-') {
        return Err(Error::input(
            "last argument must be an output path, not a flag",
        ));
    }

    let inputs = collect_inputs(&args);
    if inputs.is_empty() {
        return Err(Error::input("raw ffmpeg needs at least one -i input"));
    }

    let mut argv = ffmpeg_base(g.progress);
    for a in &args {
        argv.push(a);
    }

    let refs: Vec<&Path> = inputs.iter().map(|p| p.as_path()).collect();
    let mut contract = engine::write_job("ffmpeg", &refs, &output, vec![argv], g)?;
    contract = contract.with_extra(json!({
        "escape": true,
        "because": reason,
    }));
    Ok(contract)
}

fn collect_inputs(args: &[String]) -> Vec<PathBuf> {
    let mut inputs = Vec::new();
    let mut i = 0;
    while i < args.len() {
        if args[i] == "-i" {
            if let Some(p) = args.get(i + 1) {
                if !p.starts_with('-') {
                    inputs.push(PathBuf::from(p));
                    i += 2;
                    continue;
                }
            }
        }
        i += 1;
    }
    inputs
}

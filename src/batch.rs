use std::path::{Path, PathBuf};
use std::process::Command;

use rayon::prelude::*;

use crate::cli::{BatchArgs, Globals};
use crate::contract::{Contract, Status};
use crate::error::Error;
use crate::paths;

pub fn run(args: BatchArgs, g: &Globals) -> Result<Contract, Error> {
    if args.rest.is_empty() {
        return Err(Error::input(
            "batch needs a verb after the flags, e.g. ffkit batch in -o out -- transcode --preset h264",
        ));
    }
    if !args.dir.is_dir() {
        return Err(Error::input(format!(
            "not a directory: {}",
            args.dir.display()
        )));
    }
    let verb = args.rest[0].clone();
    if matches!(verb.as_str(), "batch" | "doctor" | "install-skill") {
        return Err(Error::input(format!("cannot batch {verb}")));
    }
    let verb_args: Vec<String> = args.rest[1..].to_vec();
    if verb_args
        .iter()
        .any(|a| a == "-o" || a == "--output" || a == "--output-dir")
    {
        return Err(Error::input(
            "do not pass -o in the batched verb; batch injects it from --output-dir",
        ));
    }

    std::fs::create_dir_all(&args.output_dir)?;
    let files = list_media(&args.dir, args.recursive)?;
    if files.is_empty() {
        return Err(Error::input(format!(
            "no media files in {}",
            args.dir.display()
        )));
    }

    let jobs = if args.jobs == 0 {
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
    } else {
        args.jobs
    };
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(jobs)
        .build()
        .map_err(|e| Error::output(e.to_string()))?;

    let exe = std::env::current_exe().map_err(|e| Error::output(e.to_string()))?;
    let overwrite = g.overwrite;
    let timeout = g.timeout.as_secs().to_string();
    let dry = g.dry_run;

    let results: Vec<Result<serde_json::Value, String>> = pool.install(|| {
        files
            .par_iter()
            .map(|input| {
                let name = input
                    .file_name()
                    .map(|n| n.to_os_string())
                    .unwrap_or_default();
                let dest = output_name(&args.output_dir, Path::new(&name), &verb);
                let mut cmd = Command::new(&exe);
                cmd.arg(&verb);
                cmd.arg(input);
                cmd.args(&verb_args);
                cmd.arg("-o").arg(&dest);
                cmd.arg("--json-brief");
                cmd.arg("--timeout").arg(&timeout);
                if overwrite {
                    cmd.arg("--overwrite");
                }
                if dry {
                    cmd.arg("--dry-run");
                }
                match cmd.output() {
                    Ok(out) => {
                        let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
                        let parsed = serde_json::from_str::<serde_json::Value>(&stdout)
                            .unwrap_or_else(|_| {
                                serde_json::json!({
                                    "status": if out.status.success() { "ok" } else { "failed" },
                                    "output": dest.display().to_string(),
                                    "raw": stdout,
                                })
                            });
                        Ok(parsed)
                    }
                    Err(e) => Err(e.to_string()),
                }
            })
            .collect()
    });

    let mut extras = Vec::new();
    let mut failed = 0;
    for r in results {
        match r {
            Ok(v) => {
                if v.get("status").and_then(|s| s.as_str()) == Some("failed") {
                    failed += 1;
                }
                extras.push(v);
            }
            Err(e) => {
                failed += 1;
                extras.push(serde_json::json!({"status": "failed", "error": e}));
            }
        }
    }

    let extra = serde_json::json!({
        "jobs": jobs,
        "count": files.len(),
        "failed": failed,
        "results": extras,
    });
    if failed > 0 {
        let err = Error::ffmpeg(format!("{failed}/{} batched jobs failed", files.len()));
        return Ok(Contract::failed("batch", &err).with_extra(extra));
    }
    let mut c =
        Contract::ok("batch", Some(paths::display(&args.output_dir)), None).with_extra(extra);
    c.summary = Some(format!("{} files", files.len()));
    if dry {
        c.status = Status::DryRun;
        c.verified = None;
    }
    Ok(c)
}

fn list_media(dir: &Path, recursive: bool) -> Result<Vec<PathBuf>, Error> {
    let mut out = Vec::new();
    walk(dir, recursive, &mut out)?;
    out.sort();
    Ok(out)
}

fn walk(dir: &Path, recursive: bool, out: &mut Vec<PathBuf>) -> Result<(), Error> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            if recursive {
                walk(&path, true, out)?;
            }
        } else if paths::is_media(&path) {
            out.push(path);
        }
    }
    Ok(())
}

fn output_name(dir: &Path, file_name: &Path, verb: &str) -> PathBuf {
    let stem = file_name
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("out");
    let ext = match verb {
        "look" => "png",
        "transcode" => file_name
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("mp4"),
        _ => file_name
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("mp4"),
    };
    dir.join(format!("{stem}.{ext}"))
}

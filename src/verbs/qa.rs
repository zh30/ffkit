use serde_json::json;

use crate::cli::{Globals, QaArgs};
use crate::contract::Contract;
use crate::engine;
use crate::error::Error;
use crate::paths;
use crate::spawn;

fn last_metric(stderr: &str, key: &str) -> Option<f64> {
    stderr.lines().rev().find_map(|l| {
        l.split(&format!("{key}:"))
            .nth(1)?
            .split_whitespace()
            .next()?
            .parse()
            .ok()
    })
}

pub fn run(args: QaArgs, g: &Globals) -> Result<Contract, Error> {
    let a = engine::probe_or_err(&args.a, g)?;
    let b = engine::probe_or_err(&args.b, g)?;
    if !a.has_video || !b.has_video {
        return Err(Error::input(
            "qa compares video — both inputs need a video stream",
        ));
    }
    paths::ensure_input(&args.a)?;
    paths::ensure_input(&args.b)?;

    let psnr = matches!(args.metric.as_str(), "psnr" | "both");
    let ssim = matches!(args.metric.as_str(), "ssim" | "both");
    let msad = args.metric == "msad";
    let vif = args.metric == "vif";
    if !psnr && !ssim && !msad && !vif {
        return Err(Error::input(
            "--metric must be psnr, ssim, msad, vif, or both",
        ));
    }

    let mut extra = json!({});
    let mut commands = Vec::new();
    for metric in ["psnr", "ssim", "msad", "vif"] {
        if (metric == "psnr" && !psnr)
            || (metric == "ssim" && !ssim)
            || (metric == "msad" && !msad)
            || (metric == "vif" && !vif)
        {
            continue;
        }
        // scale2ref so a resolution-mismatched test file still compares.
        let fc = format!("[1:v][0:v]scale2ref[b][r];[r][b]{metric}");
        // metric lines log at info level — no -loglevel error here.
        let mut argv = crate::spawn::Argv::ffmpeg();
        argv.push("-y");
        argv.extend(["-i".to_string(), args.a.display().to_string()]);
        argv.extend(["-i".to_string(), args.b.display().to_string()]);
        argv.extend(["-filter_complex".to_string(), fc]);
        argv.extend(["-f".to_string(), "null".to_string(), "-".to_string()]);
        commands.push(argv.display());
        if g.dry_run {
            continue;
        }
        let spawned = spawn::run(&argv, g.timeout, false)?;
        let stderr = String::from_utf8_lossy(&spawned.stderr).into_owned();
        if !spawned.status_ok {
            return Ok(
                Contract::failed("qa", &Error::ffmpeg(stderr)).with_commands(commands.clone())
            );
        }
        // vif's last line is `VIF scale=3 average:X` — its finest scale
        // doubles as the headline score; psnr/msad end on `average:` too
        let key = match metric {
            "ssim" => "All",
            _ => "average",
        };
        let val = last_metric(&stderr, key)
            .ok_or_else(|| Error::verification(format!("no {metric} metric in ffmpeg output")))?;
        extra[metric] = json!(val);
    }

    if g.dry_run {
        return Ok(Contract::dry_run("qa", None, Some(a)).with_commands(commands));
    }
    let mut c = Contract::ok("qa", None, Some(a)).with_commands(commands);
    c = c.with_extra(extra);
    Ok(c)
}

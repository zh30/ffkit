use serde_json::json;
use std::process::Command;

use crate::cli::{Globals, PickArgs};
use crate::contract::Contract;
use crate::engine;
use crate::error::Error;

pub fn run(args: PickArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "pick")?;
    let t = args.at.unwrap_or(probe.duration / 2.0);

    // sample a 3x2 grid of zone means at timestamp t
    let vf = "scale=3:2";
    let out = Command::new("ffmpeg")
        .arg("-v")
        .arg("error")
        .arg("-ss")
        .arg(format!("{t:.3}"))
        .arg("-i")
        .arg(&args.input)
        .arg("-vf")
        .arg(vf)
        .args(["-frames:v", "1", "-f", "rawvideo", "-pix_fmt", "rgb24", "-"])
        .output()
        .map_err(|e| Error::input(format!("ffmpeg: {e}")))?;
    if !out.status.success() {
        return Err(Error::input(format!(
            "ffmpeg: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    let d = out.stdout;
    if d.len() < 18 {
        return Err(Error::input("pick: no frame decoded"));
    }
    let colors: Vec<String> = d
        .chunks(3)
        .map(|p| format!("#{:02x}{:02x}{:02x}", p[0], p[1], p[2]))
        .collect();
    let mean = format!(
        "#{:02x}{:02x}{:02x}",
        d.iter().step_by(3).map(|p| *p as u32).sum::<u32>() / 6,
        d.iter().skip(1).step_by(3).map(|p| *p as u32).sum::<u32>() / 6,
        d.iter().skip(2).step_by(3).map(|p| *p as u32).sum::<u32>() / 6,
    );

    let c2 = Contract::ok("pick", None, Some(probe)).with_extra(json!({
        "at": t,
        "mean": mean,
        "zones": colors,
    }));
    let mut c3 = c2;
    c3.commands = vec![vec!["ffmpeg".into(), vf.into()]];
    Ok(c3)
}

use serde_json::json;

use crate::cli::{Globals, TempoArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: TempoArgs, g: &Globals) -> Result<Contract, Error> {
    if !(0.5..=8.0).contains(&args.factor) {
        return Err(Error::input("--factor must be 0.5..=8"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_audio {
        return Err(Error::input("tempo: input has no audio stream"));
    }
    if probe.has_video {
        return Err(Error::input(
            "tempo retimes audio only — use `speed` to retime video",
        ));
    }

    if args.dur.is_some() && args.at.is_none() {
        return Err(Error::input("--dur requires --at"));
    }
    // atempo accepts at most 2x per instance on old ffmpeg — chain segments.
    let mut af = String::new();
    let mut f = args.factor;
    while f > 2.0 {
        af.push_str("atempo=2.0,");
        f /= 2.0;
    }
    while f < 0.5 {
        af.push_str("atempo=0.5,");
        f /= 0.5;
    }
    af.push_str(&format!("atempo={f:.4}"));

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    match &args.at {
        Some(raw) => {
            let windows = crate::time::window_list(raw, args.dur, probe.duration)?;
            // alternating normal/retempoed segments around each window
            let mut bounds = vec![0.0];
            for (s, e) in &windows {
                bounds.push(*s);
                bounds.push(*e);
            }
            bounds.push(probe.duration);
            let mut seg: Vec<String> = Vec::new();
            let mut ins = String::new();
            let mut nseg = 0usize;
            for i in 0..bounds.len() - 1 {
                let (s, e) = (bounds[i], bounds[i + 1]);
                if e - s < 0.01 {
                    continue;
                }
                let mut ch = "asetpts=PTS-STARTPTS".to_string();
                if i % 2 == 1 {
                    ch.push_str(&format!(",{af}"));
                }
                seg.push(format!("[0:a]atrim=start={s:.3}:end={e:.3},{ch}[a{i}]"));
                ins.push_str(&format!("[a{i}]"));
                nseg += 1;
            }
            seg.push(format!("{ins}concat=n={nseg}:v=0:a=1[aout]"));
            let fc = seg.join(";");
            argv.extend(["-filter_complex", &fc, "-map", "[aout]"]);
        }
        None => {
            argv.extend(["-af", &af]);
        }
    }
    if probe.has_video {
        argv.extend(["-c:v", "copy"]);
    }
    argv.extend(["-c:a", "aac"]);
    argv.push(&args.output);

    let c = engine::write_job("tempo", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "factor": args.factor,
        "duration": probe.duration / args.factor,
    })))
}

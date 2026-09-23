use serde_json::json;

use crate::cli::{Globals, ZoomArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::paths;

pub fn run(args: ZoomArgs, g: &Globals) -> Result<Contract, Error> {
    if !(1.05..=3.0).contains(&args.factor) {
        return Err(Error::input(
            "--factor must be 1.05..=3 (1.25 = mild punch-in)",
        ));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "zoom")?;
    let (cx, cy) = match &args.center {
        Some(s) => s
            .split_once(',')
            .and_then(|(a, b)| Some((a.trim().parse::<f64>().ok()?, b.trim().parse::<f64>().ok()?)))
            .filter(|(a, b)| (0.0..=100.0).contains(a) && (0.0..=100.0).contains(b))
            .map(|(a, b)| (a / 100.0, b / 100.0))
            .ok_or_else(|| Error::input("--center must look like 50,50 (percent)"))?,
        None => (0.5, 0.5),
    };
    let w = paths::even(probe.width.unwrap_or(1280)).max(2);
    let h = paths::even(probe.height.unwrap_or(720)).max(2);
    let sw = paths::even(((w as f64) * args.factor).round() as u32).max(w + 2);
    let sh = paths::even(((h as f64) * args.factor).round() as u32).max(h + 2);
    let vf = format!("scale={sw}:{sh},crop={w}:{h}:(iw-ow)*{cx:.4}:(ih-oh)*{cy:.4},setsar=1");

    let factor0 = args.factor;
    let fps0 = probe.fps.unwrap_or(30.0).max(1.0);
    let zoompan_for = move |dur_secs: f64| -> String {
        // Push 1.0 -> factor over dur_secs; pzoom accumulates per frame.
        let fps = fps0;
        let frames = (dur_secs * fps).max(1.0);
        let step = (factor0 - 1.0) / frames;
        if args.out {
            format!(
                "zoompan=z='max({f:.4}-on*{step:.8},1.0)':d=1:x='iw*{cx:.4}-{cx:.4}*iw/zoom':y='ih*{cy:.4}-{cy:.4}*ih/zoom':s={w}x{h},setsar=1",
                f = factor0,
            )
        } else {
            format!(
                "zoompan=z='min(pzoom+{step:.8},{f:.4})':d=1:x='iw*{cx:.4}-{cx:.4}*iw/zoom':y='ih*{cy:.4}-{cy:.4}*ih/zoom':s={w}x{h},setsar=1",
                f = factor0,
            )
        }
    };
    if args.dur.is_some() && args.at.is_none() {
        return Err(Error::input("--dur needs --at"));
    }
    if args.at.is_some() {
        return windowed(args, &vf, &zoompan_for, &probe, g);
    }
    if args.motion.is_some() {
        let vf = zoompan_for(probe.duration);
        let fc = format!("[0:v]{vf}[vout]");
        let mut argv = ffmpeg_base(g.progress);
        argv.push("-i");
        argv.push(&args.input);
        argv.extend(["-filter_complex", &fc, "-map", "[vout]"]);
        if probe.has_audio {
            argv.extend(["-map", "0:a?", "-c:a", "copy"]);
        }
        argv.extend([
            "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
        ]);
        argv.push(&args.output);
        let c = engine::write_job("zoom", &[&args.input], &args.output, vec![argv], g)?;
        return Ok(c.with_extra(json!({
            "factor": factor0,
            "motion": "kenburns",
        })));
    }

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend([
        "-vf", &vf, "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
    ]);
    if probe.has_audio {
        argv.extend(["-c:a", "copy"]);
    }
    argv.push(&args.output);

    let c = engine::write_job("zoom", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "factor": args.factor,
        "frame": format!("{w}x{h}"),
    })))
}

/// Windowed punch: the zoom only lives inside [at, at+dur] (or [at, end])
/// via a 3-segment trim/concat — audio passes through untouched.
fn windowed(
    args: ZoomArgs,
    vf: &str,
    zoompan_for: &dyn Fn(f64) -> String,
    probe: &crate::probe::Probe,
    g: &Globals,
) -> Result<Contract, Error> {
    let windows = crate::time::window_list(args.at.as_deref().unwrap(), args.dur, probe.duration)?;
    let has_a = probe.has_audio;
    // alternating normal/zoomed segments around each window
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
        if i % 2 == 1 {
            let mid_vf = if args.motion.is_some() {
                zoompan_for(e - s)
            } else {
                vf.to_string()
            };
            seg.push(format!(
                "[0:v]trim=start={s:.3}:end={e:.3},setpts=PTS-STARTPTS,{mid_vf}[v{i}]"
            ));
        } else {
            seg.push(format!(
                "[0:v]trim=start={s:.3}:end={e:.3},setpts=PTS-STARTPTS[v{i}]"
            ));
        }
        ins.push_str(&format!("[v{i}]"));
        if has_a {
            seg.push(format!(
                "[0:a]atrim=start={s:.3}:end={e:.3},asetpts=PTS-STARTPTS[a{i}]"
            ));
            ins.push_str(&format!("[a{i}]"));
        }
        nseg += 1;
    }
    let (nv, na) = (1, if has_a { 1 } else { 0 });
    let outs = if has_a { "[vout][aout]" } else { "[vout]" };
    seg.push(format!("{ins}concat=n={nseg}:v={nv}:a={na}{outs}"));
    let fc = seg.join(";");

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-filter_complex", &fc, "-map", "[vout]"]);
    if probe.has_audio {
        argv.extend(["-map", "[aout]", "-c:a", "aac"]);
    }
    argv.extend([
        "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
    ]);
    argv.push(&args.output);

    let mut c = engine::write_job("zoom", &[&args.input], &args.output, vec![argv], g)?;
    c = c.with_extra(json!({
        "factor": args.factor,
        "windows": windows.iter().map(|(s, e)| json!({"at": s, "dur": e - s})).collect::<Vec<_>>(),
        "motion": args.motion.map(|_| "kenburns"),
    }));
    Ok(c)
}

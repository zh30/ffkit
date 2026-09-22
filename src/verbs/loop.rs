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
    if args.from.is_some() || args.to.is_some() {
        return section(args, g, times);
    }
    if let Some(fade) = args.fade {
        return seamless(&args, g, times, fade);
    }
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

/// xfade-chain loop: every copy crossfades `fade` seconds into the next so
/// the loop point is invisible (re-encodes — no stream copy). Each joint
/// shaves `fade` off the total: offsets land at k*(len - fade).
fn seamless(args: &LoopArgs, g: &Globals, times: u32, fade: f64) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "loop --fade")?;
    if fade <= 0.0 || fade >= probe.duration {
        return Err(Error::input(
            "--fade must be > 0 and shorter than the input",
        ));
    }
    let n = times as usize;
    let len = probe.duration;
    // split=N is one node with N outputs.
    let mut seg = format!(
        "[0:v]split={n}{};",
        (0..n).map(|i| format!("[sv{i}]")).collect::<String>()
    );
    let mut cur = "sv0".to_string();
    for i in 1..n {
        let next = format!("x{i}");
        seg.push_str(&format!(
            "[{cur}][sv{i}]xfade=transition=fade:duration={fade:.3}:offset={off:.3}[{next}];",
            off = i as f64 * (len - fade)
        ));
        cur = next;
    }
    let mut fc = format!("{seg}[{cur}]format=yuv420p[vout]");
    if probe.has_audio {
        let mut audio = format!(
            "[0:a]asplit={n}{};",
            (0..n).map(|i| format!("[sa{i}]")).collect::<String>()
        );
        let mut acur = "sa0".to_string();
        for i in 1..n {
            let next = format!("xa{i}");
            audio.push_str(&format!("[{acur}][sa{i}]acrossfade=d={fade:.3}[{next}];"));
            acur = next;
        }
        fc.push_str(&format!(
            ";{audio}[{acur}]aformat=channel_layouts=stereo[aout]"
        ));
    }
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-filter_complex", &fc, "-map", "[vout]"]);
    if probe.has_audio {
        argv.extend(["-map", "[aout]", "-c:a", "aac"]);
    }
    argv.extend(["-c:v", "libx264", "-preset", "fast", "-crf", "18"]);
    if let Some(secs) = args.until {
        argv.extend(["-t".to_string(), format!("{secs:.3}")]);
    }
    argv.push(&args.output);
    let c = engine::write_job("loop", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({ "times": times, "fade": fade, "until": args.until })))
}

/// Loop only [from, to] and keep the rest once: three concat arms —
/// head, the looped section (v loop / a aloop), tail.
fn section(args: LoopArgs, g: &Globals, times: u32) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    let from = match &args.from {
        Some(raw) => crate::time::parse_time(raw)?,
        None => 0.0,
    };
    let to = match &args.to {
        Some(raw) => crate::time::parse_time(raw)?,
        None => probe.duration,
    };
    if from >= to || to > probe.duration + 0.001 {
        return Err(Error::input(format!(
            "section {from}..{to} is outside 0..{:.3}",
            probe.duration
        )));
    }
    let n = times.saturating_sub(1).max(1);
    // loop=size=0 is a silent no-op; buffer must cover the whole section.
    let seg_frames = (((to - from) * probe.fps.unwrap_or(30.0)).ceil() as u32) + 2;
    let seg_samples = (((to - from) * probe.sample_rate.unwrap_or(44100) as f64).ceil() as u32) + 2;
    let mut fc = format!(
        "[0:v]trim=0:{from:.3},setpts=PTS-STARTPTS[v0];         [0:v]trim=start={from:.3}:end={to:.3},setpts=PTS-STARTPTS,loop=loop={n}:size={seg_frames}[v1];         [0:v]trim=start={to:.3},setpts=PTS-STARTPTS[v2];         [v0][v1][v2]concat=n=3:v=1:a=0[vout]"
    );
    if probe.has_audio {
        fc.push_str(&format!(
            ";[0:a]atrim=0:{from:.3},asetpts=PTS-STARTPTS[a0];             [0:a]atrim=start={from:.3}:end={to:.3},asetpts=PTS-STARTPTS,aloop=loop={n}:size={seg_samples}[a1];             [0:a]atrim=start={to:.3},asetpts=PTS-STARTPTS[a2];             [a0][a1][a2]concat=n=3:v=0:a=1[aout]"
        ));
    }
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-filter_complex", &fc, "-map", "[vout]"]);
    if probe.has_audio {
        argv.extend(["-map", "[aout]", "-c:a", "aac"]);
    }
    argv.extend([
        "-c:v", "libx264", "-preset", "medium", "-crf", "20", "-pix_fmt", "yuv420p",
    ]);
    argv.push(&args.output);
    let c = engine::write_job("loop", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({ "times": times, "from": from, "to": to })))
}

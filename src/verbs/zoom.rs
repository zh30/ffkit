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
    let w = paths::even(probe.width.unwrap_or(1280)).max(2);
    let h = paths::even(probe.height.unwrap_or(720)).max(2);
    let sw = paths::even(((w as f64) * args.factor).round() as u32).max(w + 2);
    let sh = paths::even(((h as f64) * args.factor).round() as u32).max(h + 2);
    let vf = format!("scale={sw}:{sh},crop={w}:{h},setsar=1");

    if args.dur.is_some() && args.at.is_none() {
        return Err(Error::input("--dur needs --at"));
    }
    if args.at.is_some() {
        return windowed(args, &vf, &probe, g);
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
    probe: &crate::probe::Probe,
    g: &Globals,
) -> Result<Contract, Error> {
    let at = crate::time::parse_time(args.at.as_deref().unwrap())?;
    if !(0.0..probe.duration - 0.1).contains(&at) {
        return Err(Error::input("--at must land inside the input"));
    }
    let end = match args.dur {
        Some(d) if d <= 0.0 => return Err(Error::input("--dur must be positive")),
        Some(d) => (at + d).min(probe.duration),
        None => probe.duration,
    };
    let mut seg: Vec<String> = vec![
        format!("[0:v]trim=0:{at:.3},setpts=PTS-STARTPTS[v0]"),
        format!("[0:v]trim={at:.3}:{end:.3},setpts=PTS-STARTPTS,{vf}[v1]"),
        format!("[0:v]trim=start={end:.3},setpts=PTS-STARTPTS[v2]"),
        "[v0][v1][v2]concat=n=3:v=1:a=0[vout]".to_string(),
    ];
    if probe.has_audio {
        seg.insert(3, format!("[0:a]atrim=0:{at:.3},asetpts=PTS-STARTPTS[a0]"));
        seg.insert(
            4,
            format!("[0:a]atrim={at:.3}:{end:.3},asetpts=PTS-STARTPTS[a1]"),
        );
        seg.insert(
            5,
            format!("[0:a]atrim=start={end:.3},asetpts=PTS-STARTPTS[a2]"),
        );
        seg.pop();
        seg.push("[v0][a0][v1][a1][v2][a2]concat=n=3:v=1:a=1[vout][aout]".to_string());
    }
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
        "at": at,
        "dur": end - at,
    }));
    Ok(c)
}

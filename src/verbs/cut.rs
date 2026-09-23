use crate::cli::{CutArgs, Globals};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::probe::Probe;
use crate::time::{fmt_time, parse_time};

pub fn run(args: CutArgs, g: &Globals) -> Result<Contract, Error> {
    let start = args
        .start
        .as_deref()
        .map(parse_time)
        .transpose()?
        .unwrap_or(0.0);
    let end = args.end.as_deref().map(parse_time).transpose()?;
    let duration = args.duration.as_deref().map(parse_time).transpose()?;

    if args.start.is_none()
        && args.end.is_none()
        && args.duration.is_none()
        && args.ranges.is_none()
        && args.drop.is_none()
    {
        return Err(Error::input(
            "cut needs --start/--end/--duration/--ranges/--drop",
        ));
    }

    let dur = match (end, duration) {
        (Some(e), Some(d)) => {
            if (e - start - d).abs() > 0.02 {
                return Err(Error::input(
                    "--end and --duration disagree; pass only one of them",
                ));
            }
            Some(d)
        }
        (Some(e), None) => {
            if e <= start {
                return Err(Error::input("--end must be after --start"));
            }
            Some(e - start)
        }
        (None, Some(d)) => Some(d),
        (None, None) => None,
    };

    if let Some(ranges) = &args.ranges {
        return ranges_cut(&args, ranges, g);
    }
    if let Some(dropped) = &args.drop {
        return drop_cut(&args, dropped, g);
    }

    let mut argv = ffmpeg_base(g.progress);
    if args.accurate {
        argv.push("-i");
        argv.push(&args.input);
        if start > 0.0 {
            argv.extend(["-ss", &fmt_time(start)]);
        }
        if let Some(d) = dur {
            argv.extend(["-t", &fmt_time(d)]);
        }
        argv.extend([
            "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-c:a", "aac",
        ]);
    } else {
        if start > 0.0 {
            argv.extend(["-ss", &fmt_time(start)]);
        }
        argv.push("-i");
        argv.push(&args.input);
        if let Some(d) = dur {
            argv.extend(["-t", &fmt_time(d)]);
        }
        argv.extend(["-c", "copy", "-avoid_negative_ts", "make_zero"]);
    }
    argv.push(&args.output);

    engine::write_job("cut", &[&args.input], &args.output, vec![argv], g)
}

fn parse_ranges(ranges: &str, duration: f64) -> Result<Vec<(f64, f64)>, Error> {
    let mut segs: Vec<(f64, f64)> = Vec::new();
    for part in ranges.split(',') {
        let (a, b) = part
            .split_once('-')
            .ok_or_else(|| Error::input(format!("bad range '{part}' — use 10-20,40-50")))?;
        let (a, b) = if a.trim().eq_ignore_ascii_case("end") {
            // `end-N` = the last N seconds of the input
            let n = parse_time(b.trim())?;
            if n <= 0.0 || n >= duration {
                return Err(Error::input(format!(
                    "bad range '{part}' — tail length inside the input"
                )));
            }
            (duration - n, duration)
        } else if b.trim().eq_ignore_ascii_case("end") {
            // `T-end` = from T through the tail
            (parse_time(a.trim())?, duration)
        } else {
            (parse_time(a.trim())?, parse_time(b.trim())?)
        };
        if b <= a || a < 0.0 {
            return Err(Error::input(format!(
                "bad range '{part}' — end after start"
            )));
        }
        segs.push((a, b));
    }
    Ok(segs)
}

/// Keep only the listed ranges, joined into one output.
fn ranges_cut(args: &CutArgs, ranges: &str, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "cut --ranges")?;
    let mut segs = parse_ranges(ranges, probe.duration)?
        .into_iter()
        .map(|(a, b)| (a, b.min(probe.duration)))
        .collect::<Vec<_>>();
    if segs.len() < 2 {
        return Err(Error::input("pass 2+ ranges or use --start/--end"));
    }
    segs.sort_by(|x, y| x.0.total_cmp(&y.0));
    concat_segs(args, segs, probe, g)
}

/// Drop the listed ranges, keep everything else joined.
fn drop_cut(args: &CutArgs, ranges: &str, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "cut --drop")?;
    let mut drops = parse_ranges(ranges, probe.duration)?
        .into_iter()
        .map(|(a, b)| (a, b.min(probe.duration)))
        .collect::<Vec<_>>();
    drops.sort_by(|x, y| x.0.total_cmp(&y.0));
    let mut segs: Vec<(f64, f64)> = Vec::new();
    let mut cur = 0.0;
    for (a, b) in drops {
        if a > cur {
            segs.push((cur, a.min(probe.duration)));
        }
        cur = cur.max(b);
    }
    if cur < probe.duration {
        segs.push((cur, probe.duration));
    }
    segs.retain(|(a, b)| b - a > 0.01);
    if segs.is_empty() {
        return Err(Error::input("--drop removes the whole input"));
    }
    concat_segs(args, segs, probe, g)
}

/// Join several ranges into one output: N atrim/trim pairs + concat.
/// Always re-encodes (frame-exact, and the concat needs aligned pts anyway).
fn concat_segs(
    args: &CutArgs,
    segs: Vec<(f64, f64)>,
    probe: Probe,
    g: &Globals,
) -> Result<Contract, Error> {
    let mut seg = String::new();
    let mut labels = String::new();
    for (i, (a, b)) in segs.iter().enumerate() {
        seg.push_str(&format!(
            "[0:v]trim=start={a:.3}:end={b:.3},setpts=PTS-STARTPTS[v{i}];"
        ));
        if probe.has_audio {
            seg.push_str(&format!(
                "[0:a]atrim=start={a:.3}:end={b:.3},asetpts=PTS-STARTPTS[a{i}];"
            ));
            labels.push_str(&format!("[v{i}][a{i}]"));
        } else {
            labels.push_str(&format!("[v{i}]"));
        }
    }
    let fc = if probe.has_audio {
        format!("{seg}{labels}concat=n={}:v=1:a=1[vout][aout]", segs.len())
    } else {
        format!("{seg}{labels}concat=n={}:v=1:a=0[vout]", segs.len())
    };

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

    let c = engine::write_job("cut", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(serde_json::json!({
        "ranges": segs.iter().map(|(a, b)| format!("{a}-{b}")).collect::<Vec<_>>(),
        "kept": segs.iter().map(|(a, b)| b - a).sum::<f64>(),
    })))
}

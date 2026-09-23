use serde_json::json;

use crate::cli::{BoomerangArgs, Globals};
use crate::contract::{Contract, Status};
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: BoomerangArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "boomerang")?;
    if args.times == 0 {
        return Err(Error::input("--times must be >= 1"));
    }
    if probe.duration < 0.3 {
        return Err(Error::input("boomerang needs at least 0.3s of footage"));
    }

    if args.dur.is_some() && args.at.is_none() {
        return Err(Error::input("--dur requires --at"));
    }
    let windows = match &args.at {
        Some(raw) => {
            let w = crate::time::window_list(raw, args.dur, probe.duration)?;
            for (s, e) in &w {
                if e - s < 0.2 {
                    return Err(Error::input("a boomerang window needs at least 0.2s"));
                }
            }
            w
        }
        None => vec![(0.0, probe.duration)],
    };
    let win = args.at.is_some();

    let mut seg: Vec<String> = Vec::new();
    if !win {
        seg.push("[0:v]split[vf][vr]".to_string());
        if probe.has_audio {
            seg.push("[0:a]asplit[af][ar]".to_string());
        }
        seg.push("[vr]reverse[vrev]".to_string());
        if probe.has_audio {
            seg.push("[ar]areverse[arev]".to_string());
        }
        let mid_dur = probe.duration;
        if args.times > 1 {
            let n = args.times - 1;
            let frames = ((mid_dur * 2.0 * probe.fps.unwrap_or(30.0)).ceil() as u32) + 2;
            seg.push(format!(
                "[vf][vrev]concat=n=2:v=1:a=0[vb0];[vb0]loop=loop={n}:size={frames}[vboom]"
            ));
            if probe.has_audio {
                let samples =
                    ((mid_dur * 2.0 * probe.sample_rate.unwrap_or(44100) as f64).ceil() as u32) + 2;
                seg.push(format!(
                    "[af][arev]concat=n=2:v=0:a=1[ab0];[ab0]aloop=loop={n}:size={samples}[aboom]"
                ));
            }
        } else {
            seg.push("[vf][vrev]concat=n=2:v=1:a=0[vboom]".to_string());
            if probe.has_audio {
                seg.push("[af][arev]concat=n=2:v=0:a=1[aboom]".to_string());
            }
        }
        seg.push("[vboom]null[vout]".to_string());
        if probe.has_audio {
            seg.push("[aboom]anull[aout]".to_string());
        }
    } else {
        // alternating normal/boomeranged segments per window
        let mut bounds = vec![0.0];
        for (s, e) in &windows {
            bounds.push(*s);
            bounds.push(*e);
        }
        bounds.push(probe.duration);
        let mut vins = String::new();
        let mut ains = String::new();
        let mut nseg = 0usize;
        for i in 0..bounds.len() - 1 {
            let (s, e) = (bounds[i], bounds[i + 1]);
            if e - s < 0.01 {
                continue;
            }
            if i % 2 == 0 {
                seg.push(format!(
                    "[0:v]trim=start={s:.3}:end={e:.3},setpts=PTS-STARTPTS[v{i}]"
                ));
                vins.push_str(&format!("[v{i}]"));
                if probe.has_audio {
                    seg.push(format!(
                        "[0:a]atrim=start={s:.3}:end={e:.3},asetpts=PTS-STARTPTS[a{i}]"
                    ));
                    ains.push_str(&format!("[a{i}]"));
                }
            } else {
                let mid_dur = e - s;
                seg.push(format!(
                    "[0:v]trim=start={s:.3}:end={e:.3},setpts=PTS-STARTPTS,split[vf{i}][vr{i}]"
                ));
                seg.push(format!("[vr{i}]reverse[vrev{i}]"));
                if args.times > 1 {
                    let n = args.times - 1;
                    let frames = ((mid_dur * 2.0 * probe.fps.unwrap_or(30.0)).ceil() as u32) + 2;
                    seg.push(format!(
                        "[vf{i}][vrev{i}]concat=n=2:v=1:a=0[vb{i}];[vb{i}]loop=loop={n}:size={frames}[vboom{i}]"
                    ));
                } else {
                    seg.push(format!("[vf{i}][vrev{i}]concat=n=2:v=1:a=0[vboom{i}]"));
                }
                vins.push_str(&format!("[vboom{i}]"));
                if probe.has_audio {
                    seg.push(format!(
                        "[0:a]atrim=start={s:.3}:end={e:.3},asetpts=PTS-STARTPTS,asplit[af{i}][ar{i}]"
                    ));
                    seg.push(format!("[ar{i}]areverse[arev{i}]"));
                    if args.times > 1 {
                        let n = args.times - 1;
                        let samples = ((mid_dur * 2.0 * probe.sample_rate.unwrap_or(44100) as f64)
                            .ceil() as u32)
                            + 2;
                        seg.push(format!(
                            "[af{i}][arev{i}]concat=n=2:v=0:a=1[ab{i}];[ab{i}]aloop=loop={n}:size={samples}[aboom{i}]"
                        ));
                    } else {
                        seg.push(format!("[af{i}][arev{i}]concat=n=2:v=0:a=1[aboom{i}]"));
                    }
                    ains.push_str(&format!("[aboom{i}]"));
                }
            }
            nseg += 1;
        }
        if probe.has_audio {
            seg.push(format!(
                "{vins}concat=n={nseg}:v=1:a=0[vout];{ains}concat=n={nseg}:v=0:a=1[aout]"
            ));
        } else {
            seg.push(format!("{vins}concat=n={nseg}:v=1:a=0[vout]"));
        }
    }
    let mut fc = seg.join(";");
    let vmap = "[vout]";
    let amap = "[aout]";
    let _ = &mut fc;

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-filter_complex", &fc, "-map", vmap]);
    if probe.has_audio {
        argv.extend(["-map", amap, "-c:a", "aac"]);
    }
    argv.extend([
        "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
    ]);
    argv.push(&args.output);

    let c = engine::write_job("boomerang", &[&args.input], &args.output, vec![argv], g)?;
    let mut extra = json!({ "loops": args.times * 2 });
    if matches!(c.status, Status::Ok) {
        if let Ok(p) = engine::probe_or_err(&args.output, g) {
            extra["probe"] = json!(p);
        }
    }
    Ok(c.with_extra(extra))
}

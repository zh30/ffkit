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
    let (ms, me) = match &args.at {
        Some(raw) => {
            let at = crate::time::parse_time(raw)?;
            if at >= probe.duration - 0.05 {
                return Err(Error::input("--at is past the end of the input"));
            }
            let end = (at + args.dur.unwrap_or(probe.duration - at)).min(probe.duration);
            if end - at < 0.2 {
                return Err(Error::input("boomerang window needs at least 0.2s"));
            }
            (at, end)
        }
        None => (0.0, probe.duration),
    };
    let win = args.at.is_some();

    let mut seg: Vec<String> = vec![];
    if win {
        // head + mid + tail; only the mid segment is boomeranged.
        seg.push(format!("[0:v]trim=0:{ms:.3},setpts=PTS-STARTPTS[vhead]"));
        seg.push(format!(
            "[0:v]trim={ms:.3}:{me:.3},setpts=PTS-STARTPTS,split[vf][vr]"
        ));
        seg.push(format!("[0:v]trim={me:.3}:,setpts=PTS-STARTPTS[vtail]"));
        if probe.has_audio {
            seg.push(format!("[0:a]atrim=0:{ms:.3},asetpts=PTS-STARTPTS[aahead]"));
            seg.push(format!(
                "[0:a]atrim={ms:.3}:{me:.3},asetpts=PTS-STARTPTS,asplit[af][ar]"
            ));
            seg.push(format!("[0:a]atrim={me:.3}:,asetpts=PTS-STARTPTS[aatail]"));
        }
    } else {
        seg.push("[0:v]split[vf][vr]".to_string());
        if probe.has_audio {
            seg.push("[0:a]asplit[af][ar]".to_string());
        }
    }
    seg.push("[vr]reverse[vrev]".to_string());
    if probe.has_audio {
        seg.push("[ar]areverse[arev]".to_string());
    }
    // --times loops the mid segment in place (loop=size=0 is a no-op).
    let mid_dur = me - ms;
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
    if win {
        seg.push("[vhead][vboom][vtail]concat=n=3:v=1:a=0[vout]".to_string());
        if probe.has_audio {
            seg.push("[aahead][aboom][aatail]concat=n=3:v=0:a=1[aout]".to_string());
        }
    } else {
        seg.push("[vboom]null[vout]".to_string());
        if probe.has_audio {
            seg.push("[aboom]anull[aout]".to_string());
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

use serde_json::json;

use crate::cli::{BoomerangArgs, Globals};
use crate::contract::{Contract, Status};
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: BoomerangArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "boomerang")?;
    if probe.duration < 0.3 {
        return Err(Error::input("boomerang needs at least 0.3s of footage"));
    }

    let mut seg: Vec<String> = vec![
        "[0:v]split[vf][vr]".to_string(),
        "[vr]reverse[vrev]".to_string(),
        "[vf][vrev]concat=n=2:v=1:a=0[vout]".to_string(),
    ];
    if probe.has_audio {
        seg.insert(1, "[0:a]asplit[af][ar]".to_string());
        seg.insert(3, "[ar]areverse[arev]".to_string());
        seg.pop();
        seg.push("[af][arev]concat=n=2:v=0:a=1[aout]".to_string());
        seg.push("[vf][vrev]concat=n=2:v=1:a=0[vout]".to_string());
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

    let c = engine::write_job("boomerang", &[&args.input], &args.output, vec![argv], g)?;
    let mut extra = json!({ "loops": 2 });
    if matches!(c.status, Status::Ok) {
        if let Ok(p) = engine::probe_or_err(&args.output, g) {
            extra["probe"] = json!(p);
        }
    }
    Ok(c.with_extra(extra))
}

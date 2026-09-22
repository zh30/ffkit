use serde_json::json;

use crate::cli::{FreezeArgs, Globals};
use crate::contract::{Contract, Status};
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: FreezeArgs, g: &Globals) -> Result<Contract, Error> {
    if args.at.is_some() && args.end.is_some() {
        return Err(Error::input(
            "--at/--dur (mid-clip freeze) and --end (outro freeze) are exclusive",
        ));
    }
    if args.dur.is_some() && args.at.is_none() {
        return Err(Error::input(
            "--dur needs --at (or use --end for an outro freeze)",
        ));
    }
    if args.at.is_none() && args.end.is_none() {
        return Err(Error::input("freeze needs --at T [--dur D] or --end D"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "freeze")?;

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    let mut extra = json!({});

    if let Some(end) = args.end {
        // Outro freeze: clone the last frame for `end` seconds, silence under it.
        if !(0.1..=30.0).contains(&end) {
            return Err(Error::input("--end must be 0.1..30 seconds"));
        }
        let mut fc = format!("[0:v]tpad=stop=-1:stop_duration={end:.3}:stop_mode=clone[vout]");
        if probe.has_audio {
            fc.push_str(&format!(
                ";[0:a]apad,atrim=0:{tot:.3}[aout]",
                tot = probe.duration + end
            ));
        }
        argv.extend(["-filter_complex", &fc, "-map", "[vout]"]);
        if probe.has_audio {
            argv.extend(["-map", "[aout]", "-c:a", "aac"]);
        }
        extra["end"] = json!(end);
    } else {
        // Mid-clip freeze at t: [0..t] + cloned frame for dur + [t..].
        let at = crate::time::parse_time(args.at.as_deref().unwrap())?;
        let dur = args.dur.unwrap_or(1.0);
        if !(0.0..probe.duration - 0.1).contains(&at) {
            return Err(Error::input("--at must land inside the input"));
        }
        if !(0.1..=30.0).contains(&dur) {
            return Err(Error::input("--dur must be 0.1..30 seconds"));
        }
        let one_frame = 1.0 / probe.fps.unwrap_or(30.0).max(1.0);
        let mut fc = format!(
            "[0:v]trim=0:{at:.3},setpts=PTS-STARTPTS[v0];\
             [0:v]trim={at:.3}:{e:.3},setpts=PTS-STARTPTS,tpad=stop=-1:stop_duration={dur:.3}:stop_mode=clone[v1];\
             [0:v]trim=start={at:.3},setpts=PTS-STARTPTS[v2]",
            e = at + one_frame,
        );
        if probe.has_audio {
            fc.push_str(&format!(
                ";[0:a]atrim=0:{at:.3},asetpts=PTS-STARTPTS[a0];\
                 anullsrc=r=48000:cl=stereo,atrim=0:{dur:.3}[a1];\
                 [0:a]atrim=start={at:.3},asetpts=PTS-STARTPTS[a2];\
                 [v0][a0][v1][a1][v2][a2]concat=n=3:v=1:a=1[vout][aout]"
            ));
        } else {
            fc.push_str(";[v0][v1][v2]concat=n=3:v=1:a=0[vout]");
        }
        argv.extend(["-filter_complex", &fc, "-map", "[vout]"]);
        if probe.has_audio {
            argv.extend(["-map", "[aout]", "-c:a", "aac"]);
        }
        extra["at"] = json!(at);
        extra["dur"] = json!(dur);
    }

    argv.extend([
        "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
    ]);
    argv.push(&args.output);

    let c = engine::write_job("freeze", &[&args.input], &args.output, vec![argv], g)?;
    if matches!(c.status, Status::Ok) {
        if let Ok(p) = engine::probe_or_err(&args.output, g) {
            extra["probe"] = json!(p);
        }
    }
    Ok(c.with_extra(extra))
}

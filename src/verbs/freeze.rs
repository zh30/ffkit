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
    if args.zoom.is_some() && args.at.is_none() {
        return Err(Error::input("--zoom works with --at freezes"));
    }
    let zoom = match args.zoom {
        Some(z) if (1.0..=2.0).contains(&z) => z,
        Some(_) => return Err(Error::input("--zoom must be 1.0..2 (end scale)")),
        None => 0.0,
    };
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
        // --ease N: the N s before --at play at half-speed (swoop-in);
        // --reverse N: the N s before --at replay backwards (rewind-in).
        let ease = args.ease.unwrap_or(0.0).clamp(0.0, at);
        let rev = args.reverse.unwrap_or(0.0).clamp(0.0, at);
        if ease > 0.0 && rev > 0.0 {
            return Err(Error::input("--ease and --reverse are exclusive"));
        }
        let swoop = ease > 0.0 || rev > 0.0;
        let head_end = at - ease - rev;
        let mut fc = String::new();
        if ease > 0.0 {
            fc.push_str(&format!(
                "[0:v]trim=0:{head_end:.3},setpts=PTS-STARTPTS[v0];\
                 [0:v]trim={head_end:.3}:{at:.3},setpts=PTS-STARTPTS,setpts=PTS*2[ve];"
            ));
        } else if rev > 0.0 {
            fc.push_str(&format!(
                "[0:v]trim=0:{head_end:.3},setpts=PTS-STARTPTS[v0];\
                 [0:v]trim={head_end:.3}:{at:.3},setpts=PTS-STARTPTS,reverse,setpts=PTS-STARTPTS[ve];"
            ));
        } else {
            fc.push_str(&format!("[0:v]trim=0:{at:.3},setpts=PTS-STARTPTS[v0];"));
        }
        // the held frame is where the swoop lands (head_end for rewind, at otherwise)
        let hold_at = if rev > 0.0 { head_end } else { at };
        if zoom > 1.0 {
            // Push-in on the held frame: zoompan replays it for dur while the
            // crop window tightens linearly 1 → zoom.
            let (vw, vh) = match (probe.width, probe.height) {
                (Some(w), Some(h)) if w >= 16 && h >= 16 => (w & !1, h & !1),
                _ => return Err(Error::input("--zoom needs a video with known dimensions")),
            };
            let fps = probe.fps.unwrap_or(30.0).max(1.0);
            let frames = (dur * fps).round().max(1.0) as u32;
            fc.push_str(&format!(
                "[0:v]trim={hold_at:.3}:{e:.3},setpts=PTS-STARTPTS,\
                 zoompan=z='1+({zoom:.4}-1)*on/{frames}':x='iw/2-(iw/zoom/2)':y='ih/2-(ih/zoom/2)':d={frames}:s={vw}x{vh}:fps={fps:.3}[v1];\
                 [0:v]trim=start={at:.3},setpts=PTS-STARTPTS[v2]",
                e = hold_at + one_frame,
            ));
        } else {
            fc.push_str(&format!(
                "[0:v]trim={hold_at:.3}:{e:.3},setpts=PTS-STARTPTS,tpad=stop=-1:stop_duration={dur:.3}:stop_mode=clone[v1];\
                 [0:v]trim=start={at:.3},setpts=PTS-STARTPTS[v2]",
                e = hold_at + one_frame,
            ));
        }
        if probe.has_audio {
            let ease_audio = if ease > 0.0 {
                format!("[0:a]atrim={head_end:.3}:{at:.3},asetpts=PTS-STARTPTS,atempo=0.5[ae];")
            } else if rev > 0.0 {
                format!("[0:a]atrim={head_end:.3}:{at:.3},asetpts=PTS-STARTPTS,areverse,asetpts=PTS-STARTPTS[ae];")
            } else {
                String::new()
            };
            let a0_end = if swoop { head_end } else { at };
            fc.push_str(&format!(
                ";[0:a]atrim=0:{a0_end:.3},asetpts=PTS-STARTPTS[a0];\
                 {ease_audio}\
                 anullsrc=r=48000:cl=stereo,atrim=0:{dur:.3}[a1];\
                 [0:a]atrim=start={at:.3},asetpts=PTS-STARTPTS[a2];"
            ));
            if swoop {
                fc.push_str("[v0][a0][ve][ae][v1][a1][v2][a2]concat=n=4:v=1:a=1[vout][aout]");
            } else {
                fc.push_str("[v0][a0][v1][a1][v2][a2]concat=n=3:v=1:a=1[vout][aout]");
            }
        } else if swoop {
            fc.push_str(";[v0][ve][v1][v2]concat=n=4:v=1:a=0[vout]");
        } else {
            fc.push_str(";[v0][v1][v2]concat=n=3:v=1:a=0[vout]");
        }
        argv.extend(["-filter_complex", &fc, "-map", "[vout]"]);
        if probe.has_audio {
            argv.extend(["-map", "[aout]", "-c:a", "aac"]);
        }
        extra["at"] = json!(at);
        extra["dur"] = json!(dur);
        extra["ease"] = json!(ease);
        extra["reverse"] = json!(rev);
        extra["zoom"] = json!(zoom);
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

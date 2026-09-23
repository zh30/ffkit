//! `insert` — splice a whole clip into the middle of a base video
//! (b-roll beat, ad read, reaction cutaway) without a manual split+concat.

use std::path::Path;

use serde_json::json;

use crate::cli::{Globals, InsertArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::time::parse_time;

pub fn run(args: InsertArgs, g: &Globals) -> Result<Contract, Error> {
    let base = engine::probe_or_err(&args.input, g)?;
    let clip = engine::probe_or_err(&args.clip, g)?;
    if !base.has_video || !clip.has_video {
        return Err(Error::input("insert: both inputs need a video stream"));
    }
    if base.has_audio != clip.has_audio {
        return Err(Error::input(
            "insert: clip must have an audio stream when the base does",
        ));
    }
    let at = if args.at == "end" {
        // splice just before the tail (the bound below requires strictly-inside)
        base.duration - 0.06
    } else {
        parse_time(&args.at)?
    };
    if !(0.05..base.duration - 0.05).contains(&at) {
        return Err(Error::input(format!(
            "--at must sit inside the {:.2}s base",
            base.duration
        )));
    }
    let bw = base.width.unwrap_or(1280);
    let bh = base.height.unwrap_or(720);

    let clip_len = match args.dur {
        Some(d) if d > 0.0 => d.min(clip.duration),
        _ => clip.duration,
    };
    let vol = match args.volume {
        Some(v) if (0.0..=4.0).contains(&v) => format!(",volume={v:.4}"),
        Some(_) => return Err(Error::input("--volume must be 0..=4")),
        None => String::new(),
    };
    if let Some(tr) = &args.transition {
        return run_xfade(&args, &base, &clip, at, clip_len, bw, bh, tr, &vol, g);
    }

    // Three segments: base head, the clip (scaled to base size, --dur capped),
    // base tail. Same trim/atrim→concat chain as the windowed verbs.
    let mut seg = vec![format!(
        "[0:v]trim=0:{at:.3},setpts=PTS-STARTPTS[v0];\
         [1:v]trim=0:{clip_len:.3},setpts=PTS-STARTPTS,scale={bw}:{bh}:force_original_aspect_ratio=decrease,pad={bw}:{bh}:(ow-iw)/2:(oh-ih)/2,setsar=1[v1];\
         [0:v]trim={at:.3}:,setpts=PTS-STARTPTS[v2]"
    )];
    let mut pins = String::from("[v0][v1][v2]");
    if base.has_audio {
        seg.push(format!(
            "[0:a]atrim=0:{at:.3},asetpts=PTS-STARTPTS[a0];\
             [1:a]atrim=0:{clip_len:.3},asetpts=PTS-STARTPTS{vol}[a1];\
             [0:a]atrim={at:.3}:,asetpts=PTS-STARTPTS[a2]"
        ));
        // concat pads interleave per segment: v0,a0,v1,a1,v2,a2
        pins = "[v0][a0][v1][a1][v2][a2]".to_string();
        seg.push(format!("{pins}concat=n=3:v=1:a=1[vout][aout]"));
    } else {
        seg.push(format!("{pins}concat=n=3:v=1:a=0[vout]"));
    }

    let mut argv = ffmpeg_base(g.progress);
    argv.extend(["-i".to_string(), args.input.display().to_string()]);
    argv.extend(["-i".to_string(), args.clip.display().to_string()]);
    argv.extend(["-filter_complex".to_string(), seg.join(";")]);
    argv.extend(["-map".to_string(), "[vout]".to_string()]);
    if base.has_audio {
        argv.extend(["-map".to_string(), "[aout]".to_string()]);
    }
    argv.extend([
        "-c:v".to_string(),
        "libx264".to_string(),
        "-preset".to_string(),
        "fast".to_string(),
        "-crf".to_string(),
        "18".to_string(),
        "-pix_fmt".to_string(),
        "yuv420p".to_string(),
    ]);
    if base.has_audio {
        argv.extend(["-c:a".to_string(), "aac".to_string()]);
    }
    argv.push(args.output.display().to_string());

    let inputs: Vec<&Path> = vec![&args.input, &args.clip];
    let mut c = engine::write_job("insert", &inputs, &args.output, vec![argv], g)?;
    c = c.with_extra(json!({ "at": at }));
    Ok(c)
}

#[allow(clippy::too_many_arguments)]
fn run_xfade(
    args: &InsertArgs,
    base: &crate::probe::Probe,
    _clip: &crate::probe::Probe,
    at: f64,
    clip_len: f64,
    bw: u32,
    bh: u32,
    transition: &str,
    vol: &str,
    g: &Globals,
) -> Result<Contract, Error> {
    let d = args.duration.unwrap_or(0.4);
    if d <= 0.0 || d >= at || 2.0 * d > clip_len {
        return Err(Error::input(format!(
            "--duration {d}s needs --at > {d} and a clip longer than {:.2}s",
            (2.0 * d).min(clip_len)
        )));
    }
    // xfade offsets are in the FIRST input's timeline:
    //   head(0..at) ⨯ clip at at-d → then ⨯ tail at at+clip.dur-2d
    let off1 = at - d;
    let off2 = at + clip_len - 2.0 * d;
    let mut segs = vec![format!(
        "[0:v]trim=0:{at:.3},setpts=PTS-STARTPTS[v0];         [1:v]trim=0:{clip_len:.3},setpts=PTS-STARTPTS,scale={bw}:{bh}:force_original_aspect_ratio=decrease,pad={bw}:{bh}:(ow-iw)/2:(oh-ih)/2,setsar=1[v1];         [0:v]trim={at:.3}:,setpts=PTS-STARTPTS[v2];         [v0][v1]xfade=transition={transition}:duration={d:.3}:offset={off1:.3}[x1];         [x1][v2]xfade=transition={transition}:duration={d:.3}:offset={off2:.3}[vout]"
    )];
    if base.has_audio {
        segs.push(format!(
            "[0:a]atrim=0:{at:.3},asetpts=PTS-STARTPTS[a0];             [1:a]atrim=0:,asetpts=PTS-STARTPTS{vol}[a1];             [0:a]atrim={at:.3}:,asetpts=PTS-STARTPTS[a2];             [a0][a1]acrossfade=d={d:.3}[x1a];             [x1a][a2]acrossfade=d={d:.3}[aout]"
        ));
    }
    let mut argv = ffmpeg_base(g.progress);
    argv.extend(["-i".to_string(), args.input.display().to_string()]);
    argv.extend(["-i".to_string(), args.clip.display().to_string()]);
    argv.extend(["-filter_complex".to_string(), segs.join(";")]);
    argv.extend(["-map".to_string(), "[vout]".to_string()]);
    if base.has_audio {
        argv.extend(["-map".to_string(), "[aout]".to_string()]);
        argv.extend(["-c:a".to_string(), "aac".to_string()]);
    }
    argv.extend(["-c:v".to_string(), "libx264".to_string()]);
    argv.extend(["-crf".to_string(), "18".to_string()]);
    argv.extend(["-pix_fmt".to_string(), "yuv420p".to_string()]);
    argv.push(args.output.display().to_string());
    let inputs: Vec<&Path> = vec![&args.input, &args.clip];
    let mut c = engine::write_job("insert", &inputs, &args.output, vec![argv], g)?;
    c = c.with_extra(json!({
        "at": at,
        "transition": transition,
        "xfade_seconds": d,
    }));
    Ok(c)
}

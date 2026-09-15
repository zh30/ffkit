use std::path::Path;

use crate::cli::{BrollArgs, FitMode, Globals};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::paths;
use crate::time;

pub fn run(args: BrollArgs, g: &Globals) -> Result<Contract, Error> {
    if args.duration <= 0.0 || !args.duration.is_finite() {
        return Err(Error::input("--duration must be > 0"));
    }
    let at = time::parse_time(&args.at)?;
    let a = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&a, "broll")?;
    let b = engine::probe_or_err(&args.insert, g)?;
    engine::need_video(&b, "broll")?;
    if at >= a.duration {
        return Err(Error::input(format!(
            "--at {at} is past A-roll duration {:.3}s",
            a.duration
        )));
    }
    let end = (at + args.duration).min(a.duration);
    let w = paths::even(a.width.unwrap_or(1280)).max(2);
    let h = paths::even(a.height.unwrap_or(720)).max(2);

    let scale = match args.fit {
        FitMode::Crop => format!(
            "scale={w}:{h}:force_original_aspect_ratio=increase,crop={w}:{h}"
        ),
        FitMode::Pad => format!(
            "scale={w}:{h}:force_original_aspect_ratio=decrease,pad={w}:{h}:(ow-iw)/2:(oh-ih)/2:black"
        ),
    };

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.push("-i");
    argv.push(&args.insert);
    // Shift B so its first frame lands at --at on A's timeline. Without this,
    // a short insert (e.g. 0.5s B at --at 1) has already EOF'd and overlay
    // freezes the last frame for the whole window.
    let fc = format!(
        "[1:v]{scale},setsar=1,format=yuv420p,setpts=PTS-STARTPTS+{at:.3}/TB[br];[0:v][br]overlay=0:0:eof_action=repeat:enable='between(t,{at:.3},{end:.3})'[vout]"
    );
    argv.extend(["-filter_complex", &fc, "-map", "[vout]"]);
    if a.has_audio {
        argv.extend(["-map", "0:a", "-c:a", "copy"]);
    }
    argv.extend(["-c:v", "libx264", "-preset", "fast", "-crf", "18"]);
    // Delayed B can outlast A on the overlay timeline; pin output to A-roll length.
    argv.extend(["-t", &format!("{:.3}", a.duration)]);
    argv.push(&args.output);

    let inputs: Vec<&Path> = vec![&args.input, &args.insert];
    let mut c = engine::write_job("broll", &inputs, &args.output, vec![argv], g)?;
    c = c.with_extra(serde_json::json!({
        "at": at,
        "end": end,
        "insert": paths::display(&args.insert),
        "fit": match args.fit {
            FitMode::Crop => "crop",
            FitMode::Pad => "pad",
        },
    }));
    Ok(c)
}

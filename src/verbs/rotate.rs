use serde_json::json;

use crate::cli::{FlipMode, Globals, RotateArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: RotateArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "rotate")?;

    let vf = match (args.flip, args.angle) {
        (_, Some(a)) => {
            if !(-360.0..=360.0).contains(&a) || a == 0.0 {
                return Err(Error::input("--angle must be -360..=360 and nonzero"));
            }
            // rotate expands the canvas; crop back so the tilt fills the frame.
            format!("rotate=a={:.6}:c=black:out_w=iw:out_h=ih", a.to_radians())
        }
        (Some(FlipMode::H), None) => "hflip".to_string(),
        (Some(FlipMode::V), None) => "vflip".to_string(),
        (None, None) => match args.deg % 360 {
            90 => "transpose=1".to_string(),
            180 => "transpose=1,transpose=1".to_string(),
            270 => "transpose=2".to_string(),
            0 => return Err(Error::input("--deg 0 is a no-op; nothing to do")),
            d => return Err(Error::input(format!("--deg must be 90/180/270, got {d}"))),
        },
    };
    // transpose swaps W/H; mp4 yuv420p wants both even — scale keeps sanity
    // for odd sources (odd x odd stays odd under transpose only when even in).
    let vf = format!("{vf},scale=trunc(iw/2)*2:trunc(ih/2)*2,setsar=1");

    let fc = format!("[0:v]{vf}[vout]");
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-filter_complex", &fc, "-map", "[vout]"]);
    if probe.has_audio {
        argv.extend(["-map", "0:a?", "-c:a", "copy"]);
    }
    argv.extend([
        "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
    ]);
    argv.push(&args.output);

    let c = engine::write_job("rotate", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "deg": args.deg,
        "angle": args.angle,
        "flip": args.flip.map(|f| format!("{f:?}").to_lowercase()),
    })))
}

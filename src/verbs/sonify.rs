use serde_json::json;

use crate::cli::{Globals, SonifyArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

/// Play a picture as sound: spectrumsynth treats image columns as a sliding
/// spectrogram (bright pixels = loud harmonics — painted peaks become notes).
/// scroll scans left→right continuously and wraps, so the loop is seamless.
pub fn run(args: SonifyArgs, g: &Globals) -> Result<Contract, Error> {
    if !(0.1..=600.0).contains(&args.dur) || args.speed <= 0.0 {
        return Err(Error::input("--dur 0.1..600, --speed > 0"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    let (w, h) = (probe.width.unwrap_or(0), probe.height.unwrap_or(0));
    if w == 0 || h == 0 {
        return Err(Error::input("sonify: input has no image/video stream"));
    }
    // stills (png/jpg via image2 @25fps → duration ≈ one frame) loop for --dur
    let still = probe.duration < 0.5;
    let fps = if still {
        25.0
    } else {
        probe.fps.unwrap_or(25.0).max(1.0)
    };
    // scroll h = width-fraction per frame → sweeps the width once per span
    let span = if still {
        args.dur
    } else {
        probe.duration.max(0.1)
    };
    let hspd = args.speed / (fps * span);
    let fc = format!(
        "[0:v]format=gray,scroll=h={hspd:.6}[m];[m][1:v]spectrumsynth=slide=scroll:sample_rate={}[a]",
        args.sample_rate
    );

    let mut argv = ffmpeg_base(g.progress);
    if still {
        argv.extend(["-loop", "1"]);
    }
    argv.push("-i");
    argv.push(&args.input);
    argv.extend([
        "-f",
        "lavfi",
        "-i",
        &format!("nullsrc=size={w}x{h}:rate={fps:.2}"),
    ]);
    argv.extend(["-filter_complex", &fc, "-map", "[a]"]);
    if still {
        argv.extend(["-t", &format!("{:.3}", args.dur)]);
    }
    argv.push(&args.output);

    let c = engine::write_job("sonify", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "duration_s": if still { args.dur } else { span },
        "sample_rate": args.sample_rate,
        "speed": args.speed,
    })))
}

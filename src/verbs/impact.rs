use serde_json::json;

use crate::cli::{Globals, ImpactArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: ImpactArgs, g: &Globals) -> Result<Contract, Error> {
    if !(4..=80).contains(&args.amp) {
        return Err(Error::input("--amp must be 4..=80 shake pixels"));
    }
    if !(0.02..=0.5).contains(&args.flash) {
        return Err(Error::input("--flash must be 0.02..=0.5 seconds"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "impact")?;
    let (w, h) = (probe.width.unwrap_or(0), probe.height.unwrap_or(0));
    let px = args.amp;
    let at = args.at.unwrap_or(0.5);
    let fe = at + args.flash;
    let (pw, ph) = (w + px * 2, h + px * 2);

    // punch: shake the frame on a decaying sine after --at, flash white on top
    let shake = format!(
        "pad={pw}:{ph}:{px}:{px},\
         crop={w}:{h}:x='{px}+{a}*exp(-max(t-{at},0)*6)*sin(2*PI*(t-{at})*18)':\
         y='{px}+{a}*exp(-max(t-{at},0)*6)*cos(2*PI*(t-{at})*22)'",
        a = px - 2,
        at = at
    );
    let fc =
        format!("[0:v]{shake}[sh];[sh][1:v]overlay=0:0:enable='between(t,{at:.3},{fe:.3})'[v]");
    let src = format!(
        "color=c=white:size={w}x{h}:duration={:.3}:rate={:.3}",
        probe.duration.max(0.1),
        probe.fps.unwrap_or(30.0)
    );

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-f", "lavfi", "-i", &src]);
    argv.extend(["-filter_complex", &fc, "-map", "[v]", "-map", "0:a?"]);
    argv.extend([
        "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
    ]);
    if probe.has_audio {
        argv.extend(["-c:a", "copy"]);
    }
    argv.push(&args.output);

    let c2 = engine::write_job("impact", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c2.with_extra(json!({ "at": at, "amp": args.amp, "flash": args.flash })))
}

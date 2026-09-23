use serde_json::json;

use crate::cli::{Globals, WaveArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: WaveArgs, g: &Globals) -> Result<Contract, Error> {
    if !(1..=60).contains(&args.amp) {
        return Err(Error::input("--amp must be 1..=60 px horizontal shift"));
    }
    if !(0.1..=10.0).contains(&args.speed) {
        return Err(Error::input("--speed must be 0.1..=10 cycles/sec"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "wave")?;
    let fps = probe.fps.unwrap_or(30.0).max(1.0);

    // watery undulation: geq resamples each row offset by a sine over Y and time
    // (4.4 geq has N frame index, no t)
    let ph = format!(
        "2*PI*Y/{wl:.1}+{sp:.3}*N/{fps:.3}",
        wl = 40.0,
        sp = args.speed
    );
    let chain = format!(
        "geq=lum='p(X+{a}*sin({ph}),Y)':cb='p(X+{a}*sin({ph}),Y)':cr='p(X+{a}*sin({ph}),Y)'",
        a = args.amp
    );

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    let fc = match &args.at {
        Some(a2) => {
            let en = crate::time::enable_expr(a2, args.dur, probe.duration)?;
            format!(
                "[0:v]split[m][f];[f]{chain}[w];[m][w]blend=all_expr='if({en},B,A)'[v]",
                en = en.replace("(t,", "(T,")
            )
        }
        None => {
            if args.dur.is_some() {
                return Err(Error::input("--dur needs --at"));
            }
            format!("[0:v]{chain}[w];[w]copy[v]")
        }
    };
    argv.extend(["-filter_complex", &fc, "-map", "[v]", "-map", "0:a?"]);
    argv.extend([
        "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
    ]);
    if probe.has_audio {
        argv.extend(["-c:a", "copy"]);
    }
    argv.push(&args.output);

    let c2 = engine::write_job("wave", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c2.with_extra(json!({ "amp": args.amp, "speed": args.speed })))
}

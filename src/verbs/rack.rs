use serde_json::json;

use crate::cli::{Globals, RackArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: RackArgs, g: &Globals) -> Result<Contract, Error> {
    if !(0.05..=5.0).contains(&args.rate) {
        return Err(Error::input("--rate must be 0.05..=5 focus cycles/sec"));
    }
    if !(2.0..=40.0).contains(&args.blur) {
        return Err(Error::input("--blur must be 2..=40 (gblur sigma)"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "rack")?;

    // rack-focus breathing blur: blend source with a blurred copy on a sine
    let fc = format!(
        "[0:v]split[a][b];[b]gblur=sigma={b:.1}[bl];\
         [a][bl]blend=all_expr='A*(0.5-0.5*sin(2*PI*T*{r:.3}))+B*(0.5+0.5*sin(2*PI*T*{r:.3}))'[v]",
        b = args.blur,
        r = args.rate
    );

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-filter_complex", &fc, "-map", "[v]", "-map", "0:a?"]);
    argv.extend([
        "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
    ]);
    if probe.has_audio {
        argv.extend(["-c:a", "copy"]);
    }
    argv.push(&args.output);

    let c2 = engine::write_job("rack", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c2.with_extra(json!({ "rate": args.rate, "blur": args.blur })))
}

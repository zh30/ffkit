use serde_json::json;

use crate::cli::{BurstArgs, Globals};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: BurstArgs, g: &Globals) -> Result<Contract, Error> {
    if !(0.05..=0.9).contains(&args.strength) {
        return Err(Error::input("--strength must be 0.05..=0.9 smear opacity"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "burst")?;
    let (w, h) = (probe.width.unwrap_or(0), probe.height.unwrap_or(0));
    let z = 1.0 + 0.06 + args.strength * 0.10;
    let (zw, zh) = ((w as f64 * z) as u32 & !1, (h as f64 * z) as u32 & !1);

    // radial zoom smear: blown-up blurred copy behind the sharp frame
    let fc = format!(
        "[0:v]split[a][b];[b]scale={zw}:{zh},crop={w}:{h},gblur=sigma={sig:.1}[bb];\
         [a][bb]blend=all_opacity={o:.3}[v]",
        sig = 8.0 + args.strength * 30.0,
        o = args.strength
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

    let c2 = engine::write_job("burst", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c2.with_extra(json!({ "strength": args.strength })))
}

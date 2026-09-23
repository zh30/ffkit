use serde_json::json;

use crate::cli::{DuotoneArgs, Globals};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: DuotoneArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "duotone")?;

    let [sr, sg, sb] = crate::color::rgb(&args.shadow)?;
    let [hr, hg, hb] = crate::color::rgb(&args.highlight)?;

    // per-channel map: luminance 0 -> shadow color, 255 -> highlight color
    let lin = |lo: u8, hi: u8| -> String {
        match hi as i32 - lo as i32 {
            255 => "val".to_string(),
            d if d >= 0 => format!("{lo}+val*{:.5}", d as f64 / 255.0),
            d => format!("{lo}-val*{:.5}", (-d) as f64 / 255.0),
        }
    };
    let ch = format!(
        "format=gray,format=rgb24,lutrgb=r='{}':g='{}':b='{}'",
        lin(sr, hr),
        lin(sg, hg),
        lin(sb, hb)
    );

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    match &args.at {
        Some(s) => {
            // the gray conversion must be gated too — blend the duotone branch
            let en = crate::time::enable_expr(s, args.dur, probe.duration)?.replace("(t,", "(T,");
            let fc = format!("[0:v]split[m][f];[f]{ch}[d];[m][d]blend=all_expr='if({en},B,A)'[v]");
            argv.extend(["-filter_complex", &fc, "-map", "[v]", "-map", "0:a?"]);
        }
        None => {
            if args.dur.is_some() {
                return Err(Error::input("--dur needs --at"));
            }
            argv.extend(["-vf", &ch]);
        }
    }
    argv.extend([
        "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
    ]);
    if probe.has_audio {
        argv.extend(["-c:a", "copy"]);
    }
    argv.push(&args.output);

    let c = engine::write_job("duotone", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "shadow": args.shadow,
        "highlight": args.highlight,
        "filter": "gray+lutrgb linear map",
    })))
}

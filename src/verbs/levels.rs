use crate::cli::LevelsArgs;
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use serde_json::json;

/// Photoshop-style levels (colorlevels): remap input black/white points to
/// output range. `--in-min 0.06 --in-max 0.92` rescues crushed-lifted web
/// rips; `--out-min 0.08` gives the matte film fade.
pub fn run(args: LevelsArgs, g: &crate::cli::Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "levels")?;
    let (i0, i1) = (args.in_min.clamp(-1.0, 1.0), args.in_max.clamp(-1.0, 1.0));
    let (o0, o1) = (args.out_min.clamp(0.0, 1.0), args.out_max.clamp(0.0, 1.0));
    // same remap on r/g/b keeps hue; colorlevels takes -1..1 input points
    let mut vf = format!(
        "colorlevels=rimin={i0}:gimin={i0}:bimin={i0}:rimax={i1}:gimax={i1}:bimax={i1}:romin={o0}:gomin={o0}:bomin={o0}:romax={o1}:gomax={o1}:bomax={o1}"
    );
    if let Some(s) = &args.at {
        let win = crate::time::enable_expr(s, args.dur, probe.duration)?;
        vf = format!("{vf}:enable='{win}'");
    } else if args.dur.is_some() {
        return Err(Error::input("--dur needs --at"));
    }

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend([
        "-vf", &vf, "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
    ]);
    if probe.has_audio {
        argv.extend(["-c:a", "copy"]);
    }
    argv.push(&args.output);
    let c = engine::write_job("levels", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "in": [i0, i1],
        "out": [o0, o1],
        "filter": vf,
    })))
}

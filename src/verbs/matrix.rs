use crate::cli::MatrixArgs;
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use serde_json::json;

/// Convert between color matrices (colormatrix): the classic fix is
/// BT.601-flagged SD footage that went green/magenta in a BT.709 timeline —
/// `matrix --to bt709` re-interprets and converts (not just re-tags).
pub fn run(args: MatrixArgs, g: &crate::cli::Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "matrix")?;
    let code = |m: crate::cli::ColorMatrix| -> i32 {
        match m {
            crate::cli::ColorMatrix::Auto => -1,
            crate::cli::ColorMatrix::Bt709 => 0,
            crate::cli::ColorMatrix::Fcc => 1,
            crate::cli::ColorMatrix::Bt601 => 2,
            crate::cli::ColorMatrix::Smpte240m => 3,
            crate::cli::ColorMatrix::Bt2020 => 4,
        }
    };
    let mut vf = format!("colormatrix=src={}:dst={}", code(args.from), code(args.to));
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
    let c = engine::write_job("matrix", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "from": format!("{:?}", args.from).to_lowercase(),
        "to": format!("{:?}", args.to).to_lowercase(),
        "filter": vf,
    })))
}

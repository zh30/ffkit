use crate::cli::{MatrixArgs, MatrixEngine};
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
    let mut vf = match args.engine {
        Some(MatrixEngine::Colorspace) => {
            // colorspace converts primaries + transfer too, not just the
            // matrix coeff — the right tool for bt2020 HDR ↔ 709 SDR
            let all = |m: crate::cli::ColorMatrix| -> Result<&'static str, Error> {
                match m {
                    crate::cli::ColorMatrix::Auto => Ok(""),
                    crate::cli::ColorMatrix::Bt709 => Ok("bt709"),
                    crate::cli::ColorMatrix::Bt601 => Ok("bt601-6-625"),
                    crate::cli::ColorMatrix::Smpte240m => Ok("smpte240m"),
                    crate::cli::ColorMatrix::Bt2020 => Ok("bt2020"),
                    crate::cli::ColorMatrix::Fcc => Err(Error::input(
                        "colorspace engine has no fcc group — use --engine colormatrix",
                    )),
                }
            };
            let (iall, allv) = (all(args.from)?, all(args.to)?);
            match iall.is_empty() {
                true => format!("colorspace=all={allv}"),
                false => format!("colorspace=iall={iall}:all={allv}"),
            }
        }
        _ => format!("colormatrix=src={}:dst={}", code(args.from), code(args.to)),
    };
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

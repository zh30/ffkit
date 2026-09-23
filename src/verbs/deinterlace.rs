use serde_json::json;

use crate::cli::{DeinterlaceArgs, DeinterlaceMode, FieldParity, Globals};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: DeinterlaceArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "deinterlace")?;

    let mode = match args.mode {
        DeinterlaceMode::Frame => 0,
        DeinterlaceMode::Field => 1,
    };
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    let parity = match args.parity {
        FieldParity::Auto => "auto",
        FieldParity::Tff => "tff",
        FieldParity::Bff => "bff",
    };
    let vf = match args.engine.unwrap_or(crate::cli::DeintEngine::Yadif) {
        crate::cli::DeintEngine::Yadif => format!("yadif=mode={mode}:parity={parity}"),
        crate::cli::DeintEngine::Bwdif => format!("bwdif=mode={mode}:parity={parity}"),
        crate::cli::DeintEngine::Estdif => format!("estdif=mode={mode}:parity={parity}"),
        crate::cli::DeintEngine::Kerndeint => "kerndeint=sharp=1:twoway=1".to_string(),
        // fieldmatch+decimate: inverse telecine — reconstruct 23.976p film
        // frames from 29.97i, drop the duplicated frames after
        crate::cli::DeintEngine::Fieldmatch => {
            "fieldmatch=order=auto:combmatch=full,decimate=dupthresh=1.1".to_string()
        }
        // w3fdif: Martin Weston three-field — sharp diagonals on SD archives
        crate::cli::DeintEngine::W3fdif => {
            let par = match args.parity {
                crate::cli::FieldParity::Tff => "tff",
                _ => "bff",
            };
            format!("w3fdif=filter=complex:parity={par}:mode={mode}")
        }
        crate::cli::DeintEngine::Mcdeint => {
            let par = match args.parity {
                crate::cli::FieldParity::Tff => "tff",
                _ => "bff",
            };
            format!("mcdeint=parity={par}")
        }
        crate::cli::DeintEngine::Detelecine => {
            let field = match args.parity {
                crate::cli::FieldParity::Bff => "bottom",
                _ => "top",
            };
            format!("detelecine=pattern=23:first_field={field}")
        }
    };
    argv.extend(["-vf", &vf]);
    if probe.has_audio {
        argv.extend(["-c:a", "copy"]);
    }
    argv.extend([
        "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
    ]);
    argv.push(&args.output);

    let name = match args.mode {
        DeinterlaceMode::Frame => "frame",
        DeinterlaceMode::Field => "field",
    };
    let c = engine::write_job("deinterlace", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({ "mode": name, "parity": parity, "engine": format!("{:?}", args.engine.unwrap_or(crate::cli::DeintEngine::Yadif)).to_lowercase() })))
}

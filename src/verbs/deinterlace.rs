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
    argv.extend(["-vf", &format!("yadif=mode={mode}:parity={parity}")]);
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
    Ok(c.with_extra(json!({ "mode": name, "parity": parity })))
}

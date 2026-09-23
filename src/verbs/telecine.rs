use crate::cli::{FieldParity, TelecineArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use serde_json::json;

/// Telecine: pull 23.976/24p film content up to interlaced NTSC fields for
/// broadcast / DVD-era delivery. Inverse of `deinterlace --engine fieldmatch`.
pub fn run(args: TelecineArgs, g: &crate::cli::Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "telecine")?;
    let field = match args.field {
        FieldParity::Tff | FieldParity::Auto => "top",
        FieldParity::Bff => "bottom",
    };
    let vf = format!(
        "telecine=pattern={}:first_field={}",
        args.pattern.replace(['"', '\''], ""),
        field
    );
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
    let c = engine::write_job("telecine", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "pattern": args.pattern,
        "first_field": field,
    })))
}

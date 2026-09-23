use crate::cli::AberrateArgs;
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use serde_json::json;

/// Chromatic aberration (rgbashift): red slides left, blue right — cheap-lens
/// fringe, VHS edge rainbow, glitch accents. Green stays put so luma reads.
pub fn run(args: AberrateArgs, g: &crate::cli::Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "aberrate")?;
    let a = args.amount.clamp(-255, 255);
    let mut vf = format!("rgbashift=rh={a}:bh={}", -a);
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
    let c = engine::write_job("aberrate", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "amount": a,
        "filter": vf,
    })))
}

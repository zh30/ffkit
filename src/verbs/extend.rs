use crate::cli::{ExtendArgs, ExtendMode};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use serde_json::json;

/// Stretch edge pixels to fill border strips (fillborders): cleans up
/// chroma-key rims, leftover letterbox slivers, stray 1px frame lines.
pub fn run(args: ExtendArgs, g: &crate::cli::Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "extend")?;
    if args.left + args.right + args.top + args.bottom == 0 {
        return Err(Error::input(
            "set at least one of --left/--right/--top/--bottom",
        ));
    }
    let mode = match args.mode {
        ExtendMode::Smear => "smear",
        ExtendMode::Mirror => "mirror",
        ExtendMode::Fixed => "fixed",
        ExtendMode::Reflect => "reflect",
        ExtendMode::Wrap => "wrap",
        ExtendMode::Fade => "fade",
    };
    let vf = format!(
        "fillborders=left={}:right={}:top={}:bottom={}:mode={}",
        args.left, args.right, args.top, args.bottom, mode
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
    let c = engine::write_job("extend", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "left": args.left,
        "right": args.right,
        "top": args.top,
        "bottom": args.bottom,
        "mode": mode,
    })))
}

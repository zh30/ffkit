use serde_json::json;

use crate::cli::{Globals, SheetArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: SheetArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "sheet")?;
    if args.cols == 0 || args.rows == 0 || args.cols * args.rows > 100 {
        return Err(Error::input("--cols x --rows must be 1..=100 tiles"));
    }
    let n = (args.cols * args.rows) as f64;
    let fps = n / probe.duration.max(0.05);
    // Even tile size so h264-ish math stays sane; tiles shrink on huge inputs.
    let w = probe.width.unwrap_or(1280);
    let h = probe.height.unwrap_or(720);
    let tw = args.tile.max(16);
    let th = ((tw * h / w) / 2) * 2;
    let vf = format!(
        "fps={fps:.6},scale={tw}:{th},tile={cols}x{rows}:margin={margin}:padding={pad}:color=0x101010",
        cols = args.cols,
        rows = args.rows,
        margin = args.margin.or(args.pad).unwrap_or(6),
        pad = args.pad.unwrap_or(6),
    );
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-vf", &vf, "-frames:v", "1"]);
    argv.push(&args.output);

    let c = engine::write_job("sheet", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "cols": args.cols,
        "rows": args.rows,
        "tile": format!("{tw}x{th}"),
        "frames": args.cols * args.rows,
    })))
}

use serde_json::json;

use crate::cli::{BarEdge, Globals, ProgressArgs};
use crate::contract::{Contract, Status};
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::spawn::Argv;

pub fn run(args: ProgressArgs, g: &Globals) -> Result<Contract, Error> {
    if args.height == 0 || args.height > 200 {
        return Err(Error::input("--height must be 1..200 px"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "progress")?;

    // drawbox's w is init-only, so the bar is a strip sliding in:
    // tip lands at W*t/duration linearly.
    let y = match args.edge {
        BarEdge::Bottom => "main_h-overlay_h".to_string(),
        BarEdge::Top => "0".to_string(),
    };
    let fc = format!(
        "[0:v][1:v]overlay=x='-main_w+main_w*t/{dur:.3}':y='{y}':shortest=1[vout]",
        dur = probe.duration,
    );

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.push("-f");
    argv.push("lavfi");
    argv.push("-i");
    let bar_src = format!(
        "color=c={c}:size={w}x{h}:d={d:.3}",
        c = args.color,
        w = probe.width.unwrap_or(320),
        h = args.height,
        d = probe.duration,
    );
    argv.push(&bar_src);
    argv.extend(["-filter_complex", &fc, "-map", "[vout]"]);
    if probe.has_audio {
        argv.extend(["-map", "0:a", "-c:a", "aac"]);
    }
    argv.extend([
        "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
    ]);
    argv.push(&args.output);

    let argvs: Vec<Argv> = vec![argv];
    let c = engine::write_job("progress", &[&args.input], &args.output, argvs, g)?;
    let mut extra = json!({
        "color": args.color,
        "height": args.height,
        "edge": format!("{:?}", args.edge).to_lowercase(),
    });
    if matches!(c.status, Status::Ok) {
        if let Ok(p) = engine::probe_or_err(&args.output, g) {
            extra["probe"] = json!(p);
        }
    }
    Ok(c.with_extra(extra))
}

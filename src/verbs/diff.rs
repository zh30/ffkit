use serde_json::json;

use crate::cli::{DiffArgs, Globals};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: DiffArgs, g: &Globals) -> Result<Contract, Error> {
    let p1 = engine::probe_or_err(&args.a, g)?;
    let p2 = engine::probe_or_err(&args.b, g)?;
    engine::need_video(&p1, "diff")?;
    engine::need_video(&p2, "diff")?;
    let dur = p1.duration.min(p2.duration.max(0.0));

    // amplified difference: |a-b| gamma-boosted for readability; a scaled to b's spec
    let fc = if args.side {
        "[0:v][1:v]scale2ref[a][b];[b]split[b1][b2];\
         [b1][a]blend=all_mode=difference,eq=brightness=0.1:gamma=1.5[d];\
         [b2][d]hstack[v]"
            .to_string()
    } else {
        "[0:v][1:v]scale2ref[a][b];\
         [b][a]blend=all_mode=difference,eq=brightness=0.1:gamma=1.5[v]"
            .to_string()
    };

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.a);
    argv.push("-i");
    argv.push(&args.b);
    argv.extend(["-filter_complex", &fc, "-map", "[v]", "-t"]);
    argv.push(format!("{dur:.3}"));
    argv.extend([
        "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
    ]);
    argv.push(&args.output);

    let c2 = engine::write_job("diff", &[&args.a, &args.b], &args.output, vec![argv], g)?;
    Ok(c2.with_extra(json!({ "side": args.side })))
}

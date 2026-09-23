use serde_json::json;

use crate::cli::{Globals, IrisArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::time::enable_expr;

pub fn run(args: IrisArgs, g: &Globals) -> Result<Contract, Error> {
    if !(2..=100).contains(&args.radius) {
        return Err(Error::input("--radius must be 2..=100 (% of frame width)"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "iris")?;
    let en = match &args.at {
        Some(a) => enable_expr(a, args.dur, probe.duration)?,
        None => {
            if args.dur.is_some() {
                return Err(Error::input("--dur needs --at"));
            }
            "1".to_string()
        }
    };

    // peephole/spotlight: dim + desat everywhere except a hard circle at --x/--y
    let fc = format!(
        "[0:v]split[m][d];[d]eq=brightness={b:.3}:saturation={s:.3}[dk];\
         [m][dk]blend=all_expr='if(lt(hypot(X-W*{x:.3},Y-H*{y:.3}),W*{r:.4})*{en},A,B)'[v]",
        b = -0.08,
        s = 0.35,
        en = en.replace("(t,", "(T,"),
        x = args.x / 100.0,
        y = args.y / 100.0,
        r = args.radius as f64 / 100.0
    );

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-filter_complex", &fc, "-map", "[v]", "-map", "0:a?"]);
    argv.extend([
        "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
    ]);
    if probe.has_audio {
        argv.extend(["-c:a", "copy"]);
    }
    argv.push(&args.output);

    let c2 = engine::write_job("iris", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c2.with_extra(json!({ "x": args.x, "y": args.y, "radius": args.radius })))
}

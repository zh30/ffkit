use serde_json::json;

use crate::cli::{Globals, SelectiveArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::time::enable_expr;

pub fn run(args: SelectiveArgs, g: &Globals) -> Result<Contract, Error> {
    if !(0.01..=0.7).contains(&args.similarity) {
        return Err(Error::input("--similarity must be 0.01..=0.7"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "selective")?;
    let color = crate::color::lavfi(&args.color);

    // keep-color: colorkey makes matching pixels transparent so the mask is
    // bright everywhere ELSE; maskedmerge paints the desaturated copy there and
    // leaves the base colored frame on matching pixels
    let chain = format!(
        "split=3[o1][o2][g];[g]hue=s=0[gy];\
         [o1]format=rgba,colorkey=color={c}:similarity={s:.2}[k];[k]alphaextract[mk];\
         [o2][gy][mk]maskedmerge",
        c = color,
        s = args.similarity
    );
    let fc = match &args.at {
        Some(a) => {
            let en = enable_expr(a, args.dur, probe.duration)?;
            format!(
                "[0:v]split[m][f];[f]{chain}[sel];[m][sel]blend=all_expr='if({en},B,A)'[v]",
                en = en.replace("(t,", "(T,")
            )
        }
        None => {
            if args.dur.is_some() {
                return Err(Error::input("--dur needs --at"));
            }
            format!("[0:v]{chain}[sel];[sel]copy[v]")
        }
    };

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

    let c2 = engine::write_job("selective", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c2.with_extra(json!({ "color": args.color, "similarity": args.similarity })))
}

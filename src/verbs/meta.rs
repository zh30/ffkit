use serde_json::json;

use crate::cli::{Globals, MetaArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: MetaArgs, g: &Globals) -> Result<Contract, Error> {
    let _probe = engine::probe_or_err(&args.input, g)?;
    let tags: Vec<(&str, &str)> = [
        ("title", args.title.as_deref()),
        ("artist", args.artist.as_deref()),
        ("comment", args.comment.as_deref()),
    ]
    .into_iter()
    .filter_map(|(k, v)| v.map(|v| (k, v)))
    .collect();
    if let Some(r) = args.rotate {
        if ![0, 90, 180, 270].contains(&r) {
            return Err(Error::input("--rotate must be 0, 90, 180 or 270"));
        }
    }
    if tags.is_empty() && args.rotate.is_none() {
        return Err(Error::input(
            "meta needs at least one of --title/--artist/--comment/--rotate",
        ));
    }
    // ffmpeg >= 7 dropped the rotate metadata tag in favour of the
    // -display_rotation input option; older ffmpeg only knows the tag.
    let new_rotate = engine::ffmpeg_major().unwrap_or(0) >= 7;
    let mut argv = ffmpeg_base(g.progress);
    if let Some(r) = args.rotate {
        if new_rotate {
            argv.extend(["-display_rotation:v:0", &format!("-{r}")]);
        }
    }
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-map", "0", "-c", "copy"]);
    for (k, v) in &tags {
        argv.extend(["-metadata", &format!("{k}={v}")]);
    }
    if let Some(r) = args.rotate {
        if !new_rotate {
            argv.extend(["-metadata:s:v:0", &format!("rotate={r}")]);
        }
    }
    argv.push(&args.output);

    let c = engine::write_job("meta", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "tags": tags.iter().map(|(k, _)| k).collect::<Vec<_>>(),
        "rotate": args.rotate,
    })))
}

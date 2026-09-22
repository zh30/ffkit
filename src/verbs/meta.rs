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
    if tags.is_empty() {
        return Err(Error::input(
            "meta needs at least one of --title/--artist/--comment",
        ));
    }
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-map", "0", "-c", "copy"]);
    for (k, v) in &tags {
        argv.extend(["-metadata", &format!("{k}={v}")]);
    }
    argv.push(&args.output);

    let c = engine::write_job("meta", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "tags": tags.iter().map(|(k, _)| k).collect::<Vec<_>>(),
    })))
}

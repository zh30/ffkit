use serde_json::json;

use crate::cli::{Globals, MetaArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: MetaArgs, g: &Globals) -> Result<Contract, Error> {
    let _probe = engine::probe_or_err(&args.input, g)?;
    // --lyrics: unsynced lyrics embedded from an .lrc/.txt file — every
    // leading [mm:ss.xx]/[key:value] bracket group is stripped so players
    // see plain lines (chapter --lrc exports feed straight in)
    let lyrics_text = args
        .lyrics
        .as_deref()
        .map(|p| {
            let text = std::fs::read_to_string(p)
                .map_err(|e| Error::input(format!("--lyrics: {}: {e}", p.display())))?;
            Ok::<_, Error>(strip_lrc(&text))
        })
        .transpose()?;
    let bpm_text = args.bpm.map(|b| b.to_string());
    let tags: Vec<(&str, &str)> = [
        ("title", args.title.as_deref()),
        ("artist", args.artist.as_deref()),
        ("album", args.album.as_deref()),
        ("genre", args.genre.as_deref()),
        ("date", args.date.as_deref()),
        ("track", args.track.as_deref()),
        ("disc", args.disc.as_deref()),
        ("composer", args.composer.as_deref()),
        ("bpm", bpm_text.as_deref()),
        ("lyrics", lyrics_text.as_deref()),
        ("copyright", args.copyright.as_deref()),
        ("album_artist", args.album_artist.as_deref()),
        ("show", args.show.as_deref()),
        ("season_number", args.season.as_deref()),
        ("episode_id", args.episode.as_deref()),
        ("network", args.network.as_deref()),
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
    if args.clear && (!tags.is_empty() || args.rotate.is_some() || args.copy.is_some()) {
        return Err(Error::input(
            "--clear strips everything; drop the tag flags and --copy",
        ));
    }
    if tags.is_empty() && args.rotate.is_none() && !args.clear && args.copy.is_none() {
        return Err(Error::input(
            "meta needs at least one tag flag, --rotate, --clear or --copy",
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
    if let Some(src) = &args.copy {
        argv.push("-i");
        argv.push(src);
    }
    if args.clear {
        argv.extend(["-map_metadata", "-1"]);
    }
    argv.extend(["-map", "0", "-c", "copy"]);
    if args.copy.is_some() {
        argv.extend(["-map_metadata", "1", "-map_chapters", "1"]);
    }
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
        "copied_from": args.copy,
    })))
}

/// Strip leading [bracket] groups from each line — LRC timestamps
/// ([mm:ss.xx]) and tag headers ([ar:...]) alike — leaving plain lyric
/// text one line per source line.
fn strip_lrc(text: &str) -> String {
    text.lines()
        .filter_map(|line| {
            let mut s = line.trim();
            while let Some(rest) = s
                .strip_prefix('[')
                .and_then(|b| b.split_once(']').map(|(_, r)| r))
            {
                s = rest.trim_start();
            }
            (!s.is_empty()).then(|| s.to_string())
        })
        .collect::<Vec<_>>()
        .join("\n")
}

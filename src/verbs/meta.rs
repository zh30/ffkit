use serde_json::json;

use crate::cli::{Globals, MetaArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: MetaArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
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
    // --creation-time auto: stamp the INPUT file's own mtime — the real
    // shoot/download day without typing a date (archive daily reels)
    let creation_text = match args.creation_time.as_deref() {
        Some("auto") => Some(file_mtime_iso(&args.input)?),
        other => other.map(str::to_string),
    };
    // iTunes stik atom values: 0 music, 1 music video, 2 tv show,
    // 9 movie, 10 audiobook (movenc writes media_type verbatim)
    let media_text = match args.media_type.as_deref() {
        None => None,
        Some("music") => Some("0"),
        Some("musicvideo") | Some("music-video") => Some("1"),
        Some("tvshow") | Some("tv-show") => Some("2"),
        Some("movie") => Some("9"),
        Some("audiobook") => Some("10"),
        Some(other) => {
            return Err(Error::input(format!(
                "--media-type: '{other}' — pick music|musicvideo|tvshow|movie|audiobook"
            )))
        }
    };
    let gapless_text = if args.gapless { Some("1") } else { None };
    let comp_text = if args.compilation { Some("1") } else { None };
    let hd_text = if args.hd { Some("1") } else { None };
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
        // `bpm` lands on mp3/flac/mkv but the mp4 muxer whitelist drops it —
        // the iTunes tmpo atom needs the `tmpo` key, so emit both.
        ("tmpo", bpm_text.as_deref()),
        ("compilation", comp_text),
        ("lyrics", lyrics_text.as_deref()),
        ("copyright", args.copyright.as_deref()),
        ("album_artist", args.album_artist.as_deref()),
        ("show", args.show.as_deref()),
        ("season_number", args.season.as_deref()),
        ("episode_id", args.episode.as_deref()),
        ("network", args.network.as_deref()),
        ("creation_time", creation_text.as_deref()),
        ("location", args.location.as_deref()),
        ("media_type", media_text),
        ("gapless_playback", gapless_text),
        ("description", args.description.as_deref()),
        ("synopsis", args.synopsis.as_deref()),
        ("hd_video", hd_text),
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
    // --lang-audio/--lang-subs: per-track language tags in track order
    // (a:0 is the first audio track, s:0 the first subtitle track — the
    // comma list's position is the track index)
    let lang_audio = parse_lang_list(
        args.lang_audio.as_deref(),
        probe.streams.iter().filter(|s| s.kind == "audio").count(),
        "audio",
    )?;
    let lang_subs = parse_lang_list(
        args.lang_subs.as_deref(),
        probe
            .streams
            .iter()
            .filter(|s| s.kind == "subtitle")
            .count(),
        "subs",
    )?;
    let title_audio = parse_title_list(
        args.title_audio.as_deref(),
        probe.streams.iter().filter(|s| s.kind == "audio").count(),
        "audio",
    )?;
    let title_subs = parse_title_list(
        args.title_subs.as_deref(),
        probe
            .streams
            .iter()
            .filter(|s| s.kind == "subtitle")
            .count(),
        "subs",
    )?;
    let title_video = parse_title_list(
        args.title_video.as_deref(),
        probe.streams.iter().filter(|s| s.kind == "video").count(),
        "video",
    )?;
    if tags.is_empty()
        && lang_audio.is_empty()
        && lang_subs.is_empty()
        && title_audio.is_empty()
        && title_subs.is_empty()
        && title_video.is_empty()
        && args.rotate.is_none()
        && !args.clear
        && args.copy.is_none()
    {
        return Err(Error::input(
            "meta needs at least one tag flag, --rotate, --clear or --copy",
        ));
    }
    if args.clear
        && (!tags.is_empty()
            || !lang_audio.is_empty()
            || !lang_subs.is_empty()
            || !title_audio.is_empty()
            || !title_subs.is_empty()
            || !title_video.is_empty()
            || args.rotate.is_some()
            || args.copy.is_some())
    {
        return Err(Error::input(
            "--clear strips everything; drop the tag flags and --copy",
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
    for (i, l) in lang_audio.iter().enumerate() {
        if l.is_empty() {
            continue;
        }
        argv.extend([
            "-metadata:s:a:".to_string() + &i.to_string(),
            format!("language={l}"),
        ]);
    }
    for (i, l) in lang_subs.iter().enumerate() {
        if l.is_empty() {
            continue;
        }
        argv.extend([
            "-metadata:s:s:".to_string() + &i.to_string(),
            format!("language={l}"),
        ]);
    }
    for (i, t) in title_audio.iter().enumerate() {
        if t.is_empty() {
            continue;
        }
        argv.extend([
            "-metadata:s:a:".to_string() + &i.to_string(),
            format!("title={t}"),
        ]);
    }
    for (i, t) in title_subs.iter().enumerate() {
        if t.is_empty() {
            continue;
        }
        argv.extend([
            "-metadata:s:s:".to_string() + &i.to_string(),
            format!("title={t}"),
        ]);
    }
    for (i, t) in title_video.iter().enumerate() {
        if t.is_empty() {
            continue;
        }
        argv.extend([
            "-metadata:s:v:".to_string() + &i.to_string(),
            format!("title={t}"),
        ]);
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
        "lang_audio": lang_audio,
        "lang_subs": lang_subs,
        "title_audio": title_audio,
        "title_subs": title_subs,
    })))
}

/// `--title-{audio,subs}` comma list → per-track display names. Position
/// is the track index; blank slots leave the tag untouched.
fn parse_title_list(raw: Option<&str>, n_tracks: usize, kind: &str) -> Result<Vec<String>, Error> {
    let Some(raw) = raw else {
        return Ok(Vec::new());
    };
    let titles: Vec<String> = raw.split(',').map(str::trim).map(String::from).collect();
    if titles.iter().filter(|t| !t.is_empty()).count() > n_tracks {
        return Err(Error::input(format!(
            "meta --title-{kind}: input has {n_tracks} {kind} track(s), got more names"
        )));
    }
    Ok(titles)
}

/// `--lang-{audio,subs}` comma list → per-track ISO codes. Position in
/// the list is the track index; blank slots leave the tag untouched.
fn parse_lang_list(raw: Option<&str>, n_tracks: usize, kind: &str) -> Result<Vec<String>, Error> {
    let Some(raw) = raw else {
        return Ok(Vec::new());
    };
    let langs: Vec<String> = raw
        .split(',')
        .map(str::trim)
        .map(|s| s.to_lowercase())
        .collect();
    for l in &langs {
        if l.is_empty() {
            continue;
        }
        if l.len() != 3 || !l.chars().all(|c| c.is_ascii_lowercase()) {
            return Err(Error::input(format!(
                "meta --lang-{kind}: '{l}' — ISO-639-2 three-letter codes (eng, jpn, …)",
            )));
        }
    }
    if langs.iter().filter(|l| !l.is_empty()).count() > n_tracks {
        return Err(Error::input(format!(
            "meta --lang-{kind}: {} codes for {n_tracks} {kind} track(s)",
            langs.iter().filter(|l| !l.is_empty()).count(),
        )));
    }
    Ok(langs)
}

/// File mtime → ISO-8601 UTC (`YYYY-MM-DDTHH:MM:SSZ`).
/// Civil-from-days (Hinnant's algorithm) — no chrono in the dep tree.
fn file_mtime_iso(p: &std::path::Path) -> Result<String, Error> {
    let mtime = std::fs::metadata(p)
        .and_then(|m| m.modified())
        .map_err(|e| Error::input(format!("--creation-time auto: {}: {e}", p.display())))?;
    let secs = mtime
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let days = secs.div_euclid(86400);
    let tod = secs.rem_euclid(86400);
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    Ok(format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        tod / 3600,
        tod % 3600 / 60,
        tod % 60
    ))
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

use serde_json::json;

use crate::cli::{ChapterArgs, Globals};
use crate::contract::{Contract, Status};
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

fn yt_ts(t: f64) -> String {
    let s = t.round().max(0.0) as u64;
    if s >= 3600 {
        format!("{}:{:02}:{:02}", s / 3600, (s / 60) % 60, s % 60)
    } else {
        format!("{}:{:02}", s / 60, s % 60)
    }
}

/// Parse a YouTube-format chapter list ("mm:ss title" or "h:mm:ss title"
/// per line) into (seconds, title) marks — the file `chapter --yt` writes
/// and YouTube descriptions carry.
fn parse_podcast_json(text: &str, path: &std::path::Path) -> Result<Vec<(f64, String)>, Error> {
    let v: serde_json::Value = serde_json::from_str(text)
        .map_err(|e| Error::input(format!("--import: {}: bad JSON: {e}", path.display())))?;
    let arr = v["chapters"].as_array().ok_or_else(|| {
        Error::input(format!(
            "--import: {}: want {{\"chapters\":[{{\"startTime\":S,\"title\":T}}]}}",
            path.display()
        ))
    })?;
    let mut out = Vec::new();
    for (i, ch) in arr.iter().enumerate() {
        let t = ch["startTime"].as_f64().ok_or_else(|| {
            Error::input(format!(
                "--import: {}: chapter {} missing numeric startTime",
                path.display(),
                i + 1
            ))
        })?;
        let title = ch["title"].as_str().unwrap_or("").trim().to_string();
        out.push((t, title));
    }
    if out.is_empty() {
        return Err(Error::input(format!(
            "--import: {}: no chapters found",
            path.display()
        )));
    }
    Ok(out)
}

fn parse_cue_list(text: &str, path: &std::path::Path) -> Result<Vec<(f64, String)>, Error> {
    // CUE sheet: TITLE "name" inside a TRACK, then INDEX 01 mm:ss:ff (75fps)
    let mut out: Vec<(f64, String)> = Vec::new();
    let mut pending_title: Option<String> = None;
    let mut in_track = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with("TRACK") {
            in_track = true;
        } else if in_track && line.starts_with("TITLE") {
            let t = line[5..].trim().trim_matches('"').to_string();
            if !t.is_empty() {
                pending_title = Some(t);
            }
        } else if in_track && line.starts_with("INDEX 01") {
            let ts = line[8..].trim();
            let parts: Vec<&str> = ts.split(':').collect();
            if parts.len() == 3 {
                if let (Ok(mm), Ok(ss), Ok(ff)) = (
                    parts[0].parse::<f64>(),
                    parts[1].parse::<f64>(),
                    parts[2].parse::<f64>(),
                ) {
                    let secs = mm * 60.0 + ss + ff / 75.0;
                    let n = out.len() + 1;
                    out.push((
                        secs,
                        pending_title.take().unwrap_or_else(|| format!("Track {n}")),
                    ));
                }
            }
            pending_title = None;
        }
    }
    if out.is_empty() {
        return Err(Error::input(format!(
            "--import: {}: no INDEX 01 marks found",
            path.display()
        )));
    }
    Ok(out)
}

pub(crate) fn parse_yt_list(path: &std::path::Path) -> Result<Vec<(f64, String)>, Error> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| Error::input(format!("reading {}: {e}", path.display())))?;
    let mut marks = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let (ts, title) = line.split_once(' ').ok_or_else(|| {
            Error::input(format!(
                "{} line {}: needs 'mm:ss title'",
                path.display(),
                i + 1
            ))
        })?;
        let secs = crate::time::parse_time(ts).map_err(|_| {
            Error::input(format!(
                "{} line {}: bad time '{ts}'",
                path.display(),
                i + 1
            ))
        })?;
        let title = title.trim();
        if title.is_empty() {
            return Err(Error::input(format!(
                "{} line {}: empty title",
                path.display(),
                i + 1
            )));
        }
        marks.push((secs, title.to_string()));
    }
    if marks.is_empty() {
        return Err(Error::input(format!(
            "{}: no chapter marks found",
            path.display()
        )));
    }
    marks.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    Ok(marks)
}

/// A mark at/after the input's end produces a zero-or-negative-length
/// chapter that ffmpeg rejects outright — catch it with a clear error.
pub(crate) fn check_marks(marks: &[(f64, String)], duration: f64, flag: &str) -> Result<(), Error> {
    if let Some((t, _)) = marks.iter().find(|(t, _)| *t >= duration) {
        return Err(Error::input(format!(
            "{flag} mark at {t:.2}s is at/past the {duration:.2}s input end",
        )));
    }
    Ok(())
}

/// ffmetadata `[CHAPTER]` table for `-map_chapters` embedding — START/END
/// in ms (TIMEBASE=1/1000); the last mark runs to `duration`.
pub(crate) fn ffmeta_table(marks: &[(f64, String)], duration: f64) -> String {
    let mut meta = String::from(";FFMETADATA1\n");
    for (i, (t, title)) in marks.iter().enumerate() {
        let end = if i + 1 < marks.len() {
            marks[i + 1].0
        } else {
            duration
        };
        meta.push_str(&format!(
            "[CHAPTER]\nTIMEBASE=1/1000\nSTART={}\nEND={}\ntitle={}\n",
            (t * 1000.0).round() as i64,
            (end * 1000.0).round() as i64,
            title.replace('=', ";").replace('\n', " "),
        ));
    }
    meta
}

pub fn run(args: ChapterArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;

    if args.list {
        let embedded = crate::probe::chapter_marks(&args.input, g.timeout)?;
        let mut c = Contract::ok("chapter", None, None);
        c = c.with_extra(json!({
            "chapters": embedded
                .iter()
                .map(|m| json!({"start": m.start, "title": m.title}))
                .collect::<Vec<_>>(),
        }));
        return Ok(c);
    }

    if args.remove {
        let mut argv = ffmpeg_base(g.progress);
        argv.push("-i");
        argv.push(&args.input);
        argv.extend([
            "-map",
            "0",
            "-c",
            "copy",
            "-map_chapters",
            "-1",
            "-movflags",
            "+faststart",
        ]);
        argv.push(&args.output);
        let c = engine::write_job("chapter", &[&args.input], &args.output, vec![argv], g)?;
        return Ok(c.with_extra(json!({ "chapters_removed": true })));
    }

    let mut marks: Vec<(f64, String)> = Vec::new();
    if let Some(min_gap) = args.auto {
        let silences = crate::silence::detect(&args.input, -35.0, min_gap, g.timeout, true)?;
        for (i, (_s, e)) in silences.iter().enumerate() {
            let t = *e;
            if t < probe.duration - 0.2 {
                marks.push((t, format!("Part {}", i + 2)));
            }
        }
        if !marks.is_empty() {
            marks.insert(0, (0.0, "Part 1".to_string()));
        }
    }
    if let Some(path) = &args.import {
        let text = std::fs::read_to_string(path)
            .map_err(|e| Error::input(format!("--import: {}: {e}", path.display())))?;
        let kind = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if kind == "json" {
            marks.extend(parse_podcast_json(&text, path)?);
        } else if kind == "cue" {
            marks.extend(parse_cue_list(&text, path)?);
        } else {
            for (ln, line) in text.lines().enumerate() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                let (t, title) = line
                .split_once('|')
                .or_else(|| line.split_once(','))
                // YouTube-description style: "0:00 Intro" / "1:02:33 Outro"
                .or_else(|| line.split_once(' '))
                .ok_or_else(|| {
                    Error::input(format!(
                        "--import line {}: want TIME|TITLE, TIME,TITLE or 'H:MM:SS Title', got '{line}'",
                        ln + 1
                    ))
                })?;
                let secs = crate::time::parse_time(t.trim()).map_err(|_| {
                    Error::input(format!("--import line {}: bad time '{t}'", ln + 1))
                })?;
                let title = title.trim().to_string();
                if title.is_empty() {
                    return Err(Error::input(format!(
                        "--import line {}: empty title",
                        ln + 1
                    )));
                }
                marks.push((secs, title));
            }
        }
    }
    for raw in &args.at {
        let (t, title) = raw
            .split_once('|')
            .ok_or_else(|| Error::input(format!("--at wants TIME|TITLE, got '{raw}'")))?;
        let secs = crate::time::parse_time(t.trim())?;
        let title = title.trim().to_string();
        if title.is_empty() {
            return Err(Error::input("chapter title must not be empty"));
        }
        marks.push((secs, title));
    }
    if let Some(shift) = args.shift {
        if shift != 0.0 {
            for m in &mut marks {
                m.0 = (m.0 + shift).max(0.0);
            }
        }
    }
    marks.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    marks.dedup_by(|a, b| (a.0 - b.0).abs() < 0.05);
    if marks.is_empty() {
        return Err(Error::input(
            "chapter needs --at TIME|TITLE or --auto found no silence gaps",
        ));
    }
    if marks[0].0 > 0.05 {
        marks[0].0 = 0.0;
    }
    for (t, _) in &marks {
        if *t >= probe.duration - 0.05 {
            return Err(Error::input(format!(
                "chapter at {t}s lands at/past the end ({:.1}s)",
                probe.duration
            )));
        }
    }

    let meta = ffmeta_table(&marks, probe.duration);
    if args.export || args.yt || args.cue || args.podcast || args.lrc {
        let text = if args.lrc {
            // LRC synced-lyrics format: one [mm:ss.xx]mark per line — music
            // players (and lyric tools) show them as seekable verse/track cues
            marks
                .iter()
                .map(|(t, ti)| {
                    let c = (*t * 100.0).round() as u64;
                    format!(
                        "[{:02}:{:02}.{:02}]{}",
                        c / 6000,
                        (c / 100) % 60,
                        c % 100,
                        ti
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
                + "\n"
        } else if args.podcast {
            // Podcasting 2.0 chapters JSON — podverse/podcastindex feeds
            // read {"chapters":[{"startTime":sec,"title":…}]}
            serde_json::to_string_pretty(&json!({
                "chapters": marks
                    .iter()
                    .map(|(t, ti)| json!({"startTime": t, "title": ti}))
                    .collect::<Vec<_>>()
            }))
            .map_err(|e| Error::output(format!("podcast chapters: {e}")))?
                + "\n"
        } else if args.yt {
            // YouTube description format — paste under the video and the
            // platform turns the marks into seek chapters.
            marks
                .iter()
                .map(|(t, ti)| format!("{} {}", yt_ts(*t), ti))
                .collect::<Vec<_>>()
                .join("\n")
                + "\n"
        } else if args.cue {
            // CUE sheet: INDEX times are mm:ss:ff at 75 frames/sec
            let media = match args
                .input
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_ascii_lowercase())
                .as_deref()
            {
                Some("mp3") => "MP3",
                Some("wav") | Some("wave") => "WAVE",
                Some("aif") | Some("aiff") => "AIFF",
                _ => "BINARY",
            };
            let name = args
                .input
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| args.input.display().to_string());
            let mut s = format!("FILE \"{name}\" {media}\n");
            for (i, (t, ti)) in marks.iter().enumerate() {
                let total = (*t * 75.0).round() as u64;
                s.push_str(&format!(
                    "  TRACK {:02} AUDIO\n    TITLE \"{}\"\n    INDEX 01 {:02}:{:02}:{:02}\n",
                    i + 1,
                    ti.replace('"', "'"),
                    total / 4500,
                    (total / 75) % 60,
                    total % 75,
                ));
            }
            s
        } else {
            meta
        };
        std::fs::write(&args.output, &text).map_err(|e| {
            Error::output(format!(
                "writing chapter metadata {}: {e}",
                args.output.display()
            ))
        })?;
        let mut c = Contract::ok("chapter", Some(args.output.display().to_string()), None);
        c = c.with_extra(json!({
            "exported": if args.lrc { "lrc" } else if args.podcast { "podcast" } else if args.yt { "youtube" } else if args.cue { "cue" } else { "ffmetadata" },
            "chapters": marks
                .iter()
                .map(|(t, ti)| json!({"time": t, "title": ti}))
                .collect::<Vec<_>>(),
        }));
        return Ok(c);
    }
    let meta_path = args.output.with_extension("ffmeta.txt");
    std::fs::write(&meta_path, &meta).map_err(|e| {
        Error::output(format!(
            "writing chapter metadata {}: {e}",
            meta_path.display()
        ))
    })?;

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.push("-i");
    argv.push(&meta_path);
    argv.extend([
        "-map_metadata",
        "1",
        "-map_chapters",
        "1",
        "-c",
        "copy",
        "-movflags",
        "+faststart",
    ]);
    argv.push(&args.output);

    let c = engine::write_job("chapter", &[&args.input], &args.output, vec![argv], g)?;
    let _ = std::fs::remove_file(&meta_path);
    let mut extra = json!({
        "chapters": marks
            .iter()
            .map(|(t, ti)| json!({"time": t, "title": ti}))
            .collect::<Vec<_>>(),
    });
    if matches!(c.status, Status::Ok) {
        if let Ok(p) = engine::probe_or_err(&args.output, g) {
            extra["probe"] = json!(p);
        }
    }
    Ok(c.with_extra(extra))
}

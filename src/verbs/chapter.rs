use serde_json::json;

use crate::cli::{ChapterArgs, Globals};
use crate::contract::{Contract, Status};
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::spawn::{self, Argv};

/// WebVTT cue timestamp: HH:MM:SS.mmm
fn vtt_ts(t: f64) -> String {
    let ms = (t * 1000.0).round() as u64;
    format!(
        "{:02}:{:02}:{:02}.{:03}",
        ms / 3_600_000,
        (ms / 60_000) % 60,
        (ms / 1000) % 60,
        ms % 1000
    )
}

/// `HH:MM:SS.mmm` or `MM:SS.mmm` → seconds (chapter --import .vtt)
fn parse_vtt_ts(s: &str) -> Option<f64> {
    let (hms, ms) = s.split_once('.')?;
    let ms: f64 = format!("0.{ms}").parse().ok()?;
    let parts: Vec<&str> = hms.split(':').collect();
    let secs = match parts.as_slice() {
        [m, s] => m.parse::<f64>().ok()? * 60.0 + s.parse::<f64>().ok()?,
        [h, m, s] => {
            h.parse::<f64>().ok()? * 3600.0
                + m.parse::<f64>().ok()? * 60.0
                + s.parse::<f64>().ok()?
        }
        _ => return None,
    };
    Some(secs + ms)
}

/// Parse the WebVTT chapter file `chapter --vtt` writes: cue start → cue text
/// (first line only; cue identifiers and timing-line settings are skipped).
fn parse_vtt_list(text: &str, path: &std::path::Path) -> Result<Vec<(f64, String)>, Error> {
    let mut marks = Vec::new();
    let mut lines = text.lines().peekable();
    let mut ln = 0usize;
    while let Some(raw) = lines.next() {
        ln += 1;
        let line = raw.trim();
        let Some((start, _end)) = line.split_once("-->") else {
            continue;
        };
        let secs = parse_vtt_ts(start.trim()).ok_or_else(|| {
            Error::input(format!(
                "{} line {ln}: bad VTT time '{line}'",
                path.display()
            ))
        })?;
        let title = lines
            .next()
            .map(|l| l.trim().to_string())
            .unwrap_or_default();
        ln += 1;
        if title.is_empty() {
            return Err(Error::input(format!(
                "{} line {ln}: cue has no title text",
                path.display()
            )));
        }
        marks.push((secs, title));
    }
    if marks.is_empty() {
        return Err(Error::input(format!(
            "{}: no VTT chapter cues found",
            path.display()
        )));
    }
    marks.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    Ok(marks)
}

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
fn parse_edl(text: &str, fps: f64) -> Result<Vec<(f64, String)>, Error> {
    // CMX-style EDL: "NNN  AX  V  C  <src-in> <src-out> <rec-in> <rec-out>"
    // + "* FROM CLIP NAME: title" — take the record-in TC as the mark and
    // the clip name as its title (our --edl export writes this shape)
    let tc = |s: &str| -> Option<f64> {
        let p: Vec<f64> = s.split(':').filter_map(|x| x.parse().ok()).collect();
        if p.len() == 4 {
            Some(p[0] * 3600.0 + p[1] * 60.0 + p[2] + p[3] / fps.max(1.0))
        } else {
            None
        }
    };
    let mut out = Vec::new();
    let mut pending_t: Option<f64> = None;
    for line in text.lines() {
        let l = line.trim();
        if let Some(rest) = l.strip_prefix("* FROM CLIP NAME:") {
            if let Some(t) = pending_t.take() {
                out.push((t, rest.trim().to_string()));
            }
            continue;
        }
        let parts: Vec<&str> = l.split_whitespace().collect();
        if parts.len() >= 8 && parts[0].chars().all(|c| c.is_ascii_digit()) {
            if let Some(t) = tc(parts[6]) {
                if let Some(t) = pending_t.replace(t) {
                    out.push((t, format!("event {}", parts[0])));
                }
            }
        }
    }
    if let Some(t) = pending_t.take() {
        out.push((t, "event".to_string()));
    }
    if out.is_empty() {
        return Err(Error::input("no CMX events found in the .edl"));
    }
    Ok(out)
}

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

// Final Cut Pro XML markers — the FCP/Resolve round-trip counterpart to
// `chapter --fcpxml`: <marker start="Ts" value="Title"/> elements become
// chapter marks (XML entities unescaped)
fn parse_fcpxml(text: &str, path: &std::path::Path) -> Result<Vec<(f64, String)>, Error> {
    let un = |v: &str| {
        v.replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
    };
    let attr = |tag: &str, key: &str| -> Option<String> {
        let pat = format!("{key}=\"");
        tag.find(&pat).and_then(|j| {
            let v = &tag[j + pat.len()..];
            v.find('"').map(|e| v[..e].to_string())
        })
    };
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(i) = rest.find("<marker ") {
        rest = &rest[i..];
        let end = rest.find('>').unwrap_or(rest.len());
        let tag = &rest[..end];
        let t = attr(tag, "start").and_then(|v| v.trim_end_matches('s').parse::<f64>().ok());
        let title = attr(tag, "value").map(|v| un(&v)).unwrap_or_default();
        if let (Some(t), false) = (t, title.is_empty()) {
            out.push((t, title));
        }
        rest = &rest[end..];
    }
    if out.is_empty() {
        return Err(Error::input(format!(
            "--import: {}: no <marker> elements",
            path.display()
        )));
    }
    Ok(out)
}

// .lrc synced-lyrics import: [mm:ss.xx] per line, optional multi-stamp
// lines ([t1][t2]text → one mark each), [key:value] header tags skipped.
// Round-trips with `chapter --lrc` export.
fn parse_lrc_list(text: &str, path: &std::path::Path) -> Result<Vec<(f64, String)>, Error> {
    let mut marks = Vec::new();
    for (ln, line) in text.lines().enumerate() {
        let mut rest = line.trim();
        if rest.is_empty() {
            continue;
        }
        let mut times = Vec::new();
        let mut is_tag = false;
        while let Some(body) = rest.strip_prefix('[') {
            let Some((b, r)) = body.split_once(']') else {
                break;
            };
            if b.contains(':') {
                match crate::time::parse_time(b.trim()) {
                    Ok(t) => times.push(t),
                    Err(_) => is_tag = true,
                }
            } else {
                is_tag = true; // bare numbers like [offset:+500] are metadata
            }
            rest = r.trim_start();
        }
        if times.is_empty() {
            if is_tag {
                continue; // header tags like [ti:title] carry no marks
            }
            return Err(Error::input(format!(
                "--import: {} line {}: no [mm:ss.xx] timestamp",
                path.display(),
                ln + 1
            )));
        }
        let title = rest.trim();
        if title.is_empty() {
            continue;
        }
        for t in times {
            marks.push((t, title.to_string()));
        }
    }
    if marks.is_empty() {
        return Err(Error::input(format!(
            "--import: {}: no [mm:ss.xx] marks found",
            path.display()
        )));
    }
    Ok(marks)
}

/// `chapter --csv` round-trip: `H:MM:SS.mmm,Title` per line; tolerates
/// a `Time`/`Name`-style header and quoted titles (Resolve/sheets).
/// FFMETADATA chapter blocks: `[CHAPTER] TIMEBASE=1/1000 START=0 END=…`
/// `title=…` — the FFmpeg-native chapter exchange `ffmpeg -i in -f
/// ffmetadata` exports; `remux --chapters` accepts the same mark list.
fn parse_ffmeta_list(text: &str, path: &std::path::Path) -> Result<Vec<(f64, String)>, Error> {
    let mut out: Vec<(f64, String)> = Vec::new();
    let mut start: Option<f64> = None;
    let mut timebase: Option<f64> = None;
    let mut title: Option<String> = None;
    let mut in_chapter = false;
    for (ln, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.eq_ignore_ascii_case("[chapter]") {
            if let (Some(st), Some(tl)) = (start.take(), title.take()) {
                if let Some(tb) = timebase {
                    out.push((st * tb, tl));
                }
            }
            in_chapter = true;
            start = None;
            timebase = None;
            title = None;
            continue;
        }
        if line.starts_with('[') {
            in_chapter = false;
        }
        if !in_chapter {
            continue;
        }
        if let Some(v) = line.strip_prefix("TIMEBASE=") {
            if let Some((n, d)) = v.trim().split_once('/') {
                if let (Ok(n), Ok(d)) = (n.parse::<f64>(), d.parse::<f64>()) {
                    if d > 0.0 {
                        timebase = Some(n / d);
                    }
                }
            }
        } else if let Some(v) = line.strip_prefix("START=") {
            if let Ok(v) = v.trim().parse::<f64>() {
                start = Some(v);
            }
        } else if let Some(v) = line.strip_prefix("title=") {
            title = Some(v.trim().to_string());
        }
        let _ = ln;
    }
    if let (Some(st), Some(tl)) = (start, title) {
        if let Some(tb) = timebase {
            out.push((st * tb, tl));
        }
    }
    if out.is_empty() {
        return Err(Error::input(format!(
            "--import {}: no [CHAPTER] blocks with START/title",
            path.display()
        )));
    }
    Ok(out)
}

fn parse_csv_list(text: &str, path: &std::path::Path) -> Result<Vec<(f64, String)>, Error> {
    let mut marks = Vec::new();
    for (ln, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (t, title) = line.split_once(',').ok_or_else(|| {
            Error::input(format!(
                "--import {path:?} line {}: want TIME,TITLE",
                ln + 1
            ))
        })?;
        let secs = match crate::time::parse_time(t.trim()) {
            Ok(s) => s,
            Err(e) => {
                if ln == 0 {
                    continue; // header row (Timecode,Name)
                }
                return Err(e);
            }
        };
        let mut title = title.trim().to_string();
        if title.starts_with('"') && title.ends_with('"') && title.len() >= 2 {
            title = title[1..title.len() - 1].replace("\"\"", "\"");
        }
        if title.is_empty() {
            return Err(Error::input(format!(
                "--import {path:?} line {}: empty title",
                ln + 1
            )));
        }
        marks.push((secs, title.to_string()));
    }
    Ok(marks)
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

    if args.scenes {
        if args.auto.is_some() {
            return Err(Error::input(
                "chapter --scenes and --auto are exclusive (different detectors)",
            ));
        }
        if !probe.has_video {
            return Err(Error::input("chapter --scenes needs a video input"));
        }
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
    if args.scenes {
        // scdet flags each cut — chapters for lecture/talking-head footage
        // where the silence gaps --auto listens for don't exist. One extra
        // decode pass; metadata=print writes lavfi.scd.time to stdout.
        let mut det = Argv::ffmpeg();
        det.push("-i");
        det.push(&args.input);
        det.extend([
            "-vf",
            "scdet=t=10,metadata=print:file=-",
            "-an",
            "-f",
            "null",
            "-",
        ]);
        let sp = spawn::require_ok(&det, spawn::run(&det, g.timeout, false)?)?;
        let mut cuts: Vec<f64> = Vec::new();
        for line in spawn::stdout_str(&sp).unwrap_or_default().lines() {
            if let Some(rest) = line
                .split("lavfi.scd.time=")
                .nth(1)
                .or_else(|| line.split("lavfi.scd.time:").nth(1))
            {
                if let Ok(t) = rest.trim().split(' ').next().unwrap_or("").parse::<f64>() {
                    if t > 0.2 && t < probe.duration - 0.2 {
                        cuts.push(t);
                    }
                }
            }
        }
        cuts.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        if !cuts.is_empty() {
            marks.push((0.0, "Scene 1".to_string()));
            for (i, t) in cuts.iter().enumerate() {
                marks.push((*t, format!("Scene {}", i + 2)));
            }
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
        } else if kind == "lrc" {
            marks.extend(parse_lrc_list(&text, path)?);
        } else if kind == "vtt" {
            marks.extend(parse_vtt_list(&text, path)?);
        } else if kind == "csv" {
            marks.extend(parse_csv_list(&text, path)?);
        } else if kind == "fcpxml" || kind == "xml" {
            marks.extend(parse_fcpxml(&text, path)?);
        } else if kind == "edl" {
            marks.extend(parse_edl(&text, args.fps.unwrap_or(30.0))?);
        } else if kind == "ffmeta" || kind == "ffmetadata" {
            marks.extend(parse_ffmeta_list(&text, path)?);
        } else if kind == "srt" {
            // transcript → chapters: every cue start becomes a mark titled
            // by the cue's first line (auto-chapter a subtitle track for
            // player scrubbing)
            for c in crate::srt::parse_srt(&text)
                .map_err(|e| Error::input(format!("--import {}: {e}", path.display())))?
            {
                let title = c.text.lines().next().unwrap_or("").trim().to_string();
                if !title.is_empty() {
                    marks.push((c.start, title));
                }
            }
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
    if let Some(n) = args.spread {
        if n == 0 {
            return Err(Error::input("--spread needs N >= 1"));
        }
        let titles: Vec<String> = match &args.titles {
            Some(raw) => {
                let t: Vec<String> = raw.split(',').map(|s| s.trim().to_string()).collect();
                if t.len() != n as usize {
                    return Err(Error::input(format!(
                        "--titles gives {} name(s) but --spread wants {n}",
                        t.len()
                    )));
                }
                if t.iter().any(|s| s.is_empty()) {
                    return Err(Error::input("--titles has an empty entry"));
                }
                t
            }
            None => (1..=n).map(|i| format!("Chapter {i}")).collect(),
        };
        // even grid: mark i lands at duration*i/N — first at 0, none past the end
        for (i, ti) in titles.into_iter().enumerate() {
            marks.push((probe.duration * i as f64 / n as f64, ti));
        }
    }
    if args.titles.is_some() && args.spread.is_none() {
        return Err(Error::input("chapter --titles needs --spread"));
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
    if let Some(rate) = args.rate {
        if rate <= 0.0 {
            return Err(Error::input("chapter --rate must be positive"));
        }
        for m in &mut marks {
            m.0 *= rate;
        }
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
    let mut min_gap_dropped = 0usize;
    if let Some(gap) = args.min_gap {
        if !gap.is_finite() || gap < 0.0 {
            return Err(Error::input(
                "chapter --min-gap needs a non-negative duration",
            ));
        }
        let mut kept: Vec<(f64, String)> = Vec::with_capacity(marks.len());
        for m in marks {
            if kept.last().map(|(t, _)| m.0 - t < gap).unwrap_or(false) {
                min_gap_dropped += 1;
            } else {
                kept.push(m);
            }
        }
        marks = kept;
    }

    let mut snapped = 0usize;
    if args.snap {
        let (_count, keys) = crate::probe::keyframe_packets(&args.input, g.timeout);
        if keys.is_empty() {
            return Err(Error::input(
                "chapter --snap: no keyframes found (needs a video stream)",
            ));
        }
        let times: Vec<f64> = keys.iter().map(|(_, t)| *t).collect();
        for m in &mut marks {
            let nearest = times
                .iter()
                .min_by(|a, b| {
                    (*a - m.0)
                        .abs()
                        .partial_cmp(&(*b - m.0).abs())
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .copied();
            if let Some(t) = nearest {
                if (t - m.0).abs() > 1e-9 {
                    m.0 = t;
                    snapped += 1;
                }
            }
        }
        marks.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        marks.dedup_by(|a, b| (a.0 - b.0).abs() < 0.05);
    }
    if marks.is_empty() {
        return Err(Error::input(
            "chapter needs --at TIME|TITLE, or the detector (--auto/--scenes) found no marks",
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
    if args.export
        || args.yt
        || args.cue
        || args.podcast
        || args.lrc
        || args.vtt
        || args.csv
        || args.srt
        || args.edl
        || args.fcpxml
    {
        let text = if args.csv {
            // Resolve/Premiere marker + spreadsheet exchange: H:MM:SS.mmm,Title
            marks
                .iter()
                .map(|(t, ti)| {
                    let ms = (*t * 1000.0).round() as u64;
                    let title = if ti.contains(',') || ti.contains('"') {
                        format!("\"{}\"", ti.replace('"', "\"\""))
                    } else {
                        ti.clone()
                    };
                    format!(
                        "{:02}:{:02}:{:02}.{:03},{title}",
                        ms / 3_600_000,
                        (ms / 60_000) % 60,
                        (ms / 1000) % 60,
                        ms % 1000,
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
                + "\n"
        } else if args.srt {
            // Chapter TOC as soft subtitles: each mark is a cue titled by
            // the chapter, spanning to the next mark / EOF — burn or mux
            // it to preview where seek points land
            let cues: Vec<crate::srt::Cue> = marks
                .iter()
                .enumerate()
                .map(|(i, (t, ti))| crate::srt::Cue {
                    start: *t,
                    end: if i + 1 < marks.len() {
                        marks[i + 1].0
                    } else {
                        probe.duration
                    },
                    text: ti.clone(),
                })
                .collect();
            crate::srt::to_srt(&cues)
        } else if args.edl {
            // CMX-style EDL — Resolve/Premiere import each event as a
            // timeline marker; spans use the clip's frame rate
            let fps = probe.fps.unwrap_or(25.0).max(1.0).round();
            let tc = |t: f64| -> String {
                let fr = (t * fps).round() as u64;
                format!(
                    "{:02}:{:02}:{:02}:{:02}",
                    fr / (fps as u64 * 3600),
                    (fr / (fps as u64 * 60)) % 60,
                    (fr / fps as u64) % 60,
                    fr % fps as u64,
                )
            };
            let mut s = String::from(
                "TITLE: ffkit chapter marks
FCM: NON-DROP FRAME

",
            );
            for (i, (t, ti)) in marks.iter().enumerate() {
                let end = if i + 1 < marks.len() {
                    marks[i + 1].0
                } else {
                    probe.duration
                };
                s.push_str(&format!(
                    "{:03}  AX       V     C        {} {} {} {}
* FROM CLIP NAME: {}
",
                    i + 1,
                    tc(*t),
                    tc(end),
                    tc(*t),
                    tc(end),
                    ti,
                ));
            }
            s
        } else if args.vtt {
            // WebVTT chapters file — <track kind="chapters"> on a web
            // <video> gives click-to-seek nav without an editor timeline
            let mut s = String::from("WEBVTT\n\n");
            for (i, (t, ti)) in marks.iter().enumerate() {
                let end = if i + 1 < marks.len() {
                    marks[i + 1].0
                } else {
                    probe.duration
                };
                s.push_str(&format!("{} --> {}\n{}\n\n", vtt_ts(*t), vtt_ts(end), ti));
            }
            s
        } else if args.fcpxml {
            // Final Cut Pro XML — FCP/Resolve import each <marker> as a
            // timeline marker; chapter TOC exchange that survives the
            // editor round-trip (unlike ffmetadata which is ffmpeg-only)
            let esc = |v: &str| -> String {
                v.replace('&', "&amp;")
                    .replace('<', "&lt;")
                    .replace('>', "&gt;")
                    .replace('"', "&quot;")
            };
            let fps = probe.fps.unwrap_or(25.0).max(1.0).round() as u64;
            let stem = args
                .input
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("clip");
            let mut s = format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE fcpxml>\n<fcpxml version=\"1.8\">\n<resources>\n<format id=\"r1\"/>\n<asset id=\"r2\" name=\"{stem}\" src=\"file://{}\" start=\"0s\" duration=\"{}s\" hasVideo=\"1\" format=\"r1\"/>\n</resources>\n<library>\n<event name=\"ffkit chapters\">\n<project name=\"{stem}\">\n<sequence format=\"r1\" duration=\"{}s\">\n<spine>\n<asset-clip name=\"{stem}\" ref=\"r2\" offset=\"0s\" duration=\"{}s\">\n",
                esc(&args.input.display().to_string()),
                probe.duration,
                probe.duration,
                probe.duration,
            );
            for (t, ti) in &marks {
                s.push_str(&format!(
                    "<marker start=\"{}s\" duration=\"1/{fps}s\" value=\"{}\"/>\n",
                    t,
                    esc(ti)
                ));
            }
            s.push_str(
                "</asset-clip>\n</spine>\n</sequence>\n</project>\n</event>\n</library>\n</fcpxml>\n",
            );
            s
        } else if args.lrc {
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
            "exported": if args.csv { "csv" } else if args.vtt { "vtt" } else if args.lrc { "lrc" } else if args.podcast { "podcast" } else if args.yt { "youtube" } else if args.cue { "cue" } else if args.srt { "srt" } else if args.edl { "edl" } else if args.fcpxml { "fcpxml" } else { "ffmetadata" },
            "chapters": marks
                .iter()
                .map(|(t, ti)| json!({"time": t, "title": ti}))
                .collect::<Vec<_>>(),
            "min_gap_dropped": min_gap_dropped,
        "snapped": snapped,
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
        "min_gap_dropped": min_gap_dropped,
        "snapped": snapped,
    });
    if matches!(c.status, Status::Ok) {
        if let Ok(p) = engine::probe_or_err(&args.output, g) {
            extra["probe"] = json!(p);
        }
    }
    Ok(c.with_extra(extra))
}

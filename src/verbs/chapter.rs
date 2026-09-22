use serde_json::json;

use crate::cli::{ChapterArgs, Globals};
use crate::contract::{Contract, Status};
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: ChapterArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;

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

    let mut meta = String::from(";FFMETADATA1\n");
    for (i, (t, title)) in marks.iter().enumerate() {
        let end = if i + 1 < marks.len() {
            marks[i + 1].0
        } else {
            probe.duration
        };
        meta.push_str(&format!(
            "[CHAPTER]\nTIMEBASE=1/1000\nSTART={}\nEND={}\ntitle={}\n",
            (t * 1000.0).round() as i64,
            (end * 1000.0).round() as i64,
            title.replace('=', ";").replace('\n', " "),
        ));
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

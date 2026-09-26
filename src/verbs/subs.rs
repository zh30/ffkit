use serde_json::json;

use crate::cli::{Globals, SubsArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::srt::Cue;

/// Read a subtitle file as text. `--encoding` decodes legacy charsets
/// (gbk/big5/sjis/latin1) via encoding_rs; without it the file must be UTF-8.
fn read_sub_file(p: &std::path::Path, enc: Option<&str>) -> Result<String, Error> {
    match enc {
        None => std::fs::read_to_string(p)
            .map_err(|e| Error::input(format!("read {}: {e}", p.display()))),
        Some(label) => {
            let bytes =
                std::fs::read(p).map_err(|e| Error::input(format!("read {}: {e}", p.display())))?;
            let codec = encoding_rs::Encoding::for_label(label.as_bytes()).ok_or_else(|| {
                Error::input(format!(
                    "unknown --encoding '{label}' (try utf-8, gbk, big5, sjis, latin1)"
                ))
            })?;
            Ok(codec.decode(&bytes).0.into_owned())
        }
    }
}

pub fn run(args: SubsArgs, g: &Globals) -> Result<Contract, Error> {
    if args.all {
        return extract_all(&args, g);
    }
    if args.find.is_some() {
        for (flag, set) in [
            ("convert", args.convert),
            ("append", args.append.is_some()),
            ("split", args.split.is_some()),
            ("resync", args.resync.is_some()),
            ("shift", args.shift.is_some()),
            ("merge", args.merge.is_some()),
            ("rate", args.rate.is_some()),
            ("burn", args.burn.is_some()),
            ("burn-si", args.burn_si.is_some()),
            ("mux", args.mux.is_some()),
            ("all", args.all),
        ] {
            if set {
                return Err(Error::input(format!(
                    "subs --find composes with the tidy flags only — --{flag} runs on its own pass"
                )));
            }
        }
    }
    if args.convert {
        return convert(&args, g);
    }
    if args.append.is_some() {
        return append_sub(&args, g);
    }
    if args.split.is_some() {
        return split_sub(&args, g);
    }
    if args.resync.is_some() {
        return resync_sub(&args, g);
    }
    if let Some(offset) = args.shift {
        return shift(&args, offset, g);
    }
    if let Some(other) = &args.merge {
        return merge(&args, other, g);
    }
    if let Some(rate) = args.rate {
        return rescale(&args, rate, g);
    }
    if args.sort
        || args.fix_overlaps
        || args.dedupe
        || args.dedupe_text
        || args.fix_cps.is_some()
        || args.fix_lines.is_some()
        || args.cps.is_some()
        || args.min_dur.is_some()
        || args.min_gap.is_some()
        || args.join.is_some()
        || args.max_lines.is_some()
        || args.replace.is_some()
        || args.strip_speakers
        || args.speakers
        || args.stats
        || args.strip_tags
        || args.strip_sdh
        || args.strip_emotes
        || args.rtl
        || args.clip.is_some()
        || args.drop.is_some()
        || args.wrap.is_some()
        || args.find.is_some()
        || args.cue_move.is_some()
        || args.snap
    {
        return tidy(&args, g);
    }
    if args.case.is_some() && args.burn.is_none() {
        return Err(Error::input("subs --case works with --burn or --convert"));
    }
    let _probe = engine::probe_or_err(&args.input, g)?;
    if let Some(si) = args.burn_si {
        let n_subs = _probe.subtitle_streams;
        if n_subs <= si {
            return Err(Error::input(format!(
                "--burn-si {si}: input only has {n_subs} subtitle stream(s)"
            )));
        }
        return burn(&args, &args.input.clone(), g, Some(si));
    }
    if let Some(subs) = &args.burn {
        return burn(&args, subs, g, None);
    }
    if args.burn_box {
        return Err(Error::input("subs --box needs --burn"));
    }
    if args.align.is_some() {
        return Err(Error::input("subs --align needs --burn"));
    }
    if let Some(subs) = &args.mux {
        return mux(&args, subs, g);
    }
    let codec = match args
        .output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
    {
        "srt" => "srt",
        "vtt" => "webvtt",
        "ass" | "ssa" => "ass",
        ext => {
            return Err(Error::input(format!(
                "subs output must be .srt/.vtt/.ass, got .{ext}"
            )))
        }
    };
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-map", &format!("0:s:{}", args.stream)]);
    argv.extend(["-c:s", codec]);
    argv.push(&args.output);

    let c = engine::write_job("subs", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "stream": args.stream,
        "codec": codec,
    })))
}

/// Extract every subtitle stream in one ffmpeg call: `stem_0.ext`,
/// `stem_1.ext` … in the `-o` extension's codec.
fn extract_all(args: &SubsArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    if probe.subtitle_streams == 0 {
        return Err(Error::input("subs --all: input has no subtitle streams"));
    }
    let codec = match args
        .output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
    {
        "srt" => "srt",
        "vtt" => "webvtt",
        "ass" | "ssa" => "ass",
        ext => {
            return Err(Error::input(format!(
                "subs output must be .srt/.vtt/.ass, got .{ext}"
            )))
        }
    };
    let stem = args
        .output
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("subs");
    let ext = args.output.extension().and_then(|e| e.to_str()).unwrap();
    let parent = args.output.parent().filter(|p| !p.as_os_str().is_empty());
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    // Per-stream option groups: -map/-c:s/output interleaved so each -o
    // holds exactly one subtitle (srt muxer rejects >1 sub stream).
    let mut files = Vec::new();
    for k in 0..probe.subtitle_streams {
        let name = format!("{stem}_{k}.{ext}");
        let path = match parent {
            Some(d) => d.join(name).display().to_string(),
            None => name,
        };
        argv.extend(["-map", &format!("0:s:{k}"), "-c:s", codec, &path]);
        files.push(path);
    }
    let c = engine::write_job(
        "subs",
        &[&args.input],
        std::path::Path::new(&files[0]),
        vec![argv],
        g,
    )?;
    Ok(c.with_extra(json!({
        "streams": probe.subtitle_streams,
        "files": files,
    })))
}

/// Merge two .srt files into one: cues from both, sorted by start, renumbered.
fn merge(args: &SubsArgs, other: &std::path::Path, g: &Globals) -> Result<Contract, Error> {
    for p in [&args.input, other] {
        if p.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .as_deref()
            != Some("srt")
        {
            return Err(Error::input("subs --merge combines two .srt files"));
        }
    }
    if !other.is_file() {
        return Err(Error::input(format!(
            "no such subtitle file: {}",
            other.display()
        )));
    }
    let enc = args.encoding.as_deref();
    let read = |p: &std::path::Path| -> Result<Vec<crate::srt::Cue>, Error> {
        let raw = read_sub_file(p, enc)?;
        crate::srt::parse_srt(&raw)
    };
    let mut cues = read(&args.input)?;
    let added = read(other)?.len();
    cues.extend(read(other)?);
    cues.sort_by(|a, b| {
        a.start
            .partial_cmp(&b.start)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let out = crate::srt::to_srt(&cues);
    if g.dry_run {
        return Ok(Contract::dry_run(
            "subs",
            Some(crate::paths::display(&args.output)),
            None,
        ));
    }
    std::fs::write(&args.output, out).map_err(|e| Error::output(e.to_string()))?;
    let mut c = Contract::ok("subs", Some(crate::paths::display(&args.output)), None);
    c.verified = Some(args.output.is_file());
    Ok(c.with_extra(json!({
        "mode": "merge",
        "added_cues": added,
        "cues": cues.len(),
    })))
}

/// Cue-file hygiene: `--sort` reorders by start, `--fix-overlaps` clamps
/// each end to the next start, `--dedupe` drops exact repeats. All read
/// the input .srt and write a clean .srt — run --sort first when times
/// are scrambled (the clamp only makes sense in start order).
fn tidy(args: &SubsArgs, g: &Globals) -> Result<Contract, Error> {
    if args
        .input
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .as_deref()
        != Some("srt")
    {
        return Err(Error::input(
            "subs tidy flags (--sort/--fix-overlaps/--dedupe/--dedupe-text/--fix-cps/--cps/--min-dur/--min-gap/--join/--max-lines/--replace/--strip-speakers/--strip-sdh/--strip-emotes/--clip/--drop/--wrap/--snap) take an .srt input",
        ));
    }
    let raw = read_sub_file(&args.input, args.encoding.as_deref())?;
    let mut cues = crate::srt::parse_srt(&raw)?;
    let mut moved = 0usize;
    // --move N,T: re-seat one cue — runs first so N is the input file's
    // numbering before any filter drops/reorders cues
    if let Some(raw) = &args.cue_move {
        let (n_raw, t_raw) = raw
            .split_once(',')
            .ok_or_else(|| Error::input("subs --move needs N,T (e.g. 3,1.5)"))?;
        let n: usize = n_raw
            .trim()
            .parse()
            .map_err(|_| Error::input("subs --move: cue index must be a number"))?;
        if n == 0 || n > cues.len() {
            return Err(Error::input(format!(
                "subs --move: cue index {n} out of range 1..={}",
                cues.len()
            )));
        }
        let t = crate::time::parse_time(t_raw)?;
        let c = &mut cues[n - 1];
        c.end = t + (c.end - c.start);
        c.start = t;
        moved = 1;
    }
    let mut dropped = 0usize;
    // --clip F,T: keep cues overlapping the window, clamp edges, re-time
    // to 0 — runs first so every other tidy op sees the clipped set.
    let mut clipped = 0usize;
    if let Some(raw) = &args.clip {
        let (f_raw, t_raw) = raw
            .split_once(',')
            .ok_or_else(|| Error::input("subs --clip needs F,T (e.g. 1.5,end)"))?;
        let f = crate::time::parse_time(f_raw)?;
        let t = if t_raw.trim().eq_ignore_ascii_case("end") {
            cues.iter().map(|c| c.end).fold(0.0, f64::max)
        } else {
            crate::time::parse_time(t_raw)?
        };
        if t <= f {
            return Err(Error::input("subs --clip: window end must be after start"));
        }
        let before = cues.len();
        cues.retain(|c| c.end > f && c.start < t);
        dropped += before - cues.len();
        if cues.is_empty() {
            return Err(Error::input(format!(
                "subs --clip {f:.3},{t:.3}: window holds no cues"
            )));
        }
        for c in &mut cues {
            c.start = (c.start - f).max(0.0);
            c.end = (c.end - f).min(t - f);
            clipped += 1;
        }
    }
    let mut excised = 0usize;
    if let Some(raw) = &args.drop {
        let (f_raw, t_raw) = raw
            .split_once(',')
            .ok_or_else(|| Error::input("subs --drop needs F,T (e.g. 1.5,end)"))?;
        let f = crate::time::parse_time(f_raw)?;
        let t = if t_raw.trim().eq_ignore_ascii_case("end") {
            cues.iter().map(|c| c.end).fold(0.0, f64::max)
        } else {
            crate::time::parse_time(t_raw)?
        };
        if t <= f {
            return Err(Error::input("subs --drop: window end must be after start"));
        }
        for c in &mut cues {
            if c.end <= f || c.start >= t {
                continue;
            }
            if c.start < f && c.end > t {
                c.end = f; // spans the whole cut — keep the head
            } else if c.end > t {
                c.start = t; // tail piece — the shift pass lands it at f
            } else if c.end > f {
                c.end = f; // head piece
            }
        }
        let before = cues.len();
        cues.retain(|c| !(c.start >= f && c.end <= t));
        excised += before - cues.len();
        if cues.is_empty() {
            return Err(Error::input(format!(
                "subs --drop {f:.3},{t:.3}: window excises every cue"
            )));
        }
        let shift = t - f;
        for c in &mut cues {
            if c.start >= t {
                c.start -= shift;
                c.end -= shift;
            }
        }
    }
    let sorted = args.sort && {
        let before: Vec<f64> = cues.iter().map(|c| c.start).collect();
        cues.sort_by(|a, b| {
            a.start
                .partial_cmp(&b.start)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        before != cues.iter().map(|c| c.start).collect::<Vec<_>>()
    };
    let mut clamped = 0usize;
    if args.fix_overlaps {
        for i in 0..cues.len() {
            if i + 1 < cues.len() && cues[i].end > cues[i + 1].start {
                cues[i].end = cues[i + 1].start;
                clamped += 1;
            }
        }
        let before = cues.len();
        cues.retain(|c| c.end > c.start);
        dropped += before - cues.len();
    }
    if args.dedupe {
        let before = cues.len();
        cues.dedup_by(|a, b| a.start == b.start && a.end == b.end && a.text == b.text);
        dropped += before - cues.len();
    }
    if args.dedupe_text {
        let norm = |s: &str| {
            s.split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .to_lowercase()
        };
        let before = cues.len();
        let mut out: Vec<crate::srt::Cue> = Vec::with_capacity(cues.len());
        for c in cues {
            if out
                .last()
                .map(|p: &crate::srt::Cue| norm(&p.text) == norm(&c.text))
                .unwrap_or(false)
            {
                continue;
            }
            out.push(c);
        }
        dropped += before - out.len();
        cues = out;
    }
    let mut joined = 0usize;
    if let Some(gap) = args.join {
        let mut i = 0;
        while i + 1 < cues.len() {
            if cues[i + 1].start - cues[i].end < gap {
                cues[i].end = cues[i + 1].end;
                cues[i].text = format!("{} {}", cues[i].text.trim(), cues[i + 1].text.trim());
                cues.remove(i + 1);
                joined += 1;
            } else {
                i += 1;
            }
        }
    }
    let mut extended = 0usize;
    if let Some(min) = args.min_dur {
        for i in 0..cues.len() {
            let next_start = cues.get(i + 1).map(|c| c.start);
            let c = &mut cues[i];
            if c.end - c.start < min {
                let cap = next_start.unwrap_or(c.start + min).min(c.start + min);
                if cap > c.end {
                    c.end = cap;
                    extended += 1;
                }
            }
        }
    }
    let mut gapped = 0usize;
    if let Some(gap) = args.min_gap {
        for i in 0..cues.len().saturating_sub(1) {
            let next_start = cues[i + 1].start;
            let c = &mut cues[i];
            if c.end > next_start - gap {
                c.end = (next_start - gap).max(c.start);
                gapped += 1;
            }
        }
        let before = cues.len();
        cues.retain(|c| c.end > c.start);
        dropped += before - cues.len();
    }
    // text transforms — after timing ops so counts reflect the final cues
    let (mut replaced, mut stripped, mut rewrapped) = (0usize, 0usize, 0usize);
    if let Some(pair) = &args.replace {
        let (old, new) = pair
            .split_once(',')
            .ok_or_else(|| Error::input("subs --replace wants OLD,NEW"))?;
        for c in &mut cues {
            let t = c.text.replace(old, new);
            if t != c.text {
                c.text = t;
                replaced += 1;
            }
        }
    }
    // shared speaker-label detection: [NAME] / <NAME> bracketed labels
    // or ALL-CAPS `NAME:` labels — used by --speakers and --strip-speakers
    fn speaker_label(l: &str) -> Option<&str> {
        let l = l.trim_start();
        let bracket = if l.starts_with('[') {
            l.find(']')
        } else if l.starts_with('<') {
            l.find('>')
        } else {
            None
        };
        if let Some(end) = bracket {
            if end <= 32 {
                return Some(&l[..=end]);
            }
        }
        if let Some(colon) = l.find(':') {
            let head = &l[..colon];
            if colon <= 30
                && !head.is_empty()
                && head.chars().all(|ch| {
                    ch.is_ascii_uppercase() || ch.is_ascii_digit() || " .'_-".contains(ch)
                })
                && head.chars().any(|ch| ch.is_ascii_uppercase())
            {
                return Some(&l[..=colon]);
            }
        }
        None
    }

    if args.strip_speakers {
        for c in &mut cues {
            let t = c
                .text
                .lines()
                .map(|l| {
                    let l = l.trim_start();
                    if let Some(label) = speaker_label(l) {
                        return l[label.len()..].trim_start();
                    }
                    l
                })
                .collect::<Vec<_>>()
                .join("\n");
            if t != c.text {
                c.text = t;
                stripped += 1;
            }
        }
    }
    if let Some(n) = args.wrap {
        for c in &mut cues {
            let mut out = String::new();
            let mut col = 0usize;
            for w in c.text.split_whitespace() {
                let wl = w.chars().count();
                if col > 0 && col + 1 + wl > n {
                    out.push('\n');
                    col = 0;
                } else if col > 0 {
                    out.push(' ');
                    col += 1;
                }
                out.push_str(w);
                col += wl;
            }
            if out != c.text {
                c.text = out;
                rewrapped += 1;
            }
        }
    }
    let mut sdh_stripped = 0usize;
    if args.strip_sdh {
        for c in &mut cues {
            let out = crate::srt::strip_sdh(&c.text);
            if out != c.text {
                c.text = out;
                sdh_stripped += 1;
            }
        }
        let before = cues.len();
        cues.retain(|c| !c.text.trim().is_empty());
        dropped += before - cues.len();
    }
    let mut emotes_stripped = 0usize;
    if args.strip_emotes {
        for c in &mut cues {
            let out = crate::srt::strip_emotes(&c.text);
            if out != c.text {
                c.text = out;
                emotes_stripped += 1;
            }
        }
        let before = cues.len();
        cues.retain(|c| !c.text.trim().is_empty());
        dropped += before - cues.len();
    }
    let mut rtl_wrapped = 0usize;
    if args.rtl {
        // Arabic/Hebrew captions render mirrored punctuation in players
        // without bidirectional marks — wrap every cue-text LINE in
        // RLE..PDF so each line resolves RTL (marks must be per-line, not
        // per-cue: a multi-line cue resets direction at each break)
        for c in &mut cues {
            let out = c
                .text
                .lines()
                .map(|l| format!("\u{202b}{l}\u{202c}"))
                .collect::<Vec<_>>()
                .join("\n");
            if out != c.text {
                c.text = out;
                rtl_wrapped += 1;
            }
        }
    }
    let mut tags_stripped = 0usize;
    if args.strip_tags {
        for c in &mut cues {
            let out = crate::srt::strip_markup(&c.text);
            if out != c.text {
                c.text = out;
                tags_stripped += 1;
            }
        }
    }
    let mut found = 0usize;
    if let Some(needle) = &args.find {
        let n = needle.to_lowercase();
        let before = cues.len();
        cues.retain(|c| c.text.to_lowercase().contains(&n));
        found = cues.len();
        dropped += before - cues.len();
        if found == 0 {
            return Err(Error::input(format!(
                "subs --find '{needle}': no cue contains that text"
            )));
        }
    }
    if cues.is_empty() {
        return Err(Error::input("tidying removed every cue"));
    }
    if g.dry_run {
        return Ok(Contract::dry_run(
            "subs",
            Some(crate::paths::display(&args.output)),
            None,
        ));
    }
    let mut stat_extra = serde_json::Map::new();
    if args.stats {
        let words: usize = cues.iter().flat_map(|c| c.text.split_whitespace()).count();
        let chars: usize = cues.iter().map(|c| c.text.chars().count()).sum();
        let span = if cues.is_empty() {
            0.0
        } else {
            cues.iter().map(|c| c.end).fold(0.0f64, f64::max)
                - cues.iter().map(|c| c.start).fold(f64::MAX, f64::min)
        };
        let mut durs: Vec<f64> = cues.iter().map(|c| c.end - c.start).collect();
        durs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let median = if durs.is_empty() {
            0.0
        } else {
            durs[durs.len() / 2]
        };
        stat_extra.insert("cues".to_string(), json!(cues.len()));
        stat_extra.insert("words".to_string(), json!(words));
        stat_extra.insert("chars".to_string(), json!(chars));
        stat_extra.insert("span_secs".to_string(), json!(span));
        stat_extra.insert("median_dur_secs".to_string(), json!(median));
    }
    let mut speakers: Vec<String> = Vec::new();
    if args.speakers {
        for c in &cues {
            for l in c.text.lines() {
                if let Some(label) = speaker_label(l) {
                    let label = label
                        .trim_start_matches(['[', '<'])
                        .trim_end_matches([']', '>', ':']);
                    if !speakers.iter().any(|s| s == label) {
                        speakers.push(label.to_string());
                    }
                }
            }
        }
    }
    let mut lines_split = 0usize;
    if let Some(n) = args.fix_lines {
        if n == 0 {
            return Err(Error::input("subs --fix-lines needs a positive line count"));
        }
        let n = n as usize;
        let mut out: Vec<Cue> = Vec::with_capacity(cues.len());
        for cue in cues {
            let lines: Vec<&str> = cue.text.lines().collect();
            if lines.len() <= n {
                out.push(cue);
                continue;
            }
            let chunks: Vec<&[&str]> = lines.chunks(n).collect();
            let span = cue.end - cue.start;
            for (i, ch) in chunks.iter().enumerate() {
                out.push(Cue {
                    start: cue.start + span * i as f64 / chunks.len() as f64,
                    end: cue.start + span * (i + 1) as f64 / chunks.len() as f64,
                    text: ch.join("\n"),
                });
            }
            lines_split += 1;
        }
        cues = out;
    }
    let mut stretched = 0usize;
    if let Some(cps) = args.fix_cps {
        if !cps.is_finite() || cps <= 0.0 {
            return Err(Error::input(
                "subs --fix-cps needs a positive chars/sec rate",
            ));
        }
        for i in 0..cues.len() {
            let dur = (cues[i].end - cues[i].start).max(0.001);
            let need = cues[i].text.chars().count() as f64 / cps;
            if need > dur {
                let cap = cues.get(i + 1).map(|n| n.start).unwrap_or(f64::INFINITY);
                cues[i].end = (cues[i].start + need).min(cap).max(cues[i].start);
                stretched += 1;
            }
        }
    }

    let mut snapped = 0usize;
    if args.snap {
        let fps = args.fps.unwrap_or(25.0);
        if !(1.0..=240.0).contains(&fps) {
            return Err(Error::input("subs --snap needs --fps in 1-240"));
        }
        let frame = 1.0 / fps;
        for c in &mut cues {
            let s = (c.start * fps).round() / fps;
            let e = ((c.end * fps).round() / fps).max(s + frame);
            if (s - c.start).abs() > 1e-9 || (e - c.end).abs() > 1e-9 {
                snapped += 1;
            }
            c.start = s;
            c.end = e;
        }
    }

    std::fs::write(&args.output, crate::srt::to_srt(&cues))
        .map_err(|e| Error::output(e.to_string()))?;
    let mut c = Contract::ok("subs", Some(crate::paths::display(&args.output)), None);
    c.verified = Some(args.output.is_file());
    let (mut over_limit, mut worst_cps) = (0usize, 0.0f64);
    let (mut over_lines, mut worst_lines) = (0usize, 0usize);
    for cue in &cues {
        if let Some(limit) = args.cps {
            let dur = (cue.end - cue.start).max(0.001);
            let v = cue.text.chars().count() as f64 / dur;
            worst_cps = worst_cps.max(v);
            if v > limit {
                over_limit += 1;
            }
        }
        if let Some(n) = args.max_lines {
            let lines = cue.text.lines().count().max(1);
            worst_lines = worst_lines.max(lines);
            if lines > n {
                over_lines += 1;
            }
        }
    }
    let mut extra = json!({
        "mode": "tidy",
        "sorted": sorted,
        "clamped": clamped,
        "dropped": dropped,
        "extended": extended,
        "gapped": gapped,
        "joined": joined,
        "replaced": replaced,
        "stripped": stripped,
        "tags_stripped": tags_stripped,
        "sdh_stripped": sdh_stripped,
        "emotes_stripped": emotes_stripped,
        "rtl_wrapped": rtl_wrapped,
        "clipped": clipped,
        "excised": excised,
        "moved": moved,
        "rewrapped": rewrapped,
        "find": args.find,
        "found": found,
        "cues": cues.len(),
        "cps_limit": args.cps,
        "stretched": stretched,
        "lines_split": lines_split,
        "snapped": snapped,
        "speakers": speakers,
        "speaker_count": speakers.len(),
        "min_gap": args.min_gap,
        "over_limit": over_limit,
        "worst_cps": (worst_cps * 100.0).round() / 100.0,
        "over_lines": over_lines,
        "worst_lines": worst_lines,
    });
    if let serde_json::Value::Object(m) = &mut extra {
        m.extend(stat_extra);
    }
    Ok(c.with_extra(extra))
}

/// Mux an .srt/.vtt/.ass into the container as a selectable subtitle stream
/// (mov_text for mp4, srt for mkv) with an optional `language=` tag.
fn mux(args: &SubsArgs, subs: &std::path::Path, g: &Globals) -> Result<Contract, Error> {
    if !subs.is_file() {
        return Err(Error::input(format!(
            "no such subtitle file: {}",
            subs.display()
        )));
    }
    let codec = match args
        .output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
    {
        "mkv" | "webm" => "srt",
        _ => "mov_text",
    };
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.push("-i");
    argv.push(subs);
    argv.extend([
        "-map", "0:v?", "-map", "0:a?", "-map", "1", "-c:v", "copy", "-c:a", "copy", "-c:s", codec,
    ]);
    if let Some(lang) = &args.lang {
        argv.extend(["-metadata:s:s:0", &format!("language={lang}")]);
    }
    argv.push(&args.output);
    let c = engine::write_job("subs", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "mode": "mux",
        "codec": codec,
        "lang": args.lang,
    })))
}

fn burn(
    args: &SubsArgs,
    subs: &std::path::Path,
    g: &Globals,
    stream_index: Option<u32>,
) -> Result<Contract, Error> {
    if !subs.is_file() {
        return Err(Error::input(format!(
            "no such subtitle file: {}",
            subs.display()
        )));
    }
    // --case/--from/--to: rewrite cues into a temp .srt before burning.
    let is_srt = subs
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("srt"))
        .unwrap_or(false);
    let win_from = match &args.from {
        Some(t) if t.trim().eq_ignore_ascii_case("end") => {
            Some(engine::probe_or_err(&args.input, g)?.duration)
        }
        Some(t) if t.trim().to_ascii_lowercase().starts_with("end-") => Some(
            engine::probe_or_err(&args.input, g)?.duration
                - crate::time::parse_time(&t.trim()[4..])?,
        ),
        Some(t) => Some(crate::time::parse_time(t)?),
        None => None,
    };
    let win_to = match &args.to {
        Some(t) if t.trim().eq_ignore_ascii_case("end") => {
            Some(engine::probe_or_err(&args.input, g)?.duration)
        }
        Some(t) => Some(crate::time::parse_time(t)?),
        None => None,
    };
    if win_to.is_some() && win_from.is_none() {
        return Err(Error::input("subs --to needs --from"));
    }
    if args.from.is_some() && !is_srt {
        return Err(Error::input("subs --from/--to filter needs an .srt file"));
    }
    let cased;
    let subs = if args.case.is_some() || args.from.is_some() {
        if args.case.is_some() && !is_srt {
            subs.to_path_buf()
        } else {
            let raw = read_sub_file(subs, args.encoding.as_deref())?;
            let mut cues = crate::srt::parse_srt(&raw)?;
            if let Some(case) = args.case {
                apply_case(&mut cues, case);
            }
            if let Some(f) = win_from {
                let t = win_to.unwrap_or(f64::MAX);
                cues.retain(|c| c.end > f && c.start < t);
            }
            cased = tempfile::Builder::new()
                .suffix(".srt")
                .tempfile()
                .map_err(|e| Error::output(e.to_string()))?;
            std::fs::write(cased.path(), crate::srt::to_srt(&cues))
                .map_err(|e| Error::output(format!("write cased srt: {e}")))?;
            cased.path().to_path_buf()
        }
    } else {
        subs.to_path_buf()
    };
    // The subtitles filter parses `:` `'` `,` etc. in filenames — escape them.
    let path = subs
        .canonicalize()
        .map_err(|e| Error::input(format!("{subs:?}: {e}")))?
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace(':', "\\:")
        .replace(',', "\\,")
        .replace('[', "\\[")
        .replace(']', "\\]");
    let size = args.size.unwrap_or(18.0).clamp(6.0, 96.0);
    // ASS hex is &HAABBGGRR — flip the RRGGBB flag arg
    let color = match &args.color {
        Some(c) => {
            let h = c.trim_start_matches('#').trim_start_matches("0x");
            if h.len() == 6 {
                format!("&H00{}{}{}", &h[4..6], &h[2..4], &h[0..2])
            } else {
                return Err(Error::input("--color must be RRGGBB hex"));
            }
        }
        None => "&H00FFFFFF".to_string(),
    };
    let align = match args.align.as_deref() {
        None => {
            if args.top {
                8
            } else {
                2
            }
        }
        Some("left") => {
            if args.top {
                7
            } else {
                1
            }
        }
        Some("center") => {
            if args.top {
                8
            } else {
                2
            }
        }
        Some("right") => {
            if args.top {
                9
            } else {
                3
            }
        }
        Some(a) => {
            return Err(Error::input(format!(
                "subs --align must be left|center|right, got '{a}'"
            )))
        }
    };
    let margin_v = if let Some(m) = args.margin {
        m
    } else if args.safe {
        // social-safe zone: keep burned text off the bottom 20% / top 15%
        let probe = engine::probe_or_err(&args.input, g)?;
        let frac = if args.top { 0.15 } else { 0.20 };
        ((probe.height.unwrap_or(720) as f64) * frac + 36.0) as u32
    } else {
        36
    };
    let font = args.font.as_deref().unwrap_or("Sans").replace(',', " ");
    let outline = args.outline.unwrap_or(1.0).clamp(0.0, 8.0);
    let shadow = args.shadow.unwrap_or(0.0).clamp(0.0, 8.0);
    let (bs, back) = if args.burn_box {
        (3, ",BackColour=&H80000000".to_string())
    } else {
        (1, String::new())
    };
    let style = format!(
        "FontName={font},FontSize={size},PrimaryColour={color},\
OutlineColour=&H80000000,BorderStyle={bs}{back},Outline={outline},Shadow={shadow},\
MarginV={margin_v},Alignment={align}"
    );
    let si = stream_index.map(|i| format!(":si={i}")).unwrap_or_default();
    let vf = format!("subtitles=filename='{path}'{si}:force_style='{style}'");

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-vf", &vf]);
    argv.extend([
        "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
    ]);
    argv.extend(["-c:a", "copy"]);
    argv.push(&args.output);

    let c = engine::write_job("subs", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({ "burned": subs.to_string_lossy() })))
}

fn shift(args: &SubsArgs, offset: f64, g: &Globals) -> Result<Contract, Error> {
    if args
        .input
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .as_deref()
        != Some("srt")
    {
        return Err(Error::input("subs --shift takes an .srt file as input"));
    }
    let raw = read_sub_file(&args.input, args.encoding.as_deref())?;
    let mut cues = crate::srt::parse_srt(&raw)?;
    // --from/--to bounds the retiming to the cues overlapping that window
    // (only part of the track is late, e.g. after an inserted segment).
    let win = match (&args.from, &args.to) {
        (None, None) => None,
        (f, t) => {
            let dur = cues.iter().map(|c| c.end).fold(0.0, f64::max);
            let at = |s: &str| -> Result<f64, Error> {
                if s.trim().eq_ignore_ascii_case("end") {
                    Ok(dur)
                } else {
                    crate::time::parse_time(s)
                }
            };
            let from = match f {
                Some(s) => at(s)?,
                None => 0.0,
            };
            let to = match t {
                Some(s) => at(s)?,
                None => dur,
            };
            if to <= from {
                return Err(Error::input("--from/--to window is empty"));
            }
            Some((from, to))
        }
    };
    for c in cues.iter_mut() {
        if let Some((f, t)) = win {
            if c.end <= f || c.start >= t {
                continue;
            }
        }
        c.start = (c.start + offset).max(0.0);
        c.end = (c.end + offset).max(0.0);
    }
    let out = crate::srt::to_srt(&cues);
    if g.dry_run {
        return Ok(Contract::dry_run(
            "subs",
            Some(crate::paths::display(&args.output)),
            None,
        ));
    }
    std::fs::write(&args.output, out).map_err(|e| Error::output(e.to_string()))?;
    let mut c = Contract::ok("subs", Some(crate::paths::display(&args.output)), None);
    c.verified = Some(args.output.is_file());
    Ok(c.with_extra(json!({
        "shift": offset,
        "cues": cues.len(),
        "window": win.map(|(f, t)| json!({"from": f, "to": t})),
    })))
}

// --resync O1,O2,N1,N2: two-point affine remap — the union of
// --shift (offset) and --rate (scale) in one pass. Subs authored for a
// different cut (or a 25fps master going to 23.976) anchor on two known
// sync points; between/beyond them the line interpolates/extrapolates.
fn resync_sub(args: &SubsArgs, g: &Globals) -> Result<Contract, Error> {
    let raw_spec = args.resync.as_deref().unwrap_or_default();
    let nums: Vec<f64> = raw_spec
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| {
            s.parse()
                .map_err(|_| Error::input("subs --resync wants O1,O2,N1,N2 in seconds"))
        })
        .collect::<Result<_, _>>()?;
    if nums.len() != 4 {
        return Err(Error::input(
            "subs --resync wants O1,O2,N1,N2 — two old times mapped to two new times",
        ));
    }
    let (o1, o2, n1, n2) = (nums[0], nums[1], nums[2], nums[3]);
    if o2 <= o1 {
        return Err(Error::input("subs --resync: O2 must come after O1"));
    }
    if n2 <= n1 {
        return Err(Error::input("subs --resync: N2 must come after N1"));
    }
    let rate = (n2 - n1) / (o2 - o1);
    if args
        .input
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .as_deref()
        != Some("srt")
    {
        return Err(Error::input("subs --resync takes an .srt file as input"));
    }
    let raw = read_sub_file(&args.input, args.encoding.as_deref())?;
    let mut cues = crate::srt::parse_srt(&raw)?;
    for c in cues.iter_mut() {
        c.start = (n1 + (c.start - o1) * rate).max(0.0);
        c.end = (n1 + (c.end - o1) * rate).max(c.start);
    }
    let out = crate::srt::to_srt(&cues);
    if g.dry_run {
        return Ok(Contract::dry_run(
            "subs",
            Some(crate::paths::display(&args.output)),
            None,
        ));
    }
    std::fs::write(&args.output, out).map_err(|e| Error::output(e.to_string()))?;
    let mut c = Contract::ok("subs", Some(crate::paths::display(&args.output)), None);
    c.verified = Some(args.output.is_file());
    Ok(c.with_extra(json!({
        "mode": "resync",
        "old": [o1, o2],
        "new": [n1, n2],
        "rate": rate,
        "cues": cues.len(),
    })))
}

/// Rescale every cue timestamp by a factor — frame-rate drift fixes
/// (25→23.976 ≈ 0.959, 23.976→25 ≈ 1.0427).
fn rescale(args: &SubsArgs, factor: f64, g: &Globals) -> Result<Contract, Error> {
    if !(0.5..=2.0).contains(&factor) {
        return Err(Error::input("--rate must be 0.5..=2.0"));
    }
    if args
        .input
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .as_deref()
        != Some("srt")
    {
        return Err(Error::input("subs --rate takes an .srt file as input"));
    }
    let raw = read_sub_file(&args.input, args.encoding.as_deref())?;
    let mut cues = crate::srt::parse_srt(&raw)?;
    for c in cues.iter_mut() {
        c.start *= factor;
        c.end *= factor;
    }
    let out = crate::srt::to_srt(&cues);
    if g.dry_run {
        return Ok(Contract::dry_run(
            "subs",
            Some(crate::paths::display(&args.output)),
            None,
        ));
    }
    std::fs::write(&args.output, out).map_err(|e| Error::output(e.to_string()))?;
    let mut c = Contract::ok("subs", Some(crate::paths::display(&args.output)), None);
    c.verified = Some(args.output.is_file());
    Ok(c.with_extra(json!({
        "rate": factor,
        "cues": cues.len(),
    })))
}

fn convert(args: &SubsArgs, g: &Globals) -> Result<Contract, Error> {
    let ext = |p: &std::path::Path| {
        p.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .unwrap_or_default()
    };
    let (in_ext, out_ext) = (ext(&args.input), ext(&args.output));
    if in_ext != "srt"
        && in_ext != "vtt"
        && in_ext != "ass"
        && in_ext != "ttml"
        && in_ext != "dfxp"
        && in_ext != "sbv"
        && in_ext != "csv"
        && in_ext != "sub"
        && in_ext != "mpl"
        && in_ext != "smi"
        && in_ext != "scc"
        && in_ext != "stl"
        && in_ext != "rt"
        && in_ext != "mps"
        && in_ext != "pjs"
        && in_ext != "psb"
        && in_ext != "jss"
        && in_ext != "ssa"
    {
        return Err(Error::input(
            "subs --convert takes .srt/.vtt/.ass/.ssa/.ttml/.dfxp/.sbv/.csv/.sub/.mpl/.smi/.scc/.stl/.rt/.mps/.pjs/.psb/.jss input",
        ));
    }
    if out_ext != "srt"
        && out_ext != "vtt"
        && out_ext != "txt"
        && out_ext != "ass"
        && out_ext != "lrc"
        && out_ext != "ttml"
        && out_ext != "dfxp"
        && out_ext != "sbv"
        && out_ext != "csv"
        && out_ext != "mpl"
        && out_ext != "smi"
        && out_ext != "sub"
        && out_ext != "pjs"
        && out_ext != "psb"
        && out_ext != "jss"
        && out_ext != "ssa"
    {
        return Err(Error::input(
            "subs --convert takes .srt/.vtt/.ass/.ssa/.ttml/.dfxp/.sbv/.csv/.sub/.mpl/.smi/.scc/.stl/.rt/.mps/.pjs/.psb/.jss input and .srt/.vtt/.txt/.ass/.ssa/.lrc/.ttml/.dfxp/.sbv/.csv/.mpl/.smi/.sub/.pjs/.psb/.jss output",
        ));
    }
    let raw = if in_ext == "scc"
        || in_ext == "stl"
        || in_ext == "rt"
        || in_ext == "mps"
        || in_ext == "ssa"
    {
        // .scc carries CEA-608 captions as hex pairs, .stl is the Spruce
        // broadcast format, .rt is RealPlayer captions, .mps is MPlayer's
        // start+duration lines, .ssa is SubStation v4 (the pre-ASS
        // dialect — its decoder reads the Marked=0 event fields) — all
        // decode through ffmpeg demuxers; parse the srt each emits
        let mut av = crate::spawn::Argv::ffmpeg();
        av.extend(["-loglevel", "error", "-i"]);
        av.push(&args.input);
        av.extend(["-f", "srt", "-"]);
        let sp = crate::spawn::require_ok(&av, crate::spawn::run(&av, g.timeout, false)?)?;
        crate::spawn::stdout_str(&sp)?.to_string()
    } else {
        read_sub_file(&args.input, args.encoding.as_deref())?
    };
    let mut cues = if in_ext == "ass" {
        parse_ass(&raw)?
    } else if in_ext == "ttml" || in_ext == "dfxp" {
        parse_ttml(&raw)?
    } else if in_ext == "sbv" {
        parse_sbv(&raw)?
    } else if in_ext == "sub" {
        // .sub is ambiguous in the wild — MicroDVD {f}{f} lines vs
        // SubViewer v1/v2. MicroDVD-shaped files keep their own errors
        // (a missing rate still reports --fps); anything else goes
        // through ffmpeg's subviewer demuxer (it reads both v1 and v2)
        let microdvd_shaped = raw
            .lines()
            .map(str::trim)
            .find(|l| !l.is_empty())
            .is_some_and(|l| l.starts_with('{'));
        if microdvd_shaped {
            parse_microdvd(&raw, args.fps)?
        } else {
            let mut av = crate::spawn::Argv::ffmpeg();
            av.extend(["-loglevel", "error", "-f", "subviewer", "-i"]);
            av.push(&args.input);
            av.extend(["-f", "srt", "-"]);
            let sp = crate::spawn::require_ok(&av, crate::spawn::run(&av, g.timeout, false)?)?;
            let srt = crate::spawn::stdout_str(&sp)?.to_string();
            crate::srt::parse_srt(&srt)?
        }
    } else if in_ext == "mpl" {
        parse_mpl2(&raw)?
    } else if in_ext == "smi" {
        parse_sami(&raw)?
    } else if in_ext == "csv" {
        parse_csv_subs(&raw)?
    } else if in_ext == "pjs" {
        parse_pjs(&raw)?
    } else if in_ext == "psb" {
        parse_psb(&raw)?
    } else if in_ext == "jss" {
        parse_jss(&raw)?
    } else {
        // vtt → srt-shaped blocks: drop WEBVTT/NOTE/STYLE blocks and cue
        // settings.
        let body = if in_ext == "vtt" {
            raw.replace("\r\n", "\n")
                .split("\n\n")
                .filter(|b| {
                    let l = b.lines().next().unwrap_or("").trim();
                    !(l.starts_with("WEBVTT") || l.starts_with("NOTE") || l == "STYLE")
                })
                .map(|b| {
                    b.lines()
                        .map(|l| {
                            if l.contains("-->") {
                                if let Some((a, rest)) = l.split_once("-->") {
                                    let e = rest.split_whitespace().next().unwrap_or("");
                                    return format!("{} --> {}", a.trim(), e);
                                }
                            }
                            l.to_string()
                        })
                        .collect::<Vec<_>>()
                        .join("\n")
                })
                .collect::<Vec<_>>()
                .join("\n\n")
        } else {
            raw
        };
        crate::srt::parse_srt(&body)?
    };
    if let Some(case) = args.case {
        apply_case(&mut cues, case);
    }
    let words = cues
        .iter()
        .map(|c| c.text.split_whitespace().count())
        .sum::<usize>();
    let ttml_clock = |t: f64| {
        let h = (t / 3600.0).floor() as u64;
        let m = ((t - h as f64 * 3600.0) / 60.0).floor() as u64;
        let s = t - h as f64 * 3600.0 - m as f64 * 60.0;
        format!("{h:02}:{m:02}:{s:06.3}")
    };
    let ttml_esc = |txt: &str| {
        txt.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('\n', "<br/>")
    };
    let out = if out_ext == "ttml" || out_ext == "dfxp" {
        // minimal TTML/DFXP (broadcast + Netflix subtitle exchange) —
        // one <p> per cue, HH:MM:SS.mmm clock times, <br/> line breaks
        let mut s = String::from(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <tt xmlns=\"http://www.w3.org/ns/ttml\">\n <body>\n  <div>\n",
        );
        for c in &cues {
            s.push_str(&format!(
                "   <p begin=\"{}\" end=\"{}\">{}</p>\n",
                ttml_clock(c.start),
                ttml_clock(c.end),
                ttml_esc(&c.text)
            ));
        }
        s.push_str("  </div>\n </body>\n</tt>\n");
        s
    } else if out_ext == "lrc" {
        // synced-lyrics: [mm:ss.xx]line per cue — music-player lyrics
        // files from a transcript (multi-line cues join with a space)
        let mut s = String::from(
            "[re:ffkit]
[ve:1.00]

",
        );
        for c in &cues {
            let m = (c.start / 60.0).floor() as u64;
            let sec = c.start - m as f64 * 60.0;
            s.push_str(&format!(
                "[{:02}:{:05.2}]{}\n",
                m,
                sec,
                c.text.split_whitespace().collect::<Vec<_>>().join(" ")
            ));
        }
        s
    } else if out_ext == "txt" {
        // plain-text transcript: flowing prose for shownotes/blogs/LLM
        // input — cue line breaks collapse to spaces, cues join with a space
        let mut s = cues
            .iter()
            .map(|c| c.text.split_whitespace().collect::<Vec<_>>().join(" "))
            .collect::<Vec<_>>()
            .join(" ");
        s.push('\n');
        s
    } else if out_ext == "ass" || out_ext == "ssa" {
        // minimal styled ASS: Default style + one Dialogue line per cue —
        // hand to Aegisub/anime-style pipelines for heavy styling; .ssa
        // gets the same v4.00+ body (what ffmpeg's own .ssa muxer writes)
        let mut s = String::from(
            "[Script Info]\nTitle: ffkit subs convert\nScriptType: v4.00+\n\n\
             [V4+ Styles]\n\
             Format: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding\n\
             Style: Default,Arial,20,&H00FFFFFF,&H000000FF,&H00000000,&H00000000,0,0,0,0,100,100,0,0,1,2,0,2,10,10,10,1\n\n\
             [Events]\n\
             Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\n",
        );
        for c in &cues {
            // {} opens ASS override blocks — swap for parens so text
            // can't be misparsed as styling markup
            let text = c
                .text
                .replace('{', "(")
                .replace('}', ")")
                .replace('\n', "\\N");
            s.push_str(&format!(
                "Dialogue: 0,{},{},Default,,0,0,0,,{}\n",
                ass_ts(c.start),
                ass_ts(c.end),
                text
            ));
        }
        s
    } else if out_ext == "csv" {
        // spreadsheet round-trip: `start,end,"text"` rows — edit cue
        // text/times in Sheets/Excel then convert back; text keeps its
        // line breaks inside a quoted cell
        let mut s = String::from("start,end,text\n");
        for c in &cues {
            let text = if c.text.contains(',') || c.text.contains('"') || c.text.contains('\n') {
                format!("\"{}\"", c.text.replace('"', "\"\""))
            } else {
                c.text.clone()
            };
            s.push_str(&format!(
                "{},{},{}\n",
                ttml_clock(c.start),
                ttml_clock(c.end),
                text
            ));
        }
        s
    } else if out_ext == "smi" {
        // SAMI — Windows Media-era captions: <SYNC Start=ms><P Class=CC>
        // blocks; each cue holds until the next SYNC (our parser reads
        // this format back). Times in ms, newlines kept as real breaks
        let mut s = String::from("<SAMI>\n<BODY>\n");
        for c in &cues {
            s.push_str(&format!(
                "<SYNC Start={}><P Class=ENCC>{}\n",
                (c.start * 1000.0).round() as i64,
                c.text
                    .replace('&', "&amp;")
                    .replace('<', "&lt;")
                    .replace('>', "&gt;")
            ));
        }
        s.push_str("</BODY>\n</SAMI>\n");
        s
    } else if out_ext == "mpl" {
        // MPL2 — Polish legacy player format: [start][end]text in
        // DECISECONDS, | is the line break. Export for editors that
        // still hand-craft .mpl for old toolchain releases
        let mut s = String::new();
        for c in &cues {
            s.push_str(&format!(
                "[{}][{}]{}\n",
                (c.start * 10.0).round() as i64,
                (c.end * 10.0).round() as i64,
                c.text.replace('\n', "|")
            ));
        }
        s
    } else if out_ext == "sbv" {
        // YouTube SubViewer: `H:MM:SS.mmm,H:MM:SS.mmm` header + text
        // lines — upload Studio-editable captions in Studio's own format
        let sbv_clock = |t: f64| {
            let h = (t / 3600.0).floor() as u64;
            let m = ((t - h as f64 * 3600.0) / 60.0).floor() as u64;
            let s = t - h as f64 * 3600.0 - m as f64 * 60.0;
            format!("{h}:{m:02}:{s:06.3}")
        };
        let mut s = String::new();
        for c in &cues {
            s.push_str(&format!(
                "{},{}\n{}\n\n",
                sbv_clock(c.start),
                sbv_clock(c.end),
                c.text
            ));
        }
        s
    } else if out_ext == "sub" {
        // MicroDVD — the other .sub dialect (our reader splits them on
        // content shape): frame-number cues `{start}{end}text` at --fps
        // (default 25), `|` is the line break
        let fps = args.fps.unwrap_or(25.0);
        if fps <= 0.0 {
            return Err(Error::input("subs --convert .sub needs a positive --fps"));
        }
        let mut s = String::new();
        for c in &cues {
            s.push_str(&format!(
                "{{{}}}{{{}}}{}\n",
                (c.start * fps).round() as i64,
                (c.end * fps).round() as i64,
                c.text.replace('\n', "|")
            ));
        }
        s
    } else if out_ext == "pjs" {
        // Phoenix Subtitle — `start,end,"text"` rows in DECISECONDS
        // (20 = 2.0s); completes the .pjs read/write pair, | folds lines
        let mut s = String::new();
        for c in &cues {
            s.push_str(&format!(
                "{},{},\"{}\"\n",
                (c.start * 10.0).round() as i64,
                (c.end * 10.0).round() as i64,
                c.text.replace('\n', "|").replace('"', "'")
            ));
        }
        s
    } else if out_ext == "psb" {
        // PowerSub — {H:MM:SS.mmm}{H:MM:SS.mmm}text; the other brace
        // dialect (MicroDVD .sub braces hold frame numbers — PSB braces
        // hold timestamps). `|` folds lines; completes the read/write pair
        let psb_ts = |t: f64| {
            let h = (t / 3600.0).floor() as u64;
            let m = ((t - h as f64 * 3600.0) / 60.0).floor() as u64;
            let s = t - h as f64 * 3600.0 - m as f64 * 60.0;
            format!("{h}:{m:02}:{s:06.3}")
        };
        let mut s = String::new();
        for c in &cues {
            s.push_str(&format!(
                "{{{}}}{{{}}}{}\n",
                psb_ts(c.start),
                psb_ts(c.end),
                c.text.replace('\n', "|")
            ));
        }
        s
    } else if out_ext == "jss" {
        // JACOsub — `HH:MM:SS.CC HH:MM:SS.CC text` centisecond clocks
        // (anime-sub archive format; `|` folds lines)
        let jss_ts = |t: f64| {
            let h = (t / 3600.0).floor() as u64;
            let m = ((t - h as f64 * 3600.0) / 60.0).floor() as u64;
            let s = t - h as f64 * 3600.0 - m as f64 * 60.0;
            format!("{h:02}:{m:02}:{s:05.2}")
        };
        let mut s = String::new();
        for c in &cues {
            s.push_str(&format!(
                "{} {} {}\n",
                jss_ts(c.start),
                jss_ts(c.end),
                c.text.replace('\n', "|")
            ));
        }
        s
    } else if out_ext == "vtt" {
        let mut s = String::from("WEBVTT\n\n");
        for c in &cues {
            s.push_str(&format!(
                "{} --> {}\n{}\n\n",
                vtt_ts(c.start),
                vtt_ts(c.end),
                c.text
            ));
        }
        s
    } else {
        crate::srt::to_srt(&cues)
    };
    if g.dry_run {
        return Ok(Contract::dry_run(
            "subs",
            Some(crate::paths::display(&args.output)),
            None,
        ));
    }
    std::fs::write(&args.output, out).map_err(|e| Error::output(e.to_string()))?;
    let mut c = Contract::ok("subs", Some(crate::paths::display(&args.output)), None);
    c.verified = Some(args.output.is_file());
    Ok(c.with_extra(json!({ "cues": cues.len(), "words": words, "format": out_ext })))
}

/// `subs a.srt --append b.srt -o ab.srt`: join two subtitle files — b's
/// cues shift to start where a's last cue ends. The matching pattern is
/// `concat` on the clips, then `--append` on their transcripts.
/// `subs a.srt --split 30 -o part.srt` → part_0.srt + part_1.srt — the
/// `split --at` counterpart for transcripts: each part's cues re-time to
/// start at 0 so they stay in sync with the matching video segment.
/// A cue spanning a cut keeps its head (end clamps to the cut).
fn split_sub(args: &SubsArgs, g: &Globals) -> Result<Contract, Error> {
    let is_srt = args
        .input
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("srt"))
        .unwrap_or(false);
    if !is_srt {
        return Err(Error::input("subs --split takes a .srt input"));
    }
    let raw = args.split.as_ref().unwrap();
    let mut cuts: Vec<f64> = Vec::new();
    for part in raw.split(',') {
        let t: f64 = part
            .trim()
            .parse()
            .map_err(|_| Error::input("subs --split needs cut times in seconds (e.g. 30,75)"))?;
        if t <= 0.0 {
            return Err(Error::input("subs --split cuts must be > 0"));
        }
        cuts.push(t);
    }
    if cuts.is_empty() {
        return Err(Error::input("subs --split needs at least one cut time"));
    }
    cuts.sort_by(|a, b| a.partial_cmp(b).unwrap());
    cuts.dedup();
    let cues = crate::srt::parse_srt(&read_sub_file(&args.input, args.encoding.as_deref())?)?;
    let stem = args
        .output
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "part".to_string());
    let mut bounds = vec![0.0];
    bounds.extend(cuts.iter());
    bounds.push(f64::INFINITY);
    let mut files: Vec<String> = Vec::new();
    let mut counts: Vec<usize> = Vec::new();
    for i in 0..=cuts.len() {
        let (lo, hi) = (bounds[i], bounds[i + 1]);
        let part_cues: Vec<crate::srt::Cue> = cues
            .iter()
            .filter(|c| c.start >= lo && c.start < hi)
            .map(|c| crate::srt::Cue {
                start: c.start - lo,
                end: (c.end - lo).min(hi - lo),
                text: c.text.clone(),
            })
            .collect();
        let f = args
            .output
            .with_file_name(format!("{stem}_{i}.srt"))
            .display()
            .to_string();
        if !g.dry_run {
            std::fs::write(&f, crate::srt::to_srt(&part_cues))
                .map_err(|e| Error::output(e.to_string()))?;
        }
        files.push(f);
        counts.push(part_cues.len());
    }
    if g.dry_run {
        return Ok(Contract::dry_run(
            "subs",
            Some(crate::paths::display(&args.output)),
            None,
        ));
    }
    let verified = files.iter().all(|f| std::path::Path::new(f).is_file());
    let mut c = Contract::ok("subs", Some(files[0].clone()), None);
    c.verified = Some(verified);
    Ok(c.with_extra(json!({
        "mode": "split",
        "cuts": cuts,
        "parts": files.len(),
        "cues": counts,
        "files": files,
    })))
}

/// .ass/.ssa input for --convert: pull `Dialogue:`/`Comment:` events from
/// the [Events] section; the Format: line fixes the column order. ASS
/// markup is flattened: `\N`/`\n` → newline, `\h` → space, `{...}`
/// override blocks drop.
fn parse_ass(raw: &str) -> Result<Vec<crate::srt::Cue>, Error> {
    let ts = |s: &str| -> Result<f64, Error> {
        // H:MM:SS.cc
        let s = s.trim();
        let (h, rest) = s
            .split_once(':')
            .ok_or_else(|| Error::input(format!("subs --convert: bad ass timestamp '{s}'")))?;
        let (m, sec) = rest
            .split_once(':')
            .ok_or_else(|| Error::input(format!("subs --convert: bad ass timestamp '{s}'")))?;
        let h: f64 = h
            .trim()
            .parse()
            .map_err(|_| Error::input(format!("subs --convert: bad ass timestamp '{s}'")))?;
        let m: f64 = m
            .trim()
            .parse()
            .map_err(|_| Error::input(format!("subs --convert: bad ass timestamp '{s}'")))?;
        let sec: f64 = sec
            .trim()
            .parse()
            .map_err(|_| Error::input(format!("subs --convert: bad ass timestamp '{s}'")))?;
        Ok(h * 3600.0 + m * 60.0 + sec)
    };
    let mut in_events = false;
    // default column order when no Format: line precedes the events
    let (mut i_start, mut i_end, mut i_text) = (1usize, 2usize, 9usize);
    let mut cols: Option<usize> = None;
    let mut cues = Vec::new();
    for line in raw.lines() {
        let l = line.trim();
        if l.starts_with('[') {
            in_events = l.eq_ignore_ascii_case("[events]");
            continue;
        }
        if !in_events {
            continue;
        }
        if let Some(fmt) = l.strip_prefix("Format:") {
            let names: Vec<String> = fmt
                .split(',')
                .map(|s| s.trim().to_ascii_lowercase())
                .collect();
            cols = Some(names.len());
            for (i, name) in names.iter().enumerate() {
                match name.as_str() {
                    "start" => i_start = i,
                    "end" => i_end = i,
                    "text" => i_text = i,
                    _ => {}
                }
            }
            continue;
        }
        if !(l.starts_with("Dialogue:") || l.starts_with("Comment:")) {
            continue;
        }
        let body = l.split_once(':').map(|(_, b)| b.trim()).unwrap_or("");
        let n_cols = cols.unwrap_or(10);
        // split into n_cols fields — the last split keeps commas in text
        let mut fields: Vec<&str> = Vec::new();
        let mut rest = body;
        for _ in 0..n_cols.saturating_sub(1) {
            match rest.split_once(',') {
                Some((a, b)) => {
                    fields.push(a);
                    rest = b;
                }
                None => break,
            }
        }
        fields.push(rest);
        let get = |i: usize| fields.get(i).copied().unwrap_or("");
        if i_start >= fields.len() || i_end >= fields.len() || i_text >= fields.len() {
            continue;
        }
        let text = get(i_text)
            .replace("\\N", "\n")
            .replace("\\n", "\n")
            .replace("\\h", " ");
        // strip {\...} override blocks
        let mut clean = String::new();
        let mut depth: u32 = 0;
        for ch in text.chars() {
            match ch {
                '{' => depth += 1,
                '}' => depth = depth.saturating_sub(1),
                _ if depth == 0 => clean.push(ch),
                _ => {}
            }
        }
        let text = clean.trim().to_string();
        if text.is_empty() {
            continue;
        }
        cues.push(crate::srt::Cue {
            start: ts(get(i_start))?,
            end: ts(get(i_end))?,
            text,
        });
    }
    if cues.is_empty() {
        return Err(Error::input(
            "subs --convert: no Dialogue events in the .ass file",
        ));
    }
    Ok(cues)
}

/// .ttml/.dfxp input for --convert: `<p begin end>` cues (the broadcast/
/// Netflix exchange format — reads back ffkit's own `.ttml` output too).
/// Clock times are `H:MM:SS.mmm`; `<br/>` breaks become newlines, XML
/// entities unescape, remaining tags strip.
fn parse_ttml(raw: &str) -> Result<Vec<crate::srt::Cue>, Error> {
    let attr = |tag: &str, name: &str| -> Option<String> {
        let pat = format!("{name}=\"");
        let i = tag.find(&pat)? + pat.len();
        let j = tag[i..].find('"')? + i;
        Some(tag[i..j].to_string())
    };
    let unescape = |s: &str| -> String {
        s.replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&apos;", "'")
            .replace("&amp;", "&")
    };
    let mut cues = Vec::new();
    let mut rest = raw;
    while let Some(i) = rest.find("<p ") {
        let seg = &rest[i + 3..];
        let Some(tag_end) = seg.find('>') else {
            break;
        };
        let tag = &seg[..tag_end];
        let after = &seg[tag_end + 1..];
        let Some(close) = after.find("</p>") else {
            break;
        };
        let body = &after[..close];
        rest = &after[close + 4..];
        let (Some(b), Some(e)) = (attr(tag, "begin"), attr(tag, "end")) else {
            continue;
        };
        let start = crate::time::parse_time(&b)
            .map_err(|_| Error::input(format!("subs --convert: bad ttml begin '{b}'")))?;
        let end = crate::time::parse_time(&e)
            .map_err(|_| Error::input(format!("subs --convert: bad ttml end '{e}'")))?;
        let text = body
            .replace("<br/>", "\n")
            .replace("<br />", "\n")
            .replace("<br>", "\n");
        let text = unescape(&crate::srt::strip_markup(&text));
        let text = text.trim();
        if text.is_empty() {
            continue;
        }
        cues.push(crate::srt::Cue {
            start,
            end,
            text: text.to_string(),
        });
    }
    if cues.is_empty() {
        return Err(Error::input(
            "subs --convert: no <p begin end> cues in the .ttml/.dfxp file",
        ));
    }
    Ok(cues)
}

/// .sbv input for --convert: YouTube SubViewer blocks
/// (`H:MM:SS.mmm,H:MM:SS.mmm` header line + text) — Studio caption
/// exports read back into the generic cue model.
fn parse_sbv(raw: &str) -> Result<Vec<crate::srt::Cue>, Error> {
    let norm = |body: &str| -> String {
        body.replace("\r\n", "\n")
            .split("\n\n")
            .filter(|b| !b.trim().is_empty())
            .map(|b| {
                b.lines()
                    .map(|l| {
                        // `H:MM:SS.mmm,H:MM:SS.mmm` → srt arrow form;
                        // a cue-text line with a comma stays untouched
                        // (both halves must actually parse as times)
                        if let Some((a, e)) = l.split_once(',') {
                            let fmt = |t: &str| {
                                let t = t.trim();
                                if t.len() >= 2 && t.chars().nth(1) == Some(':') {
                                    format!("0{t}")
                                } else {
                                    t.to_string()
                                }
                            };
                            let (fa, fe) = (fmt(a), fmt(e));
                            if crate::time::parse_time(&fa).is_ok()
                                && crate::time::parse_time(&fe).is_ok()
                            {
                                return format!("{fa} --> {fe}");
                            }
                        }
                        l.to_string()
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    };
    crate::srt::parse_srt(&norm(raw))
}

/// .csv input for --convert: `start,end,"text"` rows (the format the
/// .csv serializer writes, and what Sheets/Excel exports) — quoted
/// cells keep commas, quotes and line breaks; a header row and any
/// malformed row are skipped.
/// MicroDVD .sub: `{start_frame}{end_frame}line1|line2` — times are frame
/// counts, so a rate is required: the `{1}{1}<fps>` declaration line wins
/// when present, else --fps.
fn parse_microdvd(raw: &str, fps_arg: Option<f64>) -> Result<Vec<crate::srt::Cue>, Error> {
    let mut fps = fps_arg;
    let mut cues = Vec::new();
    for (ln, line) in raw.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Some(c1) = line.strip_prefix('{').and_then(|r| {
            r.find('}')
                .map(|i| (r[..i].parse::<f64>().ok(), &r[i + 1..]))
        }) else {
            return Err(Error::input(format!(
                "sub line {ln}: want {{start}}{{end}}text, got '{line}'"
            )));
        };
        let (Some(start_f), rest) = c1 else {
            return Err(Error::input(format!(
                "sub line {ln}: bad start frame in '{line}'"
            )));
        };
        let Some(end_f) = rest.strip_prefix('{').and_then(|r| {
            r.find('}')
                .map(|i| (r[..i].parse::<f64>().ok(), &r[i + 1..]))
        }) else {
            return Err(Error::input(format!(
                "sub line {ln}: bad end frame in '{line}'"
            )));
        };
        let (Some(end_f), text) = end_f else {
            return Err(Error::input(format!(
                "sub line {ln}: bad end frame in '{line}'"
            )));
        };
        if start_f == 1.0 && end_f == 1.0 {
            // {1}{1}<fps> declaration line
            if let Ok(f) = text.trim().parse::<f64>() {
                if f > 0.0 {
                    fps = Some(f);
                    continue;
                }
            }
        }
        let f = fps.ok_or_else(|| {
            Error::input("MicroDVD .sub times are frames — pass --fps (or a {1}{1}fps header line)")
        })?;
        if end_f <= start_f {
            return Err(Error::input(format!(
                "sub line {ln}: end frame {end_f} ≤ start {start_f}"
            )));
        }
        cues.push(crate::srt::Cue {
            start: start_f / f,
            end: end_f / f,
            text: text.replace(
                '|', "
",
            ),
        });
    }
    if cues.is_empty() {
        return Err(Error::input("sub: no cues found"));
    }
    Ok(cues)
}

/// Phoenix Japanimation Society `.pjs` — `start,end,"text"` rows where the
/// times are DECISECONDS (20 = 2.0s — same units as MPL2, different shape).
/// ffmpeg 4.4's pjs demuxer exists, but the format is a three-field row so
/// ffkit parses it directly — legacy anime-fansub archives.
fn parse_pjs(raw: &str) -> Result<Vec<crate::srt::Cue>, Error> {
    let mut cues = Vec::new();
    for (ln, line) in raw.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Some((start_s, rest)) = line.split_once(',') else {
            return Err(Error::input(format!(
                "pjs line {ln}: want start,end,\"text\" decisecond row, got '{line}'"
            )));
        };
        let Some((end_s, mut text)) = rest.split_once(',') else {
            return Err(Error::input(format!("pjs line {ln}: bad end in '{line}'")));
        };
        let start_d = start_s.trim().parse::<f64>().map_err(|_| {
            Error::input(format!("pjs line {ln}: bad start decisecond in '{line}'"))
        })?;
        let end_d = end_s
            .trim()
            .parse::<f64>()
            .map_err(|_| Error::input(format!("pjs line {ln}: bad end decisecond in '{line}'")))?;
        if end_d <= start_d {
            return Err(Error::input(format!(
                "pjs line {ln}: end {end_d} ≤ start {start_d}"
            )));
        }
        text = text.trim();
        if text.len() >= 2 && text.starts_with('"') && text.ends_with('"') {
            text = &text[1..text.len() - 1];
        }
        let text = text.replace("\\\"", "\"").replace('|', "\n");
        cues.push(crate::srt::Cue {
            start: start_d / 10.0,
            end: end_d / 10.0,
            text,
        });
    }
    if cues.is_empty() {
        return Err(Error::input("pjs: no cues parsed"));
    }
    Ok(cues)
}

/// PowerDivX `.psb` — `{hh:mm:ss.mmm}{hh:mm:ss.mmm}text` timestamp-brace
/// rows. Same brace shape as MicroDVD `.sub` but TIMESTAMPS not frame
/// numbers — paired with .pjs for the legacy-fansub-archive family.
fn parse_psb(raw: &str) -> Result<Vec<crate::srt::Cue>, Error> {
    fn ts(t: &str) -> Option<f64> {
        let (h, rest) = t.split_once(':')?;
        let (m, s) = rest.split_once(':')?;
        Some(
            h.trim().parse::<f64>().ok()? * 3600.0
                + m.trim().parse::<f64>().ok()? * 60.0
                + s.trim().parse::<f64>().ok()?,
        )
    }
    let mut cues = Vec::new();
    for (ln, line) in raw.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Some((start_t, rest)) = line.strip_prefix('{').and_then(|r| r.split_once('}')) else {
            return Err(Error::input(format!(
                "psb line {ln}: want {{start}}{{end}}text, got '{line}'"
            )));
        };
        let Some(start) = ts(start_t.trim()) else {
            return Err(Error::input(format!(
                "psb line {ln}: bad start timestamp in '{line}'"
            )));
        };
        let Some((end_t, text)) = rest.strip_prefix('{').and_then(|r| r.split_once('}')) else {
            return Err(Error::input(format!("psb line {ln}: bad end in '{line}'")));
        };
        let Some(end) = ts(end_t.trim()) else {
            return Err(Error::input(format!(
                "psb line {ln}: bad end timestamp in '{line}'"
            )));
        };
        if end <= start {
            return Err(Error::input(format!(
                "psb line {ln}: end {end} ≤ start {start}"
            )));
        }
        cues.push(crate::srt::Cue {
            start,
            end,
            text: text.replace('|', "\n"),
        });
    }
    if cues.is_empty() {
        return Err(Error::input("psb: no cues parsed"));
    }
    Ok(cues)
}

fn parse_mpl2(raw: &str) -> Result<Vec<crate::srt::Cue>, Error> {
    let mut cues = Vec::new();
    for (ln, line) in raw.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Some(st) = line.strip_prefix('[').and_then(|r| {
            r.find(']')
                .map(|i| (r[..i].parse::<f64>().ok(), &r[i + 1..]))
        }) else {
            return Err(Error::input(format!(
                "mpl line {ln}: want [start][end]text, got '{line}'"
            )));
        };
        let (Some(start_d), rest) = st else {
            return Err(Error::input(format!(
                "mpl line {ln}: bad start decisecond in '{line}'"
            )));
        };
        let Some(en) = rest.strip_prefix('[').and_then(|r| {
            r.find(']')
                .map(|i| (r[..i].parse::<f64>().ok(), &r[i + 1..]))
        }) else {
            return Err(Error::input(format!(
                "mpl line {ln}: bad end decisecond in '{line}'"
            )));
        };
        let (Some(end_d), text) = en else {
            return Err(Error::input(format!(
                "mpl line {ln}: bad end decisecond in '{line}'"
            )));
        };
        if end_d <= start_d {
            return Err(Error::input(format!(
                "mpl line {ln}: end {end_d} ≤ start {start_d}"
            )));
        }
        cues.push(crate::srt::Cue {
            // MPL2 times are deciseconds — [123] = 12.3s
            start: start_d / 10.0,
            end: end_d / 10.0,
            text: text.replace(
                '|', "
",
            ),
        });
    }
    if cues.is_empty() {
        return Err(Error::input("mpl: no cues found"));
    }
    Ok(cues)
}

/// JACOsub `.jss` — cue lines are `HH:MM:SS.CC HH:MM:SS.CC text`
/// (centisecond precision). Lines that don't start with a digit are
/// JACOsub directives/comments and are ignored. `{...}` event braces are
/// stripped and `|` folds to a newline.
fn parse_jss(raw: &str) -> Result<Vec<crate::srt::Cue>, Error> {
    fn tc(t: &str) -> Option<f64> {
        let (hm, cs) = t.split_once('.')?;
        let mut p = hm.split(':');
        let h: f64 = p.next()?.parse().ok()?;
        let m: f64 = p.next()?.parse().ok()?;
        let s: f64 = p.next()?.parse().ok()?;
        Some(h * 3600.0 + m * 60.0 + s + cs.parse::<f64>().ok()? / 100.0)
    }
    let mut cues = Vec::new();
    for (ln, line) in raw.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || !line.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            continue;
        }
        let mut parts = line.splitn(3, char::is_whitespace);
        let (a, b, text) = (
            parts.next(),
            parts.next(),
            parts.next().unwrap_or("").trim(),
        );
        match (a.and_then(tc), b.and_then(tc)) {
            (Some(start), Some(end)) if end > start => {
                let text = text
                    .trim_start_matches('{')
                    .trim_end_matches('}')
                    .replace('|', "\n");
                cues.push(crate::srt::Cue { start, end, text });
            }
            _ => {
                return Err(Error::input(format!(
                    "jss line {}: want HH:MM:SS.CC HH:MM:SS.CC text, got '{line}'",
                    ln + 1
                )));
            }
        }
    }
    if cues.is_empty() {
        return Err(Error::input("jss: no cues found"));
    }
    Ok(cues)
}

fn parse_sami(raw: &str) -> Result<Vec<crate::srt::Cue>, Error> {
    // SAMI (.smi): `<SYNC Start=ms><P Class=CC>text` — a cue holds from its
    // SYNC start until the next SYNC's start; HTML-ish tags inside the text
    // are stripped, | is not special (uses <br> or real newlines)
    let mut marks: Vec<(u64, usize)> = Vec::new(); // (start_ms, text_offset)
    let lo = raw.to_lowercase();
    let mut off = 0usize;
    while let Some(i) = lo[off..].find("<sync") {
        let at = off + i;
        let Some(st_i) = lo[at..].find("start") else {
            break;
        };
        let st_at = at + st_i;
        let after = &raw[st_at + 5..];
        let after_t = after.trim_start();
        let after_t = after_t.strip_prefix('=').unwrap_or(after_t).trim_start();
        let after_t = after_t
            .strip_prefix('"')
            .or_else(|| after_t.strip_prefix('\''))
            .unwrap_or(after_t);
        let num: String = after_t.chars().take_while(|c| c.is_ascii_digit()).collect();
        if num.is_empty() {
            off = at + 5;
            continue;
        }
        let Ok(ms) = num.parse::<u64>() else {
            off = at + 5;
            continue;
        };
        // text begins after the closing '>' of the SYNC tag
        let Some(gt) = lo[st_at..].find('>') else {
            break;
        };
        marks.push((ms, st_at + gt + 1));
        off = st_at + gt + 1;
    }
    if marks.is_empty() {
        return Err(Error::input("smi: no <SYNC Start=ms> blocks found"));
    }
    let strip_tags = |t: &str| -> String {
        let mut out = String::with_capacity(t.len());
        let mut in_tag = false;
        for ch in t.chars() {
            match ch {
                '<' => in_tag = true,
                '>' => in_tag = false,
                _ if !in_tag => out.push(ch),
                _ => {}
            }
        }
        out.replace("&nbsp;", " ")
            .replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&#39;", "'")
    };
    let mut cues = Vec::new();
    for (k, (ms, t_off)) in marks.iter().enumerate() {
        let next_ms = marks.get(k + 1).map(|m| m.0);
        let text_end = if let Some((_, o)) = marks.get(k + 1) {
            // next sync's tag start
            lo[..*o].rfind("<sync").unwrap_or(*o)
        } else {
            raw.len()
        };
        let text = strip_tags(raw[*t_off..text_end].trim());
        if text.is_empty() {
            continue;
        }
        let start = *ms as f64 / 1000.0;
        let end = next_ms.map(|e| e as f64 / 1000.0).unwrap_or(start + 4.0);
        if end <= start {
            continue;
        }
        cues.push(crate::srt::Cue { start, end, text });
    }
    if cues.is_empty() {
        return Err(Error::input("smi: no cues found"));
    }
    Ok(cues)
}

fn parse_csv_subs(raw: &str) -> Result<Vec<crate::srt::Cue>, Error> {
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut field = String::new();
    let mut in_q = false;
    let mut chars = raw.chars().peekable();
    while let Some(ch) = chars.next() {
        if in_q {
            if ch == '"' {
                if chars.peek() == Some(&'"') {
                    field.push('"');
                    chars.next();
                } else {
                    in_q = false;
                }
            } else {
                field.push(ch);
            }
        } else {
            match ch {
                '"' => in_q = true,
                ',' => row.push(std::mem::take(&mut field)),
                '\n' => {
                    row.push(std::mem::take(&mut field));
                    rows.push(std::mem::take(&mut row));
                }
                '\r' => {}
                _ => field.push(ch),
            }
        }
    }
    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        rows.push(row);
    }
    let mut blocks = String::new();
    for r in rows {
        if r.len() < 3 {
            continue;
        }
        let (a, b) = (r[0].trim(), r[1].trim());
        if crate::time::parse_time(a).is_err() || crate::time::parse_time(b).is_err() {
            continue;
        }
        blocks.push_str(&format!("{a} --> {b}\n{}\n\n", r[2..].join(",")));
    }
    crate::srt::parse_srt(&blocks)
}

fn append_sub(args: &SubsArgs, g: &Globals) -> Result<Contract, Error> {
    let second = args.append.as_ref().unwrap();
    for p in [&args.input, second, &args.output] {
        let is_srt = p
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("srt"))
            .unwrap_or(false);
        if !is_srt {
            return Err(Error::input("subs --append joins .srt files only"));
        }
    }
    let a = crate::srt::parse_srt(&read_sub_file(&args.input, args.encoding.as_deref())?)?;
    let b = crate::srt::parse_srt(&read_sub_file(second, args.encoding.as_deref())?)?;
    // shift lands b right after a's last cue — if the joined clip runs
    // past a's last cue the appended cues sit early (see gotchas)
    let offset = a.iter().map(|c| c.end).fold(0.0, f64::max);
    let mut joined = a;
    joined.extend(b.iter().map(|c| crate::srt::Cue {
        start: c.start + offset,
        end: c.end + offset,
        text: c.text.clone(),
    }));
    if g.dry_run {
        return Ok(Contract::dry_run(
            "subs",
            Some(crate::paths::display(&args.output)),
            None,
        ));
    }
    std::fs::write(&args.output, crate::srt::to_srt(&joined))?;
    let mut c = Contract::ok("subs", Some(crate::paths::display(&args.output)), None);
    c.verified = Some(args.output.is_file());
    Ok(c.with_extra(json!({
        "mode": "append",
        "offset": offset,
        "appended": b.len(),
        "cues": joined.len(),
    })))
}

fn apply_case(cues: &mut [crate::srt::Cue], case: crate::cli::TextCase) {
    use crate::cli::TextCase;
    for c in cues.iter_mut() {
        c.text = match case {
            TextCase::Upper => c.text.to_uppercase(),
            TextCase::Lower => c.text.to_lowercase(),
            TextCase::Title => c
                .text
                .split_whitespace()
                .map(|w| {
                    let mut ch = w.chars();
                    match ch.next() {
                        Some(f) => f.to_uppercase().collect::<String>() + ch.as_str(),
                        None => String::new(),
                    }
                })
                .collect::<Vec<_>>()
                .join(" "),
        };
    }
}

/// ASS centisecond timestamp: H:MM:SS.cc
fn ass_ts(secs: f64) -> String {
    let cs = (secs.max(0.0) * 100.0).round() as u64;
    format!(
        "{}:{:02}:{:02}.{:02}",
        cs / 360_000,
        (cs / 6_000) % 60,
        (cs / 100) % 60,
        cs % 100
    )
}

fn vtt_ts(secs: f64) -> String {
    let ms = (secs.max(0.0) * 1000.0).round() as u64;
    format!(
        "{:02}:{:02}:{:02}.{:03}",
        ms / 3_600_000,
        (ms / 60_000) % 60,
        (ms / 1000) % 60,
        ms % 1000
    )
}

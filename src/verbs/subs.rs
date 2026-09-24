use serde_json::json;

use crate::cli::{Globals, SubsArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

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
    if args.convert {
        return convert(&args, g);
    }
    if args.append.is_some() {
        return append_sub(&args, g);
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
        || args.cps.is_some()
        || args.min_dur.is_some()
        || args.max_lines.is_some()
        || args.replace.is_some()
        || args.strip_speakers
        || args.wrap.is_some()
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
            "subs tidy flags (--sort/--fix-overlaps/--dedupe/--cps/--min-dur/--max-lines/--replace/--strip-speakers/--wrap) take an .srt input",
        ));
    }
    let raw = read_sub_file(&args.input, args.encoding.as_deref())?;
    let mut cues = crate::srt::parse_srt(&raw)?;
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
    let mut dropped = 0usize;
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
    if args.strip_speakers {
        for c in &mut cues {
            let t = c
                .text
                .lines()
                .map(|l| {
                    let l = l.trim_start();
                    // [NAME] / <NAME> bracketed labels, or ALL-CAPS NAME: labels
                    let bracket = if l.starts_with('[') {
                        l.find(']')
                    } else if l.starts_with('<') {
                        l.find('>')
                    } else {
                        None
                    };
                    if let Some(end) = bracket {
                        if end <= 32 {
                            return l[end + 1..].trim_start();
                        }
                    }
                    if let Some(colon) = l.find(':') {
                        let head = &l[..colon];
                        if colon <= 30
                            && !head.is_empty()
                            && head.chars().all(|ch| {
                                ch.is_ascii_uppercase()
                                    || ch.is_ascii_digit()
                                    || " .'_-".contains(ch)
                            })
                            && head.chars().any(|ch| ch.is_ascii_uppercase())
                        {
                            return l[colon + 1..].trim_start();
                        }
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
    Ok(c.with_extra(json!({
        "mode": "tidy",
        "sorted": sorted,
        "clamped": clamped,
        "dropped": dropped,
        "extended": extended,
        "replaced": replaced,
        "stripped": stripped,
        "rewrapped": rewrapped,
        "cues": cues.len(),
        "cps_limit": args.cps,
        "over_limit": over_limit,
        "worst_cps": (worst_cps * 100.0).round() / 100.0,
        "over_lines": over_lines,
        "worst_lines": worst_lines,
    })))
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
    if in_ext != "srt" && in_ext != "vtt" {
        return Err(Error::input(
            "subs --convert takes .srt/.vtt input and .srt/.vtt/.txt output",
        ));
    }
    if out_ext != "srt" && out_ext != "vtt" && out_ext != "txt" {
        return Err(Error::input(
            "subs --convert takes .srt/.vtt input and .srt/.vtt/.txt output",
        ));
    }
    let raw = read_sub_file(&args.input, args.encoding.as_deref())?;
    // vtt → srt-shaped blocks: drop WEBVTT/NOTE/STYLE blocks and cue settings.
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
    let mut cues = crate::srt::parse_srt(&body)?;
    if let Some(case) = args.case {
        apply_case(&mut cues, case);
    }
    let words = cues
        .iter()
        .map(|c| c.text.split_whitespace().count())
        .sum::<usize>();
    let out = if out_ext == "txt" {
        // plain-text transcript: flowing prose for shownotes/blogs/LLM
        // input — cue line breaks collapse to spaces, cues join with a space
        let mut s = cues
            .iter()
            .map(|c| c.text.split_whitespace().collect::<Vec<_>>().join(" "))
            .collect::<Vec<_>>()
            .join(" ");
        s.push('\n');
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

use serde_json::json;

use crate::cli::{Globals, SubsArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: SubsArgs, g: &Globals) -> Result<Contract, Error> {
    if args.all {
        return extract_all(&args, g);
    }
    if args.convert {
        return convert(&args, g);
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
    let _probe = engine::probe_or_err(&args.input, g)?;
    if let Some(subs) = &args.burn {
        return burn(&args, subs, g);
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
    let read = |p: &std::path::Path| -> Result<Vec<crate::srt::Cue>, Error> {
        let raw = std::fs::read_to_string(p)
            .map_err(|e| Error::input(format!("read {}: {e}", p.display())))?;
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

fn burn(args: &SubsArgs, subs: &std::path::Path, g: &Globals) -> Result<Contract, Error> {
    if !subs.is_file() {
        return Err(Error::input(format!(
            "no such subtitle file: {}",
            subs.display()
        )));
    }
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
    let align = if args.top { 8 } else { 2 };
    let margin_v = if args.safe {
        // social-safe zone: keep burned text off the bottom 20% / top 15%
        let probe = engine::probe_or_err(&args.input, g)?;
        let frac = if args.top { 0.15 } else { 0.20 };
        ((probe.height.unwrap_or(720) as f64) * frac + 36.0) as u32
    } else {
        36
    };
    let font = args.font.as_deref().unwrap_or("Sans").replace(',', " ");
    let outline = args.outline.unwrap_or(1.0).clamp(0.0, 8.0);
    let style = format!(
        "FontName={font},FontSize={size},PrimaryColour={color},\
OutlineColour=&H80000000,BorderStyle=1,Outline={outline},Shadow=0,\
MarginV={margin_v},Alignment={align}"
    );
    let vf = format!("subtitles=filename='{path}':force_style='{style}'");

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
    let raw = std::fs::read_to_string(&args.input)
        .map_err(|e| Error::input(format!("{}: {e}", args.input.display())))?;
    let mut cues = crate::srt::parse_srt(&raw)?;
    for c in cues.iter_mut() {
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
    let raw = std::fs::read_to_string(&args.input)
        .map_err(|e| Error::input(format!("{}: {e}", args.input.display())))?;
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
    for e in [&in_ext, &out_ext] {
        if e != "srt" && e != "vtt" {
            return Err(Error::input(
                "subs --convert takes .srt/.vtt input and output",
            ));
        }
    }
    let raw = std::fs::read_to_string(&args.input)
        .map_err(|e| Error::input(format!("{}: {e}", args.input.display())))?;
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
    let cues = crate::srt::parse_srt(&body)?;
    let out = if out_ext == "vtt" {
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
    Ok(c.with_extra(json!({ "cues": cues.len(), "format": out_ext })))
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

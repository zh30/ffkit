use serde_json::json;

use crate::cli::{Globals, SubsArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: SubsArgs, g: &Globals) -> Result<Contract, Error> {
    if let Some(offset) = args.shift {
        return shift(&args, offset, g);
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
    let font = args.font.as_deref().unwrap_or("Sans").replace(',', " ");
    let style = format!(
        "FontName={font},FontSize={size},PrimaryColour={color},\
OutlineColour=&H80000000,BorderStyle=1,Outline=1,Shadow=0,\
MarginV=36,Alignment={align}"
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

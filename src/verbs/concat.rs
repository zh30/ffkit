use std::io::Write;
use std::path::Path;

use clap::ValueEnum;

use crate::cli::{ConcatArgs, Globals, XfadeTransition};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::paths;
use crate::probe::Probe;
use serde_json::json;

pub fn run(args: ConcatArgs, g: &Globals) -> Result<Contract, Error> {
    let mut args = args;
    // --list: clip manifest (one path per line) for script-generated cuts;
    // relative paths resolve against the list's own directory
    if let Some(list) = &args.list {
        if !args.inputs.is_empty() {
            return Err(Error::input(
                "concat --list replaces the positional clip args — pick one",
            ));
        }
        let text = std::fs::read_to_string(list)
            .map_err(|e| Error::input(format!("--list: {}: {e}", list.display())))?;
        let dir = list.parent().unwrap_or_else(|| Path::new(""));
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let p = Path::new(line);
            args.inputs.push(if p.is_absolute() {
                p.into()
            } else {
                dir.join(p)
            });
        }
        if args.inputs.is_empty() {
            return Err(Error::input(format!(
                "--list: {} has no clip paths",
                list.display()
            )));
        }
    }
    if let Some(l) = args.level {
        if !(-70.0..=-5.0).contains(&l) {
            return Err(Error::input("--level must be -70..=-5 LUFS (e.g. -14)"));
        }
    }
    if args.inputs.len() < 2 {
        return Err(Error::input("concat needs at least two inputs"));
    }
    let input_refs: Vec<&Path> = args.inputs.iter().map(Path::new).collect();

    let probes: Vec<Probe> = args
        .inputs
        .iter()
        .map(|p| engine::probe_or_err(p, g))
        .collect::<Result<_, _>>()?;

    // --chapters: every input becomes a container chapter titled by its
    // filename stem at its join point (audiobook/podcast assembly).
    // transition/gap shift or pad the joins, so marks would drift — refused.
    let mut chap_file: Option<std::path::PathBuf> = None;
    if args.chapters {
        if args.transition.is_some() || args.gap.is_some() {
            return Err(Error::input(
                "concat --chapters titles the plain joins — drop --transition/--gap",
            ));
        }
        let mut marks: Vec<(f64, String)> = Vec::new();
        let mut t = 0.0;
        for (p, probe) in args.inputs.iter().zip(probes.iter()) {
            let stem = p
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| format!("part {}", marks.len() + 1));
            marks.push((t, stem));
            t += probe.duration;
        }
        let tmp =
            std::env::temp_dir().join(format!("ffkit-concat-chap-{}.ffmeta", std::process::id()));
        std::fs::write(&tmp, crate::verbs::chapter::ffmeta_table(&marks, t))
            .map_err(|e| Error::output(format!("writing chapters: {e}")))?;
        chap_file = Some(tmp);
    }

    if let Some(t) = &args.transition {
        let mut kinds = Vec::new();
        for part in t.split(',') {
            let name = part.trim();
            let kind = XfadeTransition::from_str(name, true)
                .map_err(|_| Error::input(format!("concat: unknown --transition '{name}'")))?;
            kinds.push(kind);
        }
        if kinds.len() > 1 && kinds.len() != args.inputs.len() - 1 {
            return Err(Error::input(format!(
                "concat: {} transitions but {} joints — give one per joint or a single one",
                kinds.len(),
                args.inputs.len() - 1
            )));
        }
        return transition_chain(&args, g, &input_refs, &probes, &kinds);
    }

    if let Some(gap) = args.gap {
        if !(0.05..=60.0).contains(&gap) {
            return Err(Error::input("concat: --gap must be 0.05..60 seconds"));
        }
        return gap_concat(&args, g, &input_refs, &probes, gap);
    }

    let mut c = if args.audio_fade.is_none() && can_copy(&probes) {
        copy_concat(&args, g, &input_refs, chap_file.as_deref())?
    } else {
        filter_concat(&args, g, &input_refs, &probes, chap_file.as_deref())?
    };
    if let Some(cf) = &chap_file {
        c = c.with_extra(json!({ "chapters": args.inputs.len() }));
        std::fs::remove_file(cf).ok();
    }
    Ok(c)
}

fn can_copy(probes: &[Probe]) -> bool {
    let first = &probes[0];
    probes.iter().all(|p| {
        p.vcodec == first.vcodec
            && p.acodec == first.acodec
            && p.width == first.width
            && p.height == first.height
            && p.has_video == first.has_video
            && p.has_audio == first.has_audio
            && fps_close(p.fps, first.fps)
    })
}

fn fps_close(a: Option<f64>, b: Option<f64>) -> bool {
    match (a, b) {
        (Some(x), Some(y)) => (x - y).abs() < 0.05,
        (None, None) => true,
        _ => false,
    }
}

fn copy_concat(
    args: &ConcatArgs,
    g: &Globals,
    inputs: &[&Path],
    chapters: Option<&Path>,
) -> Result<Contract, Error> {
    let mut list = tempfile::NamedTempFile::new().map_err(|e| Error::output(e.to_string()))?;
    for p in &args.inputs {
        let abs = paths::abs(p);
        let escaped = abs.to_string_lossy().replace('\'', "'\\''");
        writeln!(list, "file '{escaped}'").map_err(|e| Error::output(e.to_string()))?;
    }
    list.flush().ok();
    let list_path = list.path().to_path_buf();

    let mut argv = ffmpeg_base(g.progress);
    argv.extend(["-f", "concat", "-safe", "0", "-i"]);
    argv.push(&list_path);
    if let Some(cf) = chapters {
        argv.extend(["-f", "ffmetadata", "-i"]);
        argv.push(cf);
    }
    argv.extend(["-c", "copy"]);
    if chapters.is_some() {
        argv.extend(["-map_chapters", "1"]);
    }
    argv.push(&args.output);

    let result = engine::write_job("concat", inputs, &args.output, vec![argv], g);
    drop(list);
    result
}

fn filter_concat(
    args: &ConcatArgs,
    g: &Globals,
    inputs: &[&Path],
    probes: &[Probe],
    chapters: Option<&Path>,
) -> Result<Contract, Error> {
    let first = &probes[0];
    let has_v = first.has_video;
    let has_a = first.has_audio;
    if !has_v && !has_a {
        return Err(Error::input("concat: no streams"));
    }
    if let Some(f) = args.audio_fade {
        if !(0.05..=60.0).contains(&f) {
            return Err(Error::input("--audio-fade seconds must be 0.05-60"));
        }
        if !has_a {
            return Err(Error::input("--audio-fade needs audio on every clip"));
        }
        let shortest = probes
            .iter()
            .map(|p| p.duration)
            .fold(f64::INFINITY, f64::min);
        if f >= shortest {
            return Err(Error::input("--audio-fade longer than the shortest clip"));
        }
    }
    let tw = paths::even(first.width.unwrap_or(1280));
    let th = paths::even(first.height.unwrap_or(720));
    let fps = first.fps.unwrap_or(30.0);

    let mut argv = ffmpeg_base(g.progress);
    for p in &args.inputs {
        argv.push("-i");
        argv.push(p);
    }

    let n = args.inputs.len();
    if let Some(cf) = chapters {
        argv.extend(["-f", "ffmetadata", "-i"]);
        argv.push(cf);
    }
    let mut fc = String::new();
    let mut concat_ins = String::new();
    for (i, pr) in probes.iter().enumerate() {
        if has_v {
            fc.push_str(&format!(
                "[{i}:v]scale={tw}:{th}:force_original_aspect_ratio=decrease,pad={tw}:{th}:(ow-iw)/2:(oh-ih)/2,setsar=1,fps={fps},format=yuv420p[v{i}];"
            ));
            concat_ins.push_str(&format!("[v{i}]"));
        }
        if has_a {
            match args.audio_fade {
                // Boundary fades keep each clip's duration (and lip sync): the
                // outgoing clip fades out on its tail, the incoming fades in on
                // its head — no overlap, no drift across joints.
                Some(f) => {
                    let dur = pr.duration.max(0.0);
                    let mut chain = format!("[{i}:a]asetpts=PTS-STARTPTS");
                    if i > 0 {
                        chain.push_str(&format!(",afade=t=in:st=0:d={f:.3}"));
                    }
                    if i + 1 < n {
                        let st = (dur - f).max(0.0);
                        chain.push_str(&format!(",afade=t=out:st={st:.3}:d={f:.3}"));
                    }
                    fc.push_str(&format!("{chain}[a{i}];"));
                    concat_ins.push_str(&format!("[a{i}]"));
                }
                None => concat_ins.push_str(&format!("[{i}:a]")),
            }
        }
    }
    let v = if has_v { 1 } else { 0 };
    let a = if has_a { 1 } else { 0 };
    fc.push_str(&format!("{concat_ins}concat=n={n}:v={v}:a={a}[vout]"));
    if has_a {
        fc.push_str("[aout]");
    }

    argv.extend(["-filter_complex", &fc]);
    if has_v {
        argv.extend(["-map", "[vout]"]);
    }
    if has_a {
        argv.extend(["-map", "[aout]"]);
    }
    if has_v {
        argv.extend(["-c:v", "libx264", "-preset", "fast", "-crf", "18"]);
    }
    if has_a {
        argv.extend(["-c:a", "aac"]);
    }
    if chapters.is_some() {
        argv.extend(["-map_chapters", &n.to_string()]);
    }
    argv.push(&args.output);
    let c = engine::write_job("concat", inputs, &args.output, vec![argv], g)?;
    Ok(match args.audio_fade {
        Some(f) => c.with_extra(json!({ "audio_fade": f })),
        None => c,
    })
}

// N-input xfade chain: transition i starts at cumsum(d_0..d_i) - i*fade;
// audio mirrors it with an acrossfade chain at the same boundaries.
fn transition_chain(
    args: &ConcatArgs,
    g: &Globals,
    inputs: &[&Path],
    probes: &[Probe],
    kinds: &[crate::cli::XfadeTransition],
) -> Result<Contract, Error> {
    engine::need_video(&probes[0], "concat")?;
    let fade = args.duration.max(0.01);
    for (i, p) in probes.iter().enumerate() {
        if p.duration <= fade {
            return Err(Error::input(format!(
                "concat: input {} ({:.2}s) is shorter than the {fade:.2}s transition",
                i + 1,
                p.duration
            )));
        }
    }
    let all_audio = probes.iter().all(|p| p.has_audio);
    let tw = paths::even(probes[0].width.unwrap_or(1280));
    let th = paths::even(probes[0].height.unwrap_or(720));
    let fps = probes[0].fps.unwrap_or(30.0);
    let n = args.inputs.len();
    let names: Vec<&'static str> = if kinds.len() == 1 {
        vec![kinds[0].xfade_name(); n - 1]
    } else {
        kinds.iter().map(|k| k.xfade_name()).collect()
    };

    let mut argv = ffmpeg_base(g.progress);
    for p in &args.inputs {
        argv.push("-i");
        argv.push(p);
    }

    let mut seg = Vec::new();
    for i in 0..n {
        seg.push(format!(
            "[{i}:v]scale={tw}:{th}:force_original_aspect_ratio=decrease,pad={tw}:{th}:(ow-iw)/2:(oh-ih)/2,setsar=1,fps={fps:.3},format=yuv420p[v{i}]"
        ));
        if all_audio {
            let lvl = args
                .level
                .map(|l| format!(",loudnorm=I={l:.1}"))
                .unwrap_or_default();
            seg.push(format!(
                "[{i}:a]aresample=48000,aformat=channel_layouts=stereo{lvl}[a{i}]"
            ));
        }
    }
    let mut prev_v = "v0".to_string();
    let mut prev_a = "a0".to_string();
    let mut cum = 0.0;
    for i in 1..n {
        let last = i == n - 1;
        let out_v = if last {
            "vout".to_string()
        } else {
            format!("x{i}")
        };
        let offset = cum + probes[i - 1].duration - i as f64 * fade;
        seg.push(format!(
            "[{prev_v}][v{i}]xfade=transition={}:duration={fade:.3}:offset={offset:.3}[{out_v}]",
            names[i - 1]
        ));
        prev_v = out_v;
        if all_audio {
            let out_a = if last {
                "aout".to_string()
            } else {
                format!("af{i}")
            };
            seg.push(format!("[{prev_a}][a{i}]acrossfade=d={fade:.3}[{out_a}]"));
            prev_a = out_a;
        }
        cum += probes[i - 1].duration;
    }
    let fc = seg.join(";");

    argv.extend(["-filter_complex", &fc, "-map", "[vout]"]);
    if all_audio {
        argv.extend(["-map", "[aout]", "-c:a", "aac"]);
    }
    argv.extend(["-c:v", "libx264", "-preset", "fast", "-crf", "18"]);
    argv.push(&args.output);
    let mut c = engine::write_job("concat", inputs, &args.output, vec![argv], g)?;
    c = c.with_extra(serde_json::json!({
        "transition": if names.iter().all(|n| *n == names[0]) {
            serde_json::json!(names[0])
        } else {
            serde_json::json!(names)
        },
        "transition_duration": fade,
        "clips": n,
    }));
    Ok(c)
}

/// Black+silent spacer between every pair of clips (beat gap between
/// montage sections). Always goes through filter concat so the generated
/// pads match the common canvas.
fn gap_concat(
    args: &ConcatArgs,
    g: &Globals,
    inputs: &[&Path],
    probes: &[Probe],
    gap: f64,
) -> Result<Contract, Error> {
    let first = &probes[0];
    let has_v = first.has_video;
    let has_a = first.has_audio;
    if !has_v && !has_a {
        return Err(Error::input("concat: no streams"));
    }
    let tw = paths::even(first.width.unwrap_or(1280));
    let th = paths::even(first.height.unwrap_or(720));
    let fps = first.fps.unwrap_or(30.0);
    let n = inputs.len();

    let mut argv = ffmpeg_base(g.progress);
    for p in &args.inputs {
        argv.push("-i");
        argv.push(p);
    }

    let mut fc = String::new();
    let mut concat_ins = String::new();
    let mut k = 0usize;
    for i in 0..n {
        if has_v {
            fc.push_str(&format!(
                "[{i}:v]scale={tw}:{th}:force_original_aspect_ratio=decrease,pad={tw}:{th}:(ow-iw)/2:(oh-ih)/2,setsar=1,fps={fps},format=yuv420p[v{k}];"
            ));
            concat_ins.push_str(&format!("[v{k}]"));
        }
        if has_a {
            concat_ins.push_str(&format!("[{i}:a]"));
        }
        k += 1;
        if i + 1 < n {
            if has_v {
                fc.push_str(&format!(
                    "color=black:s={tw}x{th}:r={fps}:d={gap:.3}[v{k}];"
                ));
                concat_ins.push_str(&format!("[v{k}]"));
            }
            if has_a {
                concat_ins.push_str(&format!("[g{k}]"));
                fc.push_str(&format!("anullsrc=r=48000:cl=stereo:d={gap:.3}[g{k}];"));
            }
            k += 1;
        }
    }
    let v = if has_v { 1 } else { 0 };
    let a = if has_a { 1 } else { 0 };
    let segs = concat_ins.matches('[').count() / (v + a);
    fc.push_str(&format!("{concat_ins}concat=n={segs}:v={v}:a={a}[vout]"));
    if has_a {
        fc.push_str("[aout]");
    }

    argv.extend(["-filter_complex", &fc]);
    if has_v {
        argv.extend(["-map", "[vout]"]);
    }
    if has_a {
        argv.extend(["-map", "[aout]"]);
    }
    if has_v {
        argv.extend([
            "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
        ]);
    }
    if has_a {
        argv.extend(["-c:a", "aac"]);
    }
    argv.push(&args.output);

    let c = engine::write_job("concat", inputs, &args.output, vec![argv], g)?;
    let mut extra = serde_json::Map::new();
    extra.insert("gap".to_string(), json!(gap));
    extra.insert("inputs".to_string(), json!(n));
    Ok(c.with_extra(serde_json::Value::Object(extra)))
}

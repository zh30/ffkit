use std::io::Write;
use std::path::Path;

use crate::cli::{ConcatArgs, Globals};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::paths;
use crate::probe::Probe;

pub fn run(args: ConcatArgs, g: &Globals) -> Result<Contract, Error> {
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

    if let Some(t) = args.transition {
        return transition_chain(&args, g, &input_refs, &probes, t);
    }

    if can_copy(&probes) {
        copy_concat(&args, g, &input_refs)
    } else {
        filter_concat(&args, g, &input_refs, &probes)
    }
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

fn copy_concat(args: &ConcatArgs, g: &Globals, inputs: &[&Path]) -> Result<Contract, Error> {
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
    argv.extend(["-c", "copy"]);
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

    let mut argv = ffmpeg_base(g.progress);
    for p in &args.inputs {
        argv.push("-i");
        argv.push(p);
    }

    let n = args.inputs.len();
    let mut fc = String::new();
    let mut concat_ins = String::new();
    for i in 0..n {
        if has_v {
            fc.push_str(&format!(
                "[{i}:v]scale={tw}:{th}:force_original_aspect_ratio=decrease,pad={tw}:{th}:(ow-iw)/2:(oh-ih)/2,setsar=1,fps={fps},format=yuv420p[v{i}];"
            ));
            concat_ins.push_str(&format!("[v{i}]"));
        }
        if has_a {
            concat_ins.push_str(&format!("[{i}:a]"));
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
    argv.push(&args.output);
    engine::write_job("concat", inputs, &args.output, vec![argv], g)
}

// N-input xfade chain: transition i starts at cumsum(d_0..d_i) - i*fade;
// audio mirrors it with an acrossfade chain at the same boundaries.
fn transition_chain(
    args: &ConcatArgs,
    g: &Globals,
    inputs: &[&Path],
    probes: &[Probe],
    kind: crate::cli::XfadeTransition,
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
    let name = kind.xfade_name();

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
            "[{prev_v}][v{i}]xfade=transition={name}:duration={fade:.3}:offset={offset:.3}[{out_v}]"
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
        "transition": name,
        "transition_duration": fade,
        "clips": n,
    }));
    Ok(c)
}

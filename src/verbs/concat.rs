use std::io::Write;
use std::path::Path;

use crate::cli::{ConcatArgs, Globals};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::paths;
use crate::probe::Probe;

pub fn run(args: ConcatArgs, g: &Globals) -> Result<Contract, Error> {
    if args.inputs.len() < 2 {
        return Err(Error::input("concat needs at least two inputs"));
    }
    let input_refs: Vec<&Path> = args.inputs.iter().map(Path::new).collect();

    if let Some(t) = args.transition.as_deref() {
        if t != "fade" {
            return Err(Error::input(
                "only --transition fade is supported; use graph for other xfade names",
            ));
        }
        if args.inputs.len() != 2 {
            return Err(Error::input(
                "fade concat supports exactly two inputs; chain via graph for more",
            ));
        }
        return fade_two(&args, g, &input_refs);
    }

    let probes: Vec<Probe> = args
        .inputs
        .iter()
        .map(|p| engine::probe_or_err(p, g))
        .collect::<Result<_, _>>()?;

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

fn fade_two(args: &ConcatArgs, g: &Globals, inputs: &[&Path]) -> Result<Contract, Error> {
    let a = engine::probe_or_err(&args.inputs[0], g)?;
    let _b = engine::probe_or_err(&args.inputs[1], g)?;
    engine::need_video(&a, "concat")?;
    let fade = args.duration.max(0.01);
    if a.duration <= fade {
        return Err(Error::input("first clip is shorter than the fade"));
    }
    let offset = a.duration - fade;
    let tw = paths::even(a.width.unwrap_or(1280));
    let th = paths::even(a.height.unwrap_or(720));

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.inputs[0]);
    argv.push("-i");
    argv.push(&args.inputs[1]);

    let mut fc = format!(
        "[0:v]scale={tw}:{th}:force_original_aspect_ratio=decrease,pad={tw}:{th}:(ow-iw)/2:(oh-ih)/2,setsar=1,format=yuv420p[v0];\
         [1:v]scale={tw}:{th}:force_original_aspect_ratio=decrease,pad={tw}:{th}:(ow-iw)/2:(oh-ih)/2,setsar=1,format=yuv420p[v1];\
         [v0][v1]xfade=transition=fade:duration={fade}:offset={offset}[vout]"
    );
    if a.has_audio {
        fc.push_str(&format!(";[0:a][1:a]acrossfade=d={fade}[aout]"));
    }
    argv.extend(["-filter_complex", &fc, "-map", "[vout]"]);
    if a.has_audio {
        argv.extend(["-map", "[aout]", "-c:a", "aac"]);
    }
    argv.extend(["-c:v", "libx264", "-preset", "fast", "-crf", "18"]);
    argv.push(&args.output);
    engine::write_job("concat", inputs, &args.output, vec![argv], g)
}

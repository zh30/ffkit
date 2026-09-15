use std::path::{Path, PathBuf};

use crate::cli::{Globals, LookArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::time::{fmt_time, parse_time};

pub fn run(args: LookArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "look")?;

    let times: Vec<f64> = args
        .at
        .iter()
        .map(|s| parse_time(s))
        .collect::<Result<Vec<_>, _>>()?;
    if times.len() > 8 {
        return Err(Error::input("look --at accepts at most 8 timestamps"));
    }

    let output = args
        .output
        .clone()
        .unwrap_or_else(|| default_output(&args.input, times.len()));

    if times.is_empty() {
        let (cols, rows) = parse_tiles(&args.tiles)?;
        let n = (cols * rows).max(1) as f64;
        let fps = if probe.duration > 0.0 {
            (n / probe.duration).max(0.1)
        } else {
            n
        };
        let mut argv = ffmpeg_base(g.progress);
        argv.push("-i");
        argv.push(&args.input);
        let vf = format!(
            "fps={fps:.4},scale=320:-2,format=yuv420p,tile={cols}x{rows}:padding=4:margin=4"
        );
        argv.extend(["-vf", &vf, "-frames:v", "1"]);
        argv.push(&output);
        return engine::write_job("look", &[&args.input], &output, vec![argv], g);
    }

    if times.len() == 1 {
        let mut argv = ffmpeg_base(g.progress);
        argv.extend(["-ss", &fmt_time(times[0])]);
        argv.push("-i");
        argv.push(&args.input);
        argv.extend(["-frames:v", "1", "-q:v", "2"]);
        argv.push(&output);
        return engine::write_job("look", &[&args.input], &output, vec![argv], g);
    }

    strip(&args.input, &output, &times, g)
}

fn strip(input: &Path, output: &Path, times: &[f64], g: &Globals) -> Result<Contract, Error> {
    let tmp = tempfile::tempdir().map_err(|e| Error::output(e.to_string()))?;
    let mut frames = Vec::new();
    let mut argvs = Vec::new();
    for (i, t) in times.iter().enumerate() {
        let frame = tmp.path().join(format!("f{i}.png"));
        let mut argv = ffmpeg_base(g.progress);
        argv.extend(["-ss", &fmt_time(*t)]);
        argv.push("-i");
        argv.push(input);
        argv.extend(["-frames:v", "1", "-vf", "scale=320:-2", "-q:v", "2"]);
        argv.push(&frame);
        argvs.push(argv);
        frames.push(frame);
    }

    let mut stack = ffmpeg_base(g.progress);
    for f in &frames {
        stack.push("-i");
        stack.push(f);
    }
    let n = frames.len();
    stack.extend([
        "-filter_complex",
        &format!("hstack=inputs={n}"),
        "-frames:v",
        "1",
    ]);
    stack.push(output);
    argvs.push(stack);

    let result = engine::write_job("look", &[input], output, argvs, g);
    drop(tmp);
    result
}

fn default_output(input: &Path, n_times: usize) -> PathBuf {
    let stem = input.file_stem().and_then(|s| s.to_str()).unwrap_or("out");
    let parent = input.parent().unwrap_or_else(|| Path::new("."));
    let name = match n_times {
        0 => format!("{stem}_sheet.png"),
        1 => format!("{stem}_frame.png"),
        _ => format!("{stem}_strip.png"),
    };
    parent.join(name)
}

fn parse_tiles(s: &str) -> Result<(u32, u32), Error> {
    let (a, b) = s
        .split_once('x')
        .or_else(|| s.split_once('X'))
        .ok_or_else(|| Error::input("--tiles must look like 3x2"))?;
    let cols: u32 = a
        .parse()
        .map_err(|_| Error::input("--tiles must look like 3x2"))?;
    let rows: u32 = b
        .parse()
        .map_err(|_| Error::input("--tiles must look like 3x2"))?;
    if cols == 0 || rows == 0 || cols * rows > 36 {
        return Err(Error::input("--tiles grid must be 1..=36 cells"));
    }
    Ok((cols, rows))
}

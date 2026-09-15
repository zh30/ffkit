use std::io::Write;
use std::path::{Path, PathBuf};

use serde_json::json;

use crate::cli::{Globals, RoughArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::paths;
use crate::spawn::Argv;
use crate::time::fmt_time;

const MAX_KEEPS: usize = 48;

pub fn run(args: RoughArgs, g: &Globals) -> Result<Contract, Error> {
    if args.min_duration <= 0.0 || args.min_duration > 30.0 {
        return Err(Error::input("--min-duration must be in (0, 30] seconds"));
    }
    if args.pad < 0.0 || args.pad > args.min_duration {
        return Err(Error::input("--pad must be >= 0 and <= --min-duration"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_audio {
        return Err(Error::input("rough needs an audio stream to find speech"));
    }

    let silences = crate::silence::detect(
        &args.input,
        args.threshold,
        args.min_duration,
        g.timeout,
        true,
    )?;
    let keeps = crate::silence::keep_ranges(probe.duration, &silences, args.pad);
    if keeps.is_empty() {
        return Err(Error::input("rough: clip is all silence at this threshold"));
    }
    if keeps.len() > MAX_KEEPS {
        return Err(Error::input(format!(
            "rough found {} speech islands (max {MAX_KEEPS}); raise --min-duration",
            keeps.len()
        )));
    }

    let speech: f64 = keeps.iter().map(|(s, e)| e - s).sum();
    let extra = json!({
        "copy": args.copy,
        "keeps": keeps.iter().enumerate().map(|(i, (s, e))| json!({
            "i": i,
            "start": s,
            "end": e,
            "duration": e - s,
        })).collect::<Vec<_>>(),
        "silences": silences.len(),
        "speech_seconds": speech,
        "silence_seconds": (probe.duration - speech).max(0.0),
        "kept": keeps.len(),
    });

    let Some(output) = args.output.as_ref() else {
        let mut c = Contract::ok("rough", None, Some(probe));
        c.summary = Some(format!(
            "{} keeps, {:.1}s speech / {:.1}s take",
            keeps.len(),
            speech,
            extra["silence_seconds"].as_f64().unwrap_or(0.0) + speech
        ));
        return Ok(c.with_extra(extra));
    };

    let whole = keeps.len() == 1 && keeps[0].0 < 0.02 && (probe.duration - keeps[0].1) < 0.02;
    if whole {
        let mut argv = ffmpeg_base(g.progress);
        argv.push("-i");
        argv.push(&args.input);
        argv.extend(["-c", "copy"]);
        argv.push(output);
        let c = engine::write_job("rough", &[&args.input], output, vec![argv], g)?;
        return Ok(c.with_extra(extra));
    }

    let tmp = tempfile::tempdir().map_err(|e| Error::output(e.to_string()))?;
    let mut argvs: Vec<Argv> = Vec::new();
    let mut parts: Vec<PathBuf> = Vec::new();
    for (i, (s, e)) in keeps.iter().enumerate() {
        let part = tmp.path().join(format!("{i:02}.mp4"));
        argvs.push(segment_argv(
            &args.input,
            &part,
            *s,
            *e - *s,
            args.copy,
            probe.has_video,
            g.progress,
        ));
        parts.push(part);
    }
    let list = tmp.path().join("concat.txt");
    let mut list_file = std::fs::File::create(&list).map_err(|e| Error::output(e.to_string()))?;
    for p in &parts {
        let abs = paths::abs(p);
        let escaped = abs.to_string_lossy().replace('\'', "'\\''");
        writeln!(list_file, "file '{escaped}'").map_err(|e| Error::output(e.to_string()))?;
    }
    list_file.flush().ok();

    let mut concat = ffmpeg_base(g.progress);
    concat.extend(["-f", "concat", "-safe", "0", "-i"]);
    concat.push(&list);
    concat.extend(["-c", "copy"]);
    concat.push(output);
    argvs.push(concat);

    let result = engine::write_job("rough", &[&args.input], output, argvs, g);
    drop(tmp);
    Ok(result?.with_extra(extra))
}

fn segment_argv(
    input: &Path,
    part: &Path,
    start: f64,
    dur: f64,
    copy: bool,
    has_video: bool,
    progress: bool,
) -> Argv {
    let mut argv = ffmpeg_base(progress);
    let ss = fmt_time(start);
    let td = fmt_time(dur);
    if start > 0.0 {
        argv.extend(["-ss", ss.as_str()]);
    }
    argv.push("-i");
    argv.push(input);
    argv.extend(["-t", td.as_str()]);
    if copy {
        argv.extend(["-c", "copy", "-avoid_negative_ts", "make_zero"]);
    } else if has_video {
        argv.extend([
            "-c:v", "libx264", "-preset", "veryfast", "-crf", "18", "-pix_fmt", "yuv420p", "-c:a",
            "aac",
        ]);
    } else {
        argv.extend(["-vn", "-c:a", "aac"]);
    }
    argv.push(part);
    argv
}

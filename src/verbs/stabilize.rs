use serde_json::json;

use crate::cli::{Globals, StabilizeArgs, StabilizeEngine};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: StabilizeArgs, g: &Globals) -> Result<Contract, Error> {
    if !(4..=64).contains(&args.rx) || !(4..=64).contains(&args.ry) {
        return Err(Error::input("--rx and --ry must be 4..=64"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "stabilize")?;

    // vidstab is the two-pass vid.stab stabilizer: pass 1 analyzes motion
    // into a .trf file, pass 2 renders the correction. Detected once, so a
    // tempfile holds the transform list between the two ffmpeg runs.
    if args.engine == Some(StabilizeEngine::Vidstab) {
        let smoothing = args.smoothing.unwrap_or(15);
        if !(1..=1000).contains(&smoothing) {
            return Err(Error::input("--smoothing must be 1..=1000"));
        }
        let trf = tempfile::Builder::new()
            .suffix(".trf")
            .tempfile()
            .map_err(|e| Error::output(e.to_string()))?;
        let esc = |p: &std::path::Path| {
            p.to_string_lossy()
                .replace('\\', "\\\\")
                .replace(':', "\\:")
        };
        let trf_path = esc(trf.path());
        let mut det = crate::spawn::Argv::ffmpeg();
        det.push("-i");
        det.push(&args.input);
        det.push("-vf");
        det.push(format!("vidstabdetect=result={trf_path}"));
        det.extend(["-f", "null", "-"]);
        let sp = crate::spawn::run(&det, g.timeout, false)?;
        crate::spawn::require_ok(&det, sp)?;

        let mut argv = ffmpeg_base(g.progress);
        argv.push("-i");
        argv.push(&args.input);
        argv.extend([
            "-vf",
            &format!("vidstabtransform=input={trf_path}:smoothing={smoothing}"),
            "-c:v",
            "libx264",
            "-preset",
            "fast",
            "-crf",
            "18",
            "-pix_fmt",
            "yuv420p",
        ]);
        if probe.has_audio {
            argv.extend(["-c:a", "copy"]);
        }
        argv.push(&args.output);
        let c = engine::write_job("stabilize", &[&args.input], &args.output, vec![argv], g)?;
        return Ok(c.with_extra(json!({
            "engine": "vidstab",
            "smoothing": smoothing,
        })));
    }

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend([
        "-vf",
        &format!(
            "deshake=rx={}:ry={}:edge={}",
            args.rx,
            args.ry,
            match args.edge {
                None | Some(crate::cli::StabilizeEdge::Mirror) => 3,
                Some(crate::cli::StabilizeEdge::Blank) => 0,
                Some(crate::cli::StabilizeEdge::Original) => 1,
                Some(crate::cli::StabilizeEdge::Clamped) => 2,
            }
        ),
        "-c:v",
        "libx264",
        "-preset",
        "fast",
        "-crf",
        "18",
        "-pix_fmt",
        "yuv420p",
    ]);
    if probe.has_audio {
        argv.extend(["-c:a", "copy"]);
    }
    argv.push(&args.output);

    let c = engine::write_job("stabilize", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "filter": "deshake",
        "rx": args.rx,
        "ry": args.ry,
    })))
}

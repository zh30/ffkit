use serde_json::json;

use crate::cli::{Globals, JumpcutArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: JumpcutArgs, g: &Globals) -> Result<Contract, Error> {
    if args.min_duration <= 0.0 || args.min_duration > 30.0 {
        return Err(Error::input("--min-duration must be in (0, 30] seconds"));
    }
    if args.pad < 0.0 || args.pad > args.min_duration {
        return Err(Error::input("--pad must be >= 0 and <= --min-duration"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_audio {
        return Err(Error::input(
            "jumpcut needs an audio stream to find silence",
        ));
    }

    let silences = if g.dry_run {
        Vec::new()
    } else {
        crate::silence::detect(
            &args.input,
            args.threshold,
            args.min_duration,
            g.timeout,
            false,
        )?
    };
    let keeps = crate::silence::keep_ranges(probe.duration, &silences, args.pad);
    if keeps.is_empty() {
        return Err(Error::input(
            "jumpcut: clip is all silence at this threshold",
        ));
    }

    let kept: f64 = keeps.iter().map(|(s, e)| e - s).sum();
    let removed = (probe.duration - kept).max(0.0);

    if keeps.len() == 1 && (keeps[0].0) < 0.02 && (probe.duration - keeps[0].1) < 0.02 {
        let mut argv = ffmpeg_base(g.progress);
        argv.push("-i");
        argv.push(&args.input);
        argv.extend(["-c", "copy"]);
        argv.push(&args.output);
        let c = engine::write_job("jumpcut", &[&args.input], &args.output, vec![argv], g)?;
        return Ok(c.with_extra(json!({
            "cuts": 0,
            "removed_seconds": 0.0,
            "kept_seconds": probe.duration,
        })));
    }

    let fc = concat_graph(&keeps, probe.has_video);
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-filter_complex", &fc]);
    if probe.has_video {
        argv.extend([
            "-map", "[vout]", "-c:v", "libx264", "-preset", "fast", "-crf", "18",
        ]);
    }
    argv.extend(["-map", "[aout]", "-c:a", "aac"]);
    argv.push(&args.output);

    let c = engine::write_job("jumpcut", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "cuts": keeps.len().saturating_sub(1),
        "removed_seconds": removed,
        "kept_seconds": kept,
        "silences": silences.len(),
    })))
}

fn concat_graph(keeps: &[(f64, f64)], has_video: bool) -> String {
    let n = keeps.len();
    let mut fc = String::new();
    let mut cat = String::new();
    for (i, (s, e)) in keeps.iter().enumerate() {
        if has_video {
            fc.push_str(&format!(
                "[0:v]trim=start={s}:end={e},setpts=PTS-STARTPTS[v{i}];"
            ));
            cat.push_str(&format!("[v{i}]"));
        }
        fc.push_str(&format!(
            "[0:a]atrim=start={s}:end={e},asetpts=PTS-STARTPTS[a{i}];"
        ));
        cat.push_str(&format!("[a{i}]"));
    }
    if has_video {
        fc.push_str(&format!("{cat}concat=n={n}:v=1:a=1[vout][aout]"));
    } else {
        fc.push_str(&format!("{cat}concat=n={n}:v=0:a=1[aout]"));
    }
    fc
}

#[cfg(test)]
mod tests {
    #[test]
    fn parse_pair() {
        let log = "\
[silencedetect @ 0x] silence_start: 0.4
[silencedetect @ 0x] silence_end: 0.9 | silence_duration: 0.5
";
        let s = crate::silence::parse_silences(log, 1.2);
        assert_eq!(s, vec![(0.4, 0.9)]);
    }

    #[test]
    fn keep_splits_middle_silence() {
        let k = crate::silence::keep_ranges(1.2, &[(0.4, 0.9)], 0.05);
        assert_eq!(k.len(), 2);
        assert!((k[0].1 - 0.45).abs() < 1e-6);
        assert!((k[1].0 - 0.85).abs() < 1e-6);
    }
}

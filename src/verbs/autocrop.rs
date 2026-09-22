use serde_json::json;

use crate::cli::{AutocropArgs, Globals};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::spawn;

pub fn run(args: AutocropArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "autocrop")?;
    let iw = probe.width.unwrap_or(0);
    let ih = probe.height.unwrap_or(0);
    if iw == 0 || ih == 0 {
        return Err(Error::input("autocrop: input has no frame size"));
    }
    // Scan up to the first minute with cropdetect; the last line wins
    // (cropdetect refines its box as it sees more frames).
    let scan = probe.duration.clamp(1.0, 60.0);
    let scan_s = format!("{scan:.2}");
    // cropdetect reports on stderr at info level — the shared ffmpeg_base
    // pins -loglevel error, so this probe argv is built without it.
    let mut probe_argv = crate::spawn::Argv::ffmpeg();
    probe_argv.extend(["-y", "-hide_banner", "-nostats", "-loglevel", "info"]);
    probe_argv.extend(["-t", &scan_s, "-i"]);
    probe_argv.push(&args.input);
    probe_argv.extend(["-vf", "cropdetect", "-f", "null", "-"]);
    let spawned = spawn::run(&probe_argv, std::time::Duration::from_secs(300), g.progress)?;
    let spawned = spawn::require_ok(&probe_argv, spawned)?;
    let log = spawn::stderr_str(&spawned);
    let mut best: Option<(u32, u32, u32, u32)> = None;
    for line in log.lines() {
        if let Some(i) = line.rfind("crop=") {
            let spec: Vec<u32> = line[i + 5..]
                .split(|c: char| !c.is_ascii_digit())
                .filter(|s| !s.is_empty())
                .take(4)
                .filter_map(|s| s.parse().ok())
                .collect();
            if spec.len() == 4 {
                best = Some((spec[0], spec[1], spec[2], spec[3]));
            }
        }
    }
    let (cw, ch, cx, cy) = match best {
        Some(v) => v,
        None => {
            return Err(Error::output(
                "autocrop: cropdetect produced no measurement",
            ))
        }
    };
    if (cw, ch) == (iw, ih) {
        return Err(Error::input(
            "autocrop: no black bars detected — output would be identical",
        ));
    }
    if cw < 8 || ch < 8 {
        return Err(Error::output(format!(
            "autocrop: detected crop {cw}x{ch} is degenerate"
        )));
    }
    // --buffer: grow the box back out, clamped to the frame
    let (cw, ch, cx, cy) = {
        let b = args.buffer.max(0);
        let (x, y) = ((cx as i64 - b).max(0), (cy as i64 - b).max(0));
        (
            (cw as i64 + 2 * b).min(iw as i64 - x) as u32,
            (ch as i64 + 2 * b).min(ih as i64 - y) as u32,
            x as u32,
            y as u32,
        )
    };
    let vf = format!("crop={cw}:{ch}:{cx}:{cy},setsar=1");
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-vf", &vf, "-map", "0:v", "-map", "0:a?"]);
    if probe.has_audio {
        argv.extend(["-c:a", "copy"]);
    }
    argv.extend([
        "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
    ]);
    argv.push(&args.output);

    let c = engine::write_job("autocrop", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "detected": { "w": cw, "h": ch, "x": cx, "y": cy },
        "buffer": args.buffer,
        "source": { "w": iw, "h": ih },
    })))
}

use serde_json::json;

use crate::cli::{CompressArgs, Globals};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

/// Under ~64 kbps the picture turns to mush; refuse instead of writing garbage.
const MIN_VIDEO_BPS: f64 = 64_000.0;
/// Container + moov overhead reserve so the encode lands under target.
const MUX_RESERVE: f64 = 0.98;
/// Uncompressed PCM outputs ignore -b:a; refuse them outright.
const PCM_EXTS: &[&str] = &["wav", "aif", "aiff", "caf", "flac"];

pub fn run(args: CompressArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    if let Some(crf) = args.crf {
        if crf > 51 {
            return Err(Error::input("--crf must be 0..=51"));
        }
        if !probe.has_video {
            return Err(Error::input("compress --crf is video-only"));
        }
        let vf = match args.res {
            Some(h) => format!(
                "scale=-2:{h}:force_original_aspect_ratio=decrease,scale=trunc(iw/2)*2:trunc(ih/2)*2,format=yuv420p"
            ),
            None => "scale=trunc(iw/2)*2:trunc(ih/2)*2,format=yuv420p".to_string(),
        };
        let mut argv = ffmpeg_base(g.progress);
        argv.push("-i");
        argv.push(&args.input);
        argv.extend(["-vf", &vf, "-c:v", "libx264", "-crf", &crf.to_string()]);
        if probe.has_audio {
            argv.extend([
                "-c:a",
                "aac",
                "-b:a",
                &format!("{:.0}", args.audio_kbps * 1_000.0),
            ]);
        } else {
            argv.push("-an");
        }
        argv.extend(["-movflags", "+faststart"]);
        argv.push(&args.output);
        let c = engine::write_job("compress", &[&args.input], &args.output, vec![argv], g)?;
        return Ok(c.with_extra(json!({ "crf": crf, "passes": 1 })));
    }
    let size_str = match &args.size {
        Some(s) => s.clone(),
        None => match args.target {
            Some(crate::cli::CompressTarget::Discord) => "8MB".to_string(),
            Some(crate::cli::CompressTarget::Whatsapp) => "16MB".to_string(),
            Some(crate::cli::CompressTarget::Gmail) => "25MB".to_string(),
            None => {
                return Err(Error::input("compress needs --size or --target"));
            }
        },
    };
    let target = parse_size(&size_str)?;
    if !(8.0..=512.0).contains(&args.audio_kbps) {
        return Err(Error::input("--audio-kbps must be 8–512"));
    }
    if probe.duration <= 0.0 || !probe.duration.is_finite() {
        return Err(Error::input(format!(
            "compress: cannot budget bitrate without a duration ({})",
            args.input.display()
        )));
    }
    if !probe.has_video && !probe.has_audio {
        return Err(Error::input("compress: input has no media stream"));
    }
    if !probe.has_video && is_pcm_output(&args.output) {
        return Err(Error::input(
            "compress: wav/flac outputs ignore bitrate; export aac/m4a/mp3/opus",
        ));
    }

    // Size is bitrate × duration. Reserve 2% for the container, pay the audio
    // stream first, and the rest is the video budget.
    let usable_bps = (target as f64 * 8.0 * MUX_RESERVE) / probe.duration;
    let audio_bps = if probe.has_audio {
        (args.audio_kbps * 1_000.0).min(usable_bps * 0.8)
    } else {
        0.0
    };
    let video_bps = usable_bps - audio_bps;

    if probe.has_video && video_bps < MIN_VIDEO_BPS {
        return Err(Error::input(format!(
            "--size {} cannot fit {:.1}s of video under 64 kbps; raise --size or cut first",
            size_str, probe.duration
        )));
    }
    if !probe.has_video && audio_bps < 16_000.0 {
        return Err(Error::input(format!(
            "--size {} cannot fit {:.1}s of audio; raise --size",
            size_str, probe.duration
        )));
    }

    let scale_vf = match args.res {
        Some(h) => format!(
            "scale=-2:{h}:force_original_aspect_ratio=decrease,scale=trunc(iw/2)*2:trunc(ih/2)*2,format=yuv420p"
        ),
        None => "scale=trunc(iw/2)*2:trunc(ih/2)*2,format=yuv420p".to_string(),
    };
    let mut argvs = Vec::new();
    let mut video_kbps = 0.0;
    // Two-pass stats live in a tempdir that must outlive write_job's ffmpeg run.
    let mut _passlog_dir = None;
    if probe.has_video {
        video_kbps = video_bps / 1_000.0;
        // Two-pass: pass 1 maps frame complexity, pass 2 spends the budget.
        // -f null - keeps the sink portable (no /dev/null or NUL).
        let dir =
            tempfile::tempdir().map_err(|e| Error::output(format!("passlog tempdir: {e}")))?;
        let passlog = dir.path().join("pass");
        _passlog_dir = Some(dir);

        let mut pass1 = ffmpeg_base(g.progress);
        pass1.push("-i");
        pass1.push(&args.input);
        pass1.extend([
            "-c:v",
            "libx264",
            "-b:v",
            &format!("{video_bps:.0}"),
            "-pass",
            "1",
            "-passlogfile",
        ]);
        pass1.push(&passlog);
        pass1.extend(["-an", "-f", "null", "-"]);

        let mut pass2 = ffmpeg_base(g.progress);
        pass2.push("-i");
        pass2.push(&args.input);
        pass2.extend([
            "-vf",
            &scale_vf,
            "-c:v",
            "libx264",
            "-b:v",
            &format!("{video_bps:.0}"),
            "-pass",
            "2",
            "-passlogfile",
        ]);
        pass2.push(&passlog);
        if probe.has_audio {
            pass2.extend(["-c:a", "aac", "-b:a", &format!("{:.0}", audio_bps)]);
        } else {
            pass2.push("-an");
        }
        pass2.extend(["-movflags", "+faststart"]);
        pass2.push(&args.output);

        argvs.push(pass1);
        argvs.push(pass2);
    } else {
        // Audio-only: single pass at the computed bitrate.
        let mut argv = ffmpeg_base(g.progress);
        argv.push("-i");
        argv.push(&args.input);
        argv.extend(["-vn", "-b:a", &format!("{audio_bps:.0}")]);
        argv.push(&args.output);
        argvs.push(argv);
    }

    let mut c = engine::write_job("compress", &[&args.input], &args.output, argvs, g)?;
    c = c.with_extra(json!({
        "target_bytes": target,
        "video_kbps": video_kbps,
        "audio_kbps": audio_bps / 1_000.0,
        "passes": if probe.has_video { 2 } else { 1 },
        "res": args.res,
    }));
    Ok(c)
}

/// "10MB", "800KB", "1.5GB" (decimal SI); a bare number is MB.
pub(crate) fn parse_size(s: &str) -> Result<u64, Error> {
    let t = s.trim();
    let (num, mult) = if let Some(n) = t.strip_suffix(['B', 'b']) {
        match n.strip_suffix(['K', 'k', 'M', 'G']) {
            Some(m) => (m, unit(n)),
            None => (n, 1.0),
        }
    } else if let Some(m) = t.strip_suffix(['K', 'k', 'M', 'G']) {
        (m, unit(t))
    } else {
        (t, 1_000_000.0)
    };
    let v: f64 = num
        .trim()
        .parse()
        .map_err(|_| Error::input(format!("invalid --size {s}; try 10MB")))?;
    let bytes = v * mult;
    if bytes < 1_000.0 {
        return Err(Error::input(format!(
            "--size {s} is too small to hold media"
        )));
    }
    Ok(bytes as u64)
}

fn unit(s: &str) -> f64 {
    let c = s.chars().last().unwrap_or('M').to_ascii_uppercase();
    match c {
        'K' => 1_000.0,
        'M' => 1_000_000.0,
        'G' => 1_000_000_000.0,
        _ => 1_000_000.0,
    }
}

fn is_pcm_output(output: &std::path::Path) -> bool {
    output
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| PCM_EXTS.iter().any(|x| x.eq_ignore_ascii_case(e)))
        .unwrap_or(false)
}

use serde_json::json;

use crate::cli::{DenoiseArgs, Globals};
use crate::contract::Contract;
use crate::doctor;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: DenoiseArgs, g: &Globals) -> Result<Contract, Error> {
    if !(0.0..=1.0).contains(&args.strength) {
        return Err(Error::input("--strength must be between 0 and 1"));
    }
    if !(0.0..=500.0).contains(&args.highpass) {
        return Err(Error::input("--highpass must be 0–500 Hz (0 disables)"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_audio {
        return Err(Error::input("denoise: input has no audio stream"));
    }

    // strength 0–1 → afwtdn sigma 0.02–0.08. afwtdn over afftdn: in ffmpeg
    // 9.x afftdn with default nf=-50 extracts almost nothing (measured ~0 dB
    // hiss reduction); a small fixed wavelet sigma reliably takes ~16 dB off
    // broadband noise for <3 dB voice cost across input levels. anlmdn is
    // stronger on paper but segfaults in this build. afwtdn only exists in
    // ffmpeg ≥5.1 — older builds fall back to afftdn with the noise floor
    // raised (nf=-20, ~12 dB measured).
    let mut af = if doctor::list_filters()
        .map(|f| f.contains("afwtdn"))
        .unwrap_or(false)
    {
        let sigma = 0.02 + 0.06 * args.strength;
        format!("afwtdn=sigma={sigma:.3}")
    } else {
        let nr = 6.0 + 12.0 * args.strength;
        format!("afftdn=nr={nr:.1}:nf=-20")
    };
    if args.highpass > 0.0 {
        af = format!("highpass=f={:.0},{af}", args.highpass);
    }

    // --at/--dur: windowed denoise via the shared dry/wet splitter
    let fc = match &args.at {
        Some(raw) => {
            let at = crate::time::parse_time(raw)?;
            if !(0.0..probe.duration).contains(&at) {
                return Err(Error::input("--at is outside the input"));
            }
            Some(engine::audio_window(&af, at, args.dur))
        }
        None => {
            if args.dur.is_some() {
                return Err(Error::input("--dur needs --at"));
            }
            None
        }
    };

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    if let Some(fc) = &fc {
        let chain = if args.video && probe.has_video {
            format!("{fc};[0:v]hqdn3d[vout]")
        } else {
            fc.clone()
        };
        argv.extend(["-filter_complex", &chain]);
        if args.video && probe.has_video {
            argv.extend(["-map", "[vout]"]);
        } else if probe.has_video {
            argv.extend(["-map", "0:v"]);
        }
        argv.extend(["-map", "[aout]"]);
        if args.video && probe.has_video {
            argv.extend(["-c:v", "libx264", "-preset", "fast", "-crf", "18"]);
        } else if probe.has_video {
            argv.extend(["-c:v", "copy"]);
        }
    } else if args.video && probe.has_video {
        // Degrain too: hqdn3d needs a real encode, not stream copy.
        argv.extend([
            "-filter_complex",
            &format!("[0:a]{af}[aout];[0:v]hqdn3d[vout]"),
            "-map",
            "[vout]",
            "-map",
            "[aout]",
            "-c:v",
            "libx264",
            "-preset",
            "fast",
            "-crf",
            "18",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
        ]);
    } else {
        argv.extend(["-af", &af]);
        if probe.has_video {
            argv.extend(["-c:v", "copy"]);
        }
    }
    argv.extend(["-c:a", "aac"]);
    argv.push(&args.output);

    let mut c = engine::write_job("denoise", &[&args.input], &args.output, vec![argv], g)?;
    c = c.with_extra(json!({
        "af": af,
        "video_denoise": args.video && probe.has_video,
    }));
    Ok(c)
}

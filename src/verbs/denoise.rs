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
    if let Some(r) = &args.ref_ {
        // anlms learns ref→mix, i.e. estimates the noise component inside the
        // mix: feed the reference as input 0, the noisy mix as input 1, then
        // subtract the estimate. (The naive "voice in 0, ref in 1" reading of
        // the docs destroys the voice — the filter cancels everything it can.)
        if args.at.is_some() || args.dur.is_some() {
            return Err(Error::input("denoise --ref can't be windowed (--at/--dur)"));
        }
        let rprobe = engine::probe_or_err(r, g)?;
        if !rprobe.has_audio {
            return Err(Error::input("denoise --ref: reference has no audio stream"));
        }
        let order = (64.0 + 448.0 * args.strength) as u32;
        let mut argv = ffmpeg_base(g.progress);
        argv.push("-i");
        argv.push(&args.input);
        argv.push("-i");
        argv.push(r);
        let fc = format!(
            "[1:a][0:a]anlms=order={order}:mu=0.3:eps=0.0001[e];[e]volume=-1[neg];[0:a][neg]amix=inputs=2[a]"
        );
        argv.extend(["-filter_complex", &fc]);
        argv.extend(["-map", "[a]"]);
        if probe.has_video {
            argv.extend(["-map", "0:v", "-c:v", "copy"]);
        }
        argv.extend(["-c:a", "aac", "-b:a", "192k"]);
        argv.push(&args.output);
        let c = engine::write_job("denoise", &[&args.input, r], &args.output, vec![argv], g)?;
        return Ok(c.with_extra(json!({ "ref": true, "order": order })));
    }
    let has_wavel = doctor::list_filters()
        .map(|f| f.contains("afwtdn"))
        .unwrap_or(false);
    let use_wavel = match args.engine {
        crate::cli::DenoiseEngine::Auto => has_wavel,
        crate::cli::DenoiseEngine::Wavel => {
            if !has_wavel {
                return Err(Error::input(
                    "denoise --engine wavel needs afwtdn (ffmpeg ≥5.1; this build lacks it)",
                ));
            }
            true
        }
        crate::cli::DenoiseEngine::Fftdn => false,
    };
    let mut af = if use_wavel {
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
        Some(raw) => Some(engine::audio_window_for(
            &af,
            raw,
            args.dur,
            probe.duration,
        )?),
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

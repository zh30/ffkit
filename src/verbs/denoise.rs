use serde_json::json;

use crate::cli::{DenoiseArgs, Globals};
use crate::contract::Contract;
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
    // stronger on paper but segfaults in this build.
    let sigma = 0.02 + 0.06 * args.strength;
    let mut af = format!("afwtdn=sigma={sigma:.3}");
    if args.highpass > 0.0 {
        af = format!("highpass=f={:.0},{af}", args.highpass);
    }

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    if args.video && probe.has_video {
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
    argv.push(&args.output);

    let mut c = engine::write_job("denoise", &[&args.input], &args.output, vec![argv], g)?;
    c = c.with_extra(json!({
        "af": af,
        "video_denoise": args.video && probe.has_video,
    }));
    Ok(c)
}

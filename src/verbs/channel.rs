use serde_json::json;

use std::path::Path;

use crate::cli::{ChannelArgs, ChannelMode, Globals};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: ChannelArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_audio {
        return Err(Error::input("channel needs an audio stream"));
    }
    if args.mode == ChannelMode::Split {
        let stem = args
            .output
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("out");
        let dir = args.output.parent().unwrap_or_else(|| Path::new("."));
        let l = dir.join(format!("{stem}_L.wav"));
        let r = dir.join(format!("{stem}_R.wav"));
        for p in [&l, &r] {
            crate::paths::ensure_output_allowed(p, &[&args.input], g.overwrite)?;
        }
        let fc = "[0:a]channelsplit=channel_layout=stereo[FL][FR]";
        let mut argv = ffmpeg_base(g.progress);
        argv.push("-i");
        argv.push(&args.input);
        argv.extend(["-filter_complex", fc]);
        argv.extend(["-map", "[FL]", "-c:a", "pcm_s16le"]);
        argv.push(&l);
        argv.extend(["-map", "[FR]", "-c:a", "pcm_s16le"]);
        argv.push(&r);
        let c = engine::write_job("channel", &[&args.input], &l, vec![argv], g)?;
        if !g.dry_run && !r.exists() {
            return Err(Error::verification(format!(
                "output was not written: {}",
                r.display()
            )));
        }
        return Ok(c.with_extra(json!({
            "mode": "Split",
            "outputs": [l.display().to_string(), r.display().to_string()],
        })));
    }
    let af = match args.mode {
        // single-mic voice recorded on one ear → copy ch0 to all
        ChannelMode::Dualmono => "pan=stereo|FL<c0|FR<c0".to_string(),
        ChannelMode::Mono => "aformat=channel_layouts=mono".to_string(),
        ChannelMode::Swap => "channelmap=map=FR-FL|FL-FR:channel_layout=stereo".to_string(),
        // ITU fold-down: dialogue keeps center, surrounds fold at 0.707
        ChannelMode::Widen => "extrastereo=m=2.5".to_string(),
        // pan p>0 fades the left side into the right ear and vice-versa
        ChannelMode::Pan => {
            let p = args.pan.unwrap_or(0.5);
            if !(-1.0..=1.0).contains(&p) {
                return Err(Error::input("--pan must be -1..=1"));
            }
            let fl = 1.0 - p.max(0.0);
            let fr = 1.0 - (-p).max(0.0);
            format!("pan=stereo|FL<{fl:.3}*FL|FR<{fr:.3}*FR")
        }
        ChannelMode::Mix51 => {
            "pan=stereo|FL<FL+0.707*FC+0.707*BL+0.5*LFE|FR<FR+0.707*FC+0.707*BR+0.5*LFE".to_string()
        }
        ChannelMode::Invert => match args.side.as_deref().unwrap_or("both") {
            "left" => "aeval='-val(0)|val(1)':c=stereo,aformat=channel_layouts=stereo".to_string(),
            "right" => "aeval='val(0)|-val(1)':c=stereo,aformat=channel_layouts=stereo".to_string(),
            "both" => "aeval='-val(0)|-val(1)':c=stereo,aformat=channel_layouts=stereo".to_string(),
            other => {
                return Err(Error::input(format!(
                    "--side must be left|right|both (got {other})"
                )))
            }
        },
        // stereotools side-level cut: drops room echo / wide ambience so the
        // center voice sits drier — amount is the side keep-ratio 0..1
        ChannelMode::Ambience => {
            let a = args.amount.unwrap_or(0.5);
            if !(0.0..=1.0).contains(&a) {
                return Err(Error::input("--amount must be 0..1"));
            }
            format!("stereotools=slev={a:.3}")
        }
        // M/S decode: mid = (L+R)/2 per ear, side = ±(L−R)/2 — polarity
        // flips on the right ear, which is what makes side ambience-only
        ChannelMode::Mid => "pan=stereo|FL<0.5*FL+0.5*FR|FR<0.5*FL+0.5*FR".to_string(),
        ChannelMode::Side => "pan=stereo|FL<0.5*FL-0.5*FR|FR<-0.5*FL+0.5*FR".to_string(),
        // Haas effect: micro L/R delays + polarity flip — mono-safe width.
        // --amount scales the side gain 0.5..3.0 (default 1.75)
        ChannelMode::Haas => {
            let a = args.amount.unwrap_or(0.5);
            if !(0.0..=1.0).contains(&a) {
                return Err(Error::input("--amount must be 0..1"));
            }
            format!("haas=side_gain={:.2}", 0.5 + 2.5 * a)
        }
        // Stereo→5.1 soundfield upmix: derived surround + LFE (aac takes 5.1)
        ChannelMode::Surround => "surround=chl_out=5.1".to_string(),
        // stereotools stereo base: -1 folds toward mono (fixes over-wide
        // recordings / stereo-phase issues), +1 exaggerates width
        ChannelMode::Ms => "stereotools=mode=ms>lr".to_string(),
        // balance correction: stereotools balance_in attenuates the hot side —
        // --pan -1 (fix left-heavy) .. 1 (fix right-heavy). Verified: pan +0.5
        // drops the left channel ~6dB, swinging the image right
        ChannelMode::Bal => {
            let b = args.pan.unwrap_or(0.0).clamp(-1.0, 1.0);
            format!("stereotools=balance_in={b:.3}")
        }
        ChannelMode::Base => {
            let p = args.pan.unwrap_or(0.5);
            if !(-1.0..=1.0).contains(&p) {
                return Err(Error::input("--pan must be -1..=1"));
            }
            format!("stereotools=base={p:.3}")
        }
        ChannelMode::Split => unreachable!("split returns early"),
    };
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-af", &af, "-map", "0:v?", "-map", "0:a"]);
    if probe.has_video {
        argv.extend(["-c:v", "copy"]);
    }
    argv.extend(["-c:a", "aac"]);
    argv.push(&args.output);

    let c = engine::write_job("channel", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "mode": format!("{:?}", args.mode),
    })))
}

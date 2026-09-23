use serde_json::json;

use crate::cli::{Globals, MusicArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: MusicArgs, g: &Globals) -> Result<Contract, Error> {
    if !(0.05..=1.0).contains(&args.gain) {
        return Err(Error::input("--gain must be between 0.05 and 1.0"));
    }
    let talk = engine::probe_or_err(&args.input, g)?;
    crate::paths::ensure_input(&args.track)?;

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-stream_loop", "-1", "-i"]);
    argv.push(&args.track);

    // sidechaincompress only accepts packed dbl. Pin *immediately* before it:
    // `volume` after aformat would convert away from dbl, and Ubuntu/apt ffmpeg
    // will not insert the converter (Homebrew 9 does).
    // --fade: afade in/out on the bed itself (fade-out end = the talk's length).
    let mut ats = Vec::new();
    match &args.at {
        Some(s) => {
            for part in s.split(',') {
                ats.push(crate::time::resolve_at(
                    part.trim(),
                    args.dur,
                    talk.duration,
                )?);
            }
        }
        None => ats.push(0.0),
    };
    ats.sort_by(|a, b| a.partial_cmp(b).unwrap());
    for &at in &ats {
        if at < 0.0 || at >= talk.duration {
            return Err(Error::input("--at is outside the input"));
        }
    }
    let at = ats[0];
    let win = match args.dur {
        Some(d) if d > 0.0 => d.min(talk.duration - at),
        _ => talk.duration - at,
    };
    let mut bed_pre = String::new();
    if let Some(d) = args.dur {
        bed_pre.push_str(&format!("atrim=duration={d:.3},asetpts=PTS-STARTPTS,"));
    }
    let fade_of = |w: f64| -> String {
        if args.fade > 0.0 {
            let f = args.fade.min(w / 2.0).max(0.05);
            format!(
                ",afade=t=in:st=0:d={f:.3},afade=t=out:st={:.3}:d={f:.3}",
                w - f
            )
        } else {
            String::new()
        }
    };
    let fade = fade_of(win);
    let delay = if at > 0.0 {
        format!(",adelay={:.0}:all=1", at * 1000.0)
    } else {
        String::new()
    };
    const AF: &str = "aformat=sample_fmts=dbl:sample_rates=48000:channel_layouts=stereo";
    // Multi-entrance bed (intro + outro stings): one wet branch per --at point,
    // amixed into a single bed before the duck/mix stage.
    let wet_chain = |fc: &mut String, n: usize| {
        let mut spl = String::from("[1:a]asplit=");
        spl.push_str(&n.to_string());
        for i in 0..n {
            spl.push_str(&format!("[sp{i}]"));
        }
        fc.push_str(&spl);
        for (i, &s) in ats.iter().enumerate() {
            let w = match args.dur {
                Some(d) if d > 0.0 => d.min(talk.duration - s),
                _ => talk.duration - s,
            };
            let d = if s > 0.0 {
                format!(",adelay={:.0}:all=1", s * 1000.0)
            } else {
                String::new()
            };
            fc.push_str(&format!(
                ";[sp{i}]{bed_pre}volume={gain}{}{d}[bg{i}]",
                fade_of(w),
                gain = args.gain
            ));
        }
    };
    let fc = if ats.len() == 1 {
        if talk.has_audio && args.duck {
            format!(
                "[1:a]{bed_pre}volume={gain}{fade}{delay}[bgraw];[0:a]asplit=2[voice][scraw];[bgraw]{AF}[bg];[scraw]{AF}[sc];[bg][sc]sidechaincompress=threshold=0.05:ratio=6:attack=20:release=250[dk];[voice][dk]amix=inputs=2:duration=first:dropout_transition=0:normalize=0[aout]",
                gain = args.gain,
                bed_pre = bed_pre,
                delay = delay,
            )
        } else if talk.has_audio {
            format!(
                "[1:a]{bed_pre}volume={gain}{fade}{delay}[bg];[0:a][bg]amix=inputs=2:duration=first:dropout_transition=0:normalize=0[aout]",
                gain = args.gain,
                bed_pre = bed_pre,
                delay = delay,
            )
        } else {
            format!("[1:a]{bed_pre}volume={}{fade}{delay}[aout]", args.gain)
        }
    } else {
        let n = ats.len();
        let mut fc = String::new();
        wet_chain(&mut fc, n);
        let mut wet = String::new();
        for i in 0..n {
            wet.push_str(&format!("[bg{i}]"));
        }
        if talk.has_audio && args.duck {
            fc.push_str(&format!(
                ";{wet}amix=inputs={n}:duration=longest:normalize=0[bgraw];[0:a]asplit=2[voice][scraw];[bgraw]{AF}[bgm];[scraw]{AF}[sc];[bgm][sc]sidechaincompress=threshold=0.05:ratio=6:attack=20:release=250[dk];[voice][dk]amix=inputs=2:duration=first:dropout_transition=0:normalize=0[aout]"
            ));
        } else if talk.has_audio {
            fc.push_str(&format!(
                ";[0:a]{wet}amix=inputs={}:duration=first:dropout_transition=0:normalize=0[aout]",
                n + 1
            ));
        } else {
            fc.push_str(&format!(
                ";{wet}amix=inputs={n}:duration=longest:normalize=0[aout]"
            ));
        }
        fc
    };
    argv.extend(["-filter_complex", &fc, "-map", "[aout]", "-c:a", "aac"]);
    if talk.has_video {
        argv.extend(["-map", "0:v", "-c:v", "copy"]);
    }
    argv.push("-shortest");
    argv.push(&args.output);

    let mut c = engine::write_job(
        "music",
        &[&args.input, &args.track],
        &args.output,
        vec![argv],
        g,
    )?;
    c = c.with_extra(json!({
        "gain": args.gain,
        "duck": args.duck && talk.has_audio,
    }));
    Ok(c)
}

use serde_json::json;
use std::path::Path;

use crate::cli::{Globals, MixArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::paths;

pub fn run(args: MixArgs, g: &Globals) -> Result<Contract, Error> {
    let pa = engine::probe_or_err(&args.a, g)?;
    let pb = engine::probe_or_err(&args.b, g)?;
    for (p, path) in [(&pa, &args.a), (&pb, &args.b)] {
        if !p.has_audio {
            return Err(Error::input(format!(
                "`{}` has no audio stream — mix merges audio",
                paths::display(path)
            )));
        }
    }
    if !(0.0..=4.0).contains(&args.vol_a) || !(0.0..=4.0).contains(&args.vol_b) {
        return Err(Error::input("--vol must be 0..=4 (linear scale)"));
    }
    let dur = if args.longest { "longest" } else { "first" };

    // --at/--dur: gate the B track into the window (outside it, A alone).
    let gate = match &args.at {
        Some(raw) => {
            let at = crate::time::parse_time(raw)?;
            if !(0.0..pa.duration).contains(&at) {
                return Err(Error::input("--at is outside the A input"));
            }
            let end = args.dur.map(|d| at + d).unwrap_or(pa.duration);
            let fade = match args.fade {
                Some(f) if f > 0.0 => {
                    let f = f.min((end - at) / 2.0);
                    format!(
                        ",afade=t=in:st={at:.3}:d={f:.3},afade=t=out:st={:.3}:d={f:.3}",
                        end - f
                    )
                }
                _ => String::new(),
            };
            format!(",volume='between(t,{at:.3},{end:.3})':eval=frame{fade}")
        }
        None => {
            if args.dur.is_some() {
                return Err(Error::input("--dur needs --at"));
            }
            String::new()
        }
    };

    let (split, b1, bed) = if args.duck {
        // sidechain: the bed (B) compresses whenever the voice (A) is loud —
        // ffmpeg <7 needs an explicit asplit to feed A to both the key and the mix
        (
            ",asplit=2[a0][a0sc]",
            "[a1pre]",
            ";[a1pre]aformat=sample_fmts=fltp:channel_layouts=stereo[a1f];[a0sc]aformat=sample_fmts=fltp:channel_layouts=stereo[a0scf];[a1f][a0scf]sidechaincompress=threshold=0.02:ratio=8:attack=25:release=350:makeup=1[a1]",
        )
    } else {
        ("[a0]", "[a1]", "")
    };
    let norm = if args.normalize { 1 } else { 0 };
    let fc = format!(
        "[0:a]aresample=48000,volume={:.4}{split};\
         [1:a]aresample=48000,volume={:.4}{gate}{b1}{bed};\
         [a0][a1]amix=inputs=2:duration={dur}:normalize={norm}[aout]",
        args.vol_a, args.vol_b
    );

    let mut argv = ffmpeg_base(g.progress);
    argv.extend(["-i".to_string(), args.a.display().to_string()]);
    if args.loop_track {
        argv.extend(["-stream_loop".to_string(), "-1".to_string()]);
    }
    argv.extend(["-i".to_string(), args.b.display().to_string()]);
    argv.extend(["-filter_complex".to_string(), fc]);
    if pa.has_video {
        argv.extend(["-map".to_string(), "0:v".to_string()]);
        argv.extend(["-c:v".to_string(), "copy".to_string()]);
    }
    argv.extend(["-map".to_string(), "[aout]".to_string()]);
    if pa.has_video {
        argv.extend([
            "-c:a".to_string(),
            "aac".to_string(),
            "-b:a".to_string(),
            "192k".to_string(),
        ]);
    }
    argv.push(args.output.display().to_string());

    let inputs: Vec<&Path> = vec![&args.a, &args.b];
    let mut c = engine::write_job("mix", &inputs, &args.output, vec![argv], g)?;
    c = c.with_extra(json!({
        "duration_mode": dur,
        "vol_a": args.vol_a,
        "vol_b": args.vol_b,
        "kept_video": pa.has_video,
    }));
    Ok(c)
}

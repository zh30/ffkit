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

    let fc = if talk.has_audio && args.duck {
        format!(
            "[1:a]volume={}[bg];[0:a]asplit=2[voice][sc];[bg][sc]sidechaincompress=threshold=0.05:ratio=6:attack=20:release=250[dk];[voice][dk]amix=inputs=2:duration=first:dropout_transition=0:normalize=0[aout]",
            args.gain
        )
    } else if talk.has_audio {
        format!(
            "[1:a]volume={}[bg];[0:a][bg]amix=inputs=2:duration=first:dropout_transition=0:normalize=0[aout]",
            args.gain
        )
    } else {
        format!("[1:a]volume={}[aout]", args.gain)
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

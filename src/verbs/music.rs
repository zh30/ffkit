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

    // sidechaincompress only accepts dbl. Ubuntu/apt ffmpeg will not auto-insert
    // aformat (Homebrew 9 does), so pin both legs or the graph fails to negotiate.
    const AF: &str = "aformat=sample_fmts=dbl:sample_rates=48000:channel_layouts=stereo";
    let fc = if talk.has_audio && args.duck {
        format!(
            "[1:a]{AF},volume={gain}[bg];[0:a]{AF},asplit=2[voice][sc];[bg][sc]sidechaincompress=threshold=0.05:ratio=6:attack=20:release=250[dk];[voice][dk]amix=inputs=2:duration=first:dropout_transition=0:normalize=0[aout]",
            gain = args.gain
        )
    } else if talk.has_audio {
        format!(
            "[1:a]{AF},volume={gain}[bg];[0:a]{AF}[voice];[voice][bg]amix=inputs=2:duration=first:dropout_transition=0:normalize=0[aout]",
            gain = args.gain
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

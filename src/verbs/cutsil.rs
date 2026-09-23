use serde_json::json;

use crate::cli::{CutsilArgs, Globals};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

/// Strip dead air at the head AND tail: `stop_periods=-1` makes
/// silenceremove count silence from the end of the stream backwards.
/// For audio-only sources (podcast clips, voice memos); video inputs
/// would desync — those belong to `jumpcut`.
pub fn run(args: CutsilArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_audio {
        return Err(Error::input("cutsil needs an audio stream"));
    }
    if probe.has_video {
        return Err(Error::input(
            "cutsil is audio-only — on video it desyncs (use jumpcut)",
        ));
    }
    if !(-90.0..0.0).contains(&args.thresh) {
        return Err(Error::input("--thresh must be a negative dB, e.g. -45"));
    }
    let af = format!(
        "silenceremove=start_periods=1:start_threshold={}dB:stop_periods=-1:stop_threshold={}dB",
        args.thresh, args.thresh
    );
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-af", &af, "-map", "0:a", "-c:a", "aac"]);
    argv.push(&args.output);

    let mut c = engine::write_job("cutsil", &[&args.input], &args.output, vec![argv], g)?;
    c = c.with_extra(json!({
        "thresh_db": args.thresh,
        "source_duration": probe.duration,
    }));
    Ok(c)
}

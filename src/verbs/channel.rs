use serde_json::json;

use crate::cli::{ChannelArgs, ChannelMode, Globals};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: ChannelArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_audio {
        return Err(Error::input("channel needs an audio stream"));
    }
    let af = match args.mode {
        // single-mic voice recorded on one ear → copy ch0 to all
        ChannelMode::Dualmono => "pan=stereo|FL<c0|FR<c0".to_string(),
        ChannelMode::Mono => "aformat=channel_layouts=mono".to_string(),
        ChannelMode::Swap => "channelmap=map=FR-FL|FL-FR:channel_layout=stereo".to_string(),
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

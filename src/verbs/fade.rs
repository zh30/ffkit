use serde_json::json;

use crate::cli::{FadeArgs, Globals};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: FadeArgs, g: &Globals) -> Result<Contract, Error> {
    let fade_in = args.fade_in;
    let fade_out = args.fade_out;
    if fade_in < 0.0 || fade_out < 0.0 {
        return Err(Error::input("--fade-in and --fade-out must be >= 0"));
    }
    if fade_in == 0.0 && fade_out == 0.0 {
        return Err(Error::input(
            "set --fade-in and/or --fade-out greater than 0",
        ));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    if fade_in + fade_out >= probe.duration && probe.duration > 0.0 {
        return Err(Error::input(format!(
            "fade in+out ({:.3}s) must be shorter than duration {:.3}s",
            fade_in + fade_out,
            probe.duration
        )));
    }

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    if probe.has_video {
        let mut vf = Vec::new();
        if fade_in > 0.0 {
            vf.push(format!("fade=t=in:st=0:d={fade_in}"));
        }
        if fade_out > 0.0 {
            let st = (probe.duration - fade_out).max(0.0);
            vf.push(format!("fade=t=out:st={st}:d={fade_out}"));
        }
        argv.extend(["-vf", &vf.join(",")]);
        argv.extend([
            "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
        ]);
    }
    if probe.has_audio {
        let mut af = Vec::new();
        if fade_in > 0.0 {
            af.push(format!("afade=t=in:st=0:d={fade_in}"));
        }
        if fade_out > 0.0 {
            let st = (probe.duration - fade_out).max(0.0);
            af.push(format!("afade=t=out:st={st}:d={fade_out}"));
        }
        argv.extend(["-af", &af.join(",")]);
        argv.extend(["-c:a", "aac"]);
    }
    argv.push(&args.output);

    let c = engine::write_job("fade", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "fade_in": fade_in,
        "fade_out": fade_out,
    })))
}

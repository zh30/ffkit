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
    if fade_in == 0.0 && fade_out == 0.0 && args.dip.is_none() {
        return Err(Error::input(
            "set --fade-in/--fade-out/--dip greater than 0",
        ));
    }
    if args.dur.is_some() && args.dip.is_none() {
        return Err(Error::input("--dur needs --dip"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    if fade_in + fade_out >= probe.duration && probe.duration > 0.0 {
        return Err(Error::input(format!(
            "fade in+out ({:.3}s) must be shorter than duration {:.3}s",
            fade_in + fade_out,
            probe.duration
        )));
    }

    let mut dips = Vec::new();
    if let Some(raw) = &args.dip {
        let dl = args.dur.unwrap_or(0.8);
        let h = dl / 2.0;
        for part in raw.split(',') {
            let t = crate::time::parse_time(part.trim())?;
            if h <= 0.0 || t - h < 0.0 || t + h > probe.duration {
                return Err(Error::input("--dip ± --dur/2 must fit inside the input"));
            }
            dips.push((t, h));
        }
        dips.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    }
    let color = args.color.as_deref().unwrap_or("black");

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    if probe.has_video {
        let mut vf = Vec::new();
        if fade_in > 0.0 {
            vf.push(format!("fade=t=in:st=0:d={fade_in}:color={color}"));
        }
        if fade_out > 0.0 {
            let st = (probe.duration - fade_out).max(0.0);
            vf.push(format!("fade=t=out:st={st}:d={fade_out}:color={color}"));
        }
        for &(t, h) in &dips {
            vf.push(format!(
                "fade=t=out:st={:.3}:d={h:.3}:color={color},fade=t=in:st={t:.3}:d={h:.3}:color={color}",
                t - h
            ));
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
        for &(t, h) in &dips {
            af.push(format!(
                "afade=t=out:st={:.3}:d={h:.3},afade=t=in:st={t:.3}:d={h:.3}",
                t - h
            ));
        }
        argv.extend(["-af", &af.join(",")]);
        argv.extend(["-c:a", "aac"]);
    }
    argv.push(&args.output);

    let c = engine::write_job("fade", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "fade_in": fade_in,
        "fade_out": fade_out,
        "dip": dips.iter().map(|(t, _)| t).collect::<Vec<_>>(),
    })))
}

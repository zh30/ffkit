use serde_json::json;

use crate::cli::{FxArgs, FxKind, Globals};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

/// Audio FX rack: one filter per effect, --strength scales its depth.
pub fn run(args: FxArgs, g: &Globals) -> Result<Contract, Error> {
    if !(0.0..=1.0).contains(&args.strength) {
        return Err(Error::input("--strength must be 0..1"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_audio {
        return Err(Error::input("fx: input has no audio stream"));
    }
    let s = args.strength;
    let mut af = match args.kind {
        FxKind::Tremolo => format!("tremolo=f=6:d={:.2}", 0.3 + 0.7 * s),
        FxKind::Vibrato => format!("vibrato=f=5:d={:.2}", 0.3 + 0.7 * s),
        FxKind::Flanger => format!(
            "flanger=delay={:.1}:depth={:.1}",
            2.0 + 8.0 * s,
            1.0 + 3.0 * s
        ),
        FxKind::Phaser => format!(
            "aphaser=in_gain=0.8:out_gain=0.9:delay={:.1}:decay=0.5:speed={:.2}:type=t",
            1.5 + 4.0 * s,
            0.5 + s
        ),
        FxKind::Chorus => format!("chorus=0.7:0.9:55:0.4:0.25:{:.1}", 1.0 + 2.0 * s),
    };
    if args.at.is_some() && matches!(args.kind, FxKind::Chorus) {
        return Err(Error::input("chorus has no timeline support — drop --at"));
    }
    match &args.at {
        Some(raw) => {
            let at = crate::time::parse_time(raw)?;
            if !(0.0..probe.duration).contains(&at) {
                return Err(Error::input("--at is outside the input"));
            }
            af.push_str(&match args.dur {
                Some(d) if at + d < probe.duration => {
                    format!(":enable='between(t,{at:.3},{:.3})'", at + d)
                }
                _ => format!(":enable='gte(t,{at:.3})'"),
            });
        }
        None => {
            if args.dur.is_some() {
                return Err(Error::input("--dur needs --at"));
            }
        }
    }

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-af", &af]);
    if probe.has_video {
        argv.extend(["-c:v", "copy"]);
    }
    argv.extend(["-c:a", "aac"]);
    argv.push(&args.output);

    let c = engine::write_job("fx", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "effect": format!("{:?}", args.kind).to_lowercase(),
        "strength": s,
        "filter": af,
    })))
}

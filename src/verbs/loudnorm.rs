use serde_json::json;

use crate::cli::{Globals, LoudnormArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::spawn::{self, Argv};

pub fn run(args: LoudnormArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_audio {
        return Err(Error::input("loudnorm: input has no audio stream"));
    }

    let (i, tp, lra) = match args.target {
        Some(crate::cli::LoudnormTarget::Spotify | crate::cli::LoudnormTarget::Youtube) => {
            (-14.0, -1.5, 11.0)
        }
        Some(crate::cli::LoudnormTarget::Podcast) => (-16.0, -1.5, 11.0),
        Some(crate::cli::LoudnormTarget::Broadcast) => (-23.0, -2.0, 7.0),
        None => (-14.0, -1.5, 11.0),
    };
    let i = args.i.unwrap_or(i);
    let tp = args.tp.unwrap_or(tp);
    let lra = args.lra.unwrap_or(lra);
    let filter = measure_filter(i, tp, lra);

    // Keep loglevel high enough for loudnorm's JSON on stderr.
    let mut measure = Argv::ffmpeg();
    measure.extend(["-nostats", "-i"]);
    measure.push(&args.input);
    measure.extend(["-af", &filter, "-f", "null", "-"]);

    if g.dry_run {
        let mut apply = ffmpeg_base(g.progress);
        apply.push("-i");
        apply.push(&args.input);
        apply.extend(["-af", &filter]);
        if probe.has_video {
            apply.extend(["-c:v", "copy"]);
        }
        apply.push(&args.output);
        return engine::write_job(
            "loudnorm",
            &[&args.input],
            &args.output,
            vec![measure, apply],
            g,
        );
    }

    crate::paths::ensure_output_allowed(&args.output, &[&args.input], g.overwrite)?;
    let spawned = spawn::run(&measure, g.timeout, false)?;
    let spawned = spawn::require_ok(&measure, spawned)?;
    let stderr = spawn::stderr_str(&spawned);
    let meas = parse_measured(&stderr)?;
    let second = apply_filter(i, tp, lra, &meas);

    let mut apply = ffmpeg_base(g.progress);
    apply.push("-i");
    apply.push(&args.input);
    apply.extend(["-af", &second]);
    if probe.has_video {
        apply.extend(["-c:v", "copy"]);
    }
    apply.push(&args.output);

    let mut contract = engine::write_job("loudnorm", &[&args.input], &args.output, vec![apply], g)?;
    let mut commands = engine::commands_of(&[measure]);
    commands.extend(contract.commands.clone());
    contract.commands = commands;
    contract = contract.with_extra(json!({
        "target_i": i,
        "target_tp": tp,
        "measured": meas,
    }));
    Ok(contract)
}

pub(crate) fn measure_filter(i: f64, tp: f64, lra: f64) -> String {
    format!("loudnorm=I={i}:TP={tp}:LRA={lra}:print_format=json")
}

pub(crate) fn apply_filter(i: f64, tp: f64, lra: f64, meas: &serde_json::Value) -> String {
    format!(
        "loudnorm=I={i}:TP={tp}:LRA={lra}:measured_I={}:measured_TP={}:measured_LRA={}:measured_thresh={}:offset={}:linear=true",
        num(meas, "input_i"),
        num(meas, "input_tp"),
        num(meas, "input_lra"),
        num(meas, "input_thresh"),
        num(meas, "target_offset"),
    )
}

pub(crate) fn parse_measured(stderr: &str) -> Result<serde_json::Value, Error> {
    let measured = extract_json_object(stderr)
        .ok_or_else(|| Error::ffmpeg("loudnorm first pass did not print JSON"))?;
    Ok(serde_json::from_str(&measured)?)
}

fn num(v: &serde_json::Value, key: &str) -> String {
    match v.get(key) {
        Some(serde_json::Value::Number(n)) => n.to_string(),
        Some(serde_json::Value::String(s)) => s.clone(),
        _ => "0".into(),
    }
}

fn extract_json_object(s: &str) -> Option<String> {
    let start = s.rfind('{')?;
    let end = s.rfind('}')?;
    if end <= start {
        return None;
    }
    Some(s[start..=end].to_string())
}

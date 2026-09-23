use crate::cli::DespillArgs;
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use serde_json::json;

/// Remove green/blue screen spill from a keyed edge — despill without keying:
/// recolours the fringe the spill map catches. For fixing edges left by an
/// upstream keyer, or footage that only needs the colour cast gone.
pub fn run(args: DespillArgs, g: &crate::cli::Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "despill")?;
    let kind = match args.kind {
        crate::cli::SpillType::Green => 0,
        crate::cli::SpillType::Blue => 1,
    };
    let mix = args.mix.clamp(0.0, 1.0);
    let expand = args.expand.clamp(0.0, 1.0);
    let mut vf = format!("despill=type={kind}:mix={mix}:expand={expand}");
    if let Some(s) = &args.at {
        let win = crate::time::enable_expr(s, args.dur, probe.duration)?;
        vf = format!("{vf}:enable='{win}'");
    } else if args.dur.is_some() {
        return Err(Error::input("--dur needs --at"));
    }

    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend([
        "-vf", &vf, "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
    ]);
    if probe.has_audio {
        argv.extend(["-c:a", "copy"]);
    }
    argv.push(&args.output);
    let c = engine::write_job("despill", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "type": format!("{:?}", args.kind).to_lowercase(),
        "mix": mix,
        "expand": expand,
        "filter": vf,
    })))
}

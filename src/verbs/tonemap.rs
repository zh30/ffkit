use crate::cli::{TonemapAlgo, TonemapArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use serde_json::json;

/// HDR → SDR tone mapping: zscale to linear light, tonemap curve, zscale
/// back to bt709. For PQ/HLG phone or camera footage headed to SDR platforms.
pub fn run(args: TonemapArgs, g: &crate::cli::Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "tonemap")?;
    let algo = match args.algo {
        TonemapAlgo::Hable => "hable",
        TonemapAlgo::Reinhard => "reinhard",
        TonemapAlgo::Gamma => "gamma",
        TonemapAlgo::Clip => "clip",
        TonemapAlgo::Linear => "linear",
    };
    let peak = args.peak.clamp(10.0, 10000.0);
    let vf = format!(
        "zscale=t=linear:npl={peak:.0},format=gbrpf32le,tonemap={algo},zscale=t=bt709:m=bt709:p=bt709:tin=linear,format=yuv420p"
    );
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
    let c = engine::write_job("tonemap", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "algorithm": algo,
        "peak_nits": peak,
        "transfer": "bt709",
    })))
}

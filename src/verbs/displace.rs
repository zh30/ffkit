use serde_json::json;

use crate::cli::{DisplaceArgs, DisplaceEdge, Globals};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

/// Warp the picture by a second clip's displacement map — heat ripple, liquid
/// glitch, water reflections. `displace` needs three inputs (source + x-map +
/// y-map); we feed the map clip twice, scaled to the source size via
/// scale2ref.
pub fn run(args: DisplaceArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "displace")?;
    let map_probe = engine::probe_or_err(&args.map, g)?;
    engine::need_video(&map_probe, "displace --map")?;

    let edge = match args.edge {
        DisplaceEdge::Smear => "smear",
        DisplaceEdge::Blank => "blank",
        DisplaceEdge::Wrap => "wrap",
        DisplaceEdge::Mirror => "mirror",
    };
    let mut disp = format!("displace=edge={edge}");
    match &args.at {
        Some(s) => {
            disp.push_str(&format!(
                ":enable='{}'",
                crate::time::enable_expr(s, args.dur, probe.duration)?
            ));
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
    argv.push("-i");
    argv.push(&args.map);
    let fc = format!("[1:v][0:v]scale2ref[mp][src];[mp]split[x][y];[src][x][y]{disp}[v]");
    argv.extend(["-filter_complex", &fc, "-map", "[v]", "-map", "0:a?"]);
    argv.extend([
        "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
    ]);
    if probe.has_audio {
        argv.extend(["-c:a", "copy"]);
    }
    argv.push(&args.output);

    let c = engine::write_job(
        "displace",
        &[&args.input, &args.map],
        &args.output,
        vec![argv],
        g,
    )?;
    Ok(c.with_extra(json!({
        "edge": format!("{:?}", args.edge).to_lowercase(),
        "filter": "displace",
    })))
}

use serde_json::json;

use crate::cli::{EqvizArgs, Globals};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

/// Apply an EQ and render its frequency-response curve as the video — the
/// clip doubles as a "what I did to this mix" QC card. `anequalizer` outputs
/// two pads: the drawn curve (video) and the EQ'd audio — both must be
/// consumed (any dangling pad stalls the graph).
pub fn run(args: EqvizArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_audio {
        return Err(Error::input("eqviz: input has no audio stream"));
    }
    let (w, h) = crate::verbs::meter::parse_size(&args.size)?;

    // one band definition is applied to every channel; a " | "-separated list
    // maps c0|c1... per channel
    let params = match &args.bands {
        Some(b) => {
            let bands: Vec<&str> = b.split('|').map(|x| x.trim()).collect();
            let chans = probe.channels.unwrap_or(2).max(1) as usize;
            if bands.len() == 1 {
                (0..chans)
                    .map(|i| format!("c{i} {}", bands[0]))
                    .collect::<Vec<_>>()
                    .join("|")
            } else {
                (0..chans.min(bands.len()))
                    .map(|i| format!("c{i} {}", bands[i]))
                    .collect::<Vec<_>>()
                    .join("|")
            }
        }
        None => String::new(),
    };

    let mut argv = ffmpeg_base(g.progress);
    if let Some(raw) = &args.at {
        let at = crate::time::resolve_frame_at(raw.trim(), probe.duration)?;
        argv.extend(["-ss", &crate::time::fmt_time(at.max(0.0))]);
    }
    argv.push("-i");
    argv.push(&args.input);
    if let Some(d) = args.dur {
        if d <= 0.0 {
            return Err(Error::input("--dur must be > 0"));
        }
        argv.extend(["-t", &crate::time::fmt_time(d)]);
    }
    let eq = if params.is_empty() {
        String::new()
    } else {
        format!("params='{params}':")
    };
    argv.extend([
        "-filter_complex",
        &format!("[0:a]anequalizer={eq}curves=1:size={w}x{h}[v][a]"),
        "-map",
        "[v]",
        "-map",
        "[a]",
        "-c:v",
        "libx264",
        "-preset",
        "veryfast",
        "-crf",
        "22",
        "-pix_fmt",
        "yuv420p",
        "-c:a",
        "aac",
        "-b:a",
        "192k",
        "-shortest",
    ]);
    argv.push(&args.output);

    let c = engine::write_job("eqviz", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({
        "bands": args.bands,
        "filter": "anequalizer",
    })))
}

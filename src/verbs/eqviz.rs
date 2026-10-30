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
    if args.dur.is_some_and(|d| d <= 0.0) {
        return Err(Error::input("--dur must be > 0"));
    }
    let eq = if params.is_empty() {
        String::new()
    } else {
        format!("params='{params}':")
    };
    let graph = if curves_pad_works() {
        format!("[0:a]anequalizer={eq}curves=1:size={w}x{h}[v][a]")
    } else {
        // ffmpeg 9's anequalizer cannot negotiate formats once curves=1
        // adds its video pad. Same picture another way: the EQ's response
        // is the spectrum of one impulse sent through it, drawn once and
        // held over black for the length of the audio.
        argv.extend(["-f", "lavfi", "-i"]);
        argv.push("aevalsrc=exprs='if(eq(n\\,0)\\,1\\,0)':s=48000:d=0.2");
        argv.extend(["-f", "lavfi", "-i"]);
        argv.push(format!("color=black:s={w}x{h}:r=25"));
        let first = params.split('|').next().unwrap_or("");
        let band = first.split_once(' ').map_or("", |(_, b)| b);
        let curve_eq = if band.is_empty() {
            String::new()
        } else {
            format!("=params='c0 {band}'")
        };
        let eq_audio = if eq.is_empty() {
            "curves=0".to_string()
        } else {
            format!("{eq}curves=0")
        };
        format!(
            "[0:a]anequalizer={eq_audio}[a];\
             [1:a]anequalizer{curve_eq},showfreqs=s={w}x{h}:mode=line:fscale=log:ascale=log:win_size=8192:colors=white,trim=end_frame=1[c];\
             [2:v][c]overlay=eof_action=repeat[v]"
        )
    };
    // output option: after every input, or it binds to the next -i
    if let Some(d) = args.dur {
        argv.extend(["-t", &crate::time::fmt_time(d)]);
    }
    argv.extend([
        "-filter_complex",
        &graph,
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

/// Whether this ffmpeg's anequalizer can emit its curves video pad; one
/// tiny run, asked once per process.
fn curves_pad_works() -> bool {
    static OK: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *OK.get_or_init(|| {
        let mut argv = crate::spawn::Argv::ffmpeg();
        argv.extend([
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "anullsrc=d=0.1",
            "-filter_complex",
            "[0:a]anequalizer=curves=1[v][a]",
            "-map",
            "[v]",
            "-map",
            "[a]",
            "-f",
            "null",
            "-",
        ]);
        crate::spawn::run(&argv, std::time::Duration::from_secs(10), false)
            .is_ok_and(|s| s.status_ok)
    })
}

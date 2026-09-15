use crate::cli::{CaptionArgs, CaptionMode, Globals};
use crate::contract::Contract;
use crate::doctor;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::paths;

pub fn run(args: CaptionArgs, g: &Globals) -> Result<Contract, Error> {
    paths::ensure_input(&args.srt)?;
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "caption")?;

    let mut argv = ffmpeg_base(g.progress);
    match args.mode {
        CaptionMode::Burn => {
            let filters = doctor::list_filters()?;
            if !filters.contains("subtitles") {
                return Err(Error::missing_tool(
                    "caption --mode burn needs ffmpeg built with libass (subtitles filter). Use --mode mux, or install ffmpeg with --enable-libass",
                ));
            }
            argv.push("-i");
            argv.push(&args.input);
            let escaped = escape_subtitles_path(&paths::abs(&args.srt).to_string_lossy());
            let mut filter = format!("subtitles=filename={escaped}");
            if let Some(font) = &args.font {
                let style = font.replace('\'', "\\'");
                filter.push_str(&format!(":force_style='FontName={style}'"));
            }
            argv.extend(["-vf", &filter]);
            argv.extend(["-c:v", "libx264", "-preset", "fast", "-crf", "18"]);
            if probe.has_audio {
                argv.extend(["-c:a", "copy"]);
            }
        }
        CaptionMode::Mux => {
            argv.push("-i");
            argv.push(&args.input);
            argv.push("-i");
            argv.push(&args.srt);
            argv.extend(["-c", "copy"]);
            let ext = args
                .output
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("mp4")
                .to_ascii_lowercase();
            if ext == "mp4" || ext == "mov" || ext == "m4v" {
                argv.extend(["-c:s", "mov_text"]);
            }
        }
    }
    argv.push(&args.output);
    engine::write_job(
        "caption",
        &[&args.input, &args.srt],
        &args.output,
        vec![argv],
        g,
    )
}

fn escape_subtitles_path(p: &str) -> String {
    p.replace('\\', "\\\\")
        .replace(':', "\\:")
        .replace('\'', "\\'")
        .replace('[', "\\[")
        .replace(']', "\\]")
        .replace(',', "\\,")
        .replace(';', "\\;")
}

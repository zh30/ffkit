use crate::cli::{ExtractArgs, Globals};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::time::{fmt_time, parse_time};

pub fn run(args: ExtractArgs, g: &Globals) -> Result<Contract, Error> {
    if args.bounce && !args.gif {
        return Err(Error::input("--bounce needs --gif"));
    }
    let ext = args
        .output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    let mut argv = ffmpeg_base(g.progress);

    match ext.as_str() {
        _ if args.gif => {
            // --bounce handled below in the paletteuse pass
            // 2-pass palette GIF from the window, like transcode --preset gif
            let mut gen = ffmpeg_base(g.progress);
            if let Some(at) = &args.at {
                gen.extend(["-ss", &fmt_time(parse_time(at)?)]);
            }
            if let Some(d) = args.dur {
                gen.extend(["-t", &fmt_time(d)]);
            }
            gen.push("-i");
            gen.push(&args.input);
            let fps = args.fps.unwrap_or(10).clamp(1, 30);
            let w = args.width.unwrap_or(480).clamp(16, 1920);
            let scale = format!("fps={fps},scale={w}:-2:flags=lanczos");
            let palette = tempfile::Builder::new()
                .suffix(".png")
                .tempfile()
                .map_err(|e| Error::output(e.to_string()))?;
            let palette_path = palette.path().to_path_buf();
            gen.extend(["-vf", &format!("{scale},palettegen=stats_mode=full")]);
            gen.push(&palette_path);

            let mut use_p = ffmpeg_base(g.progress);
            if let Some(at) = &args.at {
                use_p.extend(["-ss", &fmt_time(parse_time(at)?)]);
            }
            if let Some(d) = args.dur {
                use_p.extend(["-t", &fmt_time(d)]);
            }
            use_p.push("-i");
            use_p.push(&args.input);
            use_p.push("-i");
            use_p.push(&palette_path);
            let seq = if args.bounce {
                format!("{scale},split[a][b];[b]reverse[r];[a][r]concat=n=2:v=1:a=0[x]")
            } else {
                format!("{scale}[x]")
            };
            use_p.extend([
                "-lavfi",
                &format!("{seq};[x][1:v]paletteuse=dither=bayer"),
                "-an",
            ]);
            use_p.push(&args.output);
            let r = engine::write_job("extract", &[&args.input], &args.output, vec![gen, use_p], g);
            drop(palette);
            return r;
        }
        "png" | "jpg" | "jpeg" | "webp" => {
            if let Some(at) = &args.at {
                let t = parse_time(at)?;
                argv.extend(["-ss", &fmt_time(t)]);
            }
            argv.push("-i");
            argv.push(&args.input);
            if let Some(w) = args.width {
                argv.extend(["-vf", &format!("scale={w}:-2")]);
            }
            argv.extend(["-frames:v", "1", "-q:v", "2"]);
        }
        "wav" => {
            argv.push("-i");
            argv.push(&args.input);
            argv.extend(["-vn", "-acodec", "pcm_s16le"]);
        }
        "mp3" => {
            argv.push("-i");
            argv.push(&args.input);
            argv.extend(["-vn", "-c:a", "libmp3lame", "-q:a", "2"]);
        }
        "m4a" | "aac" => {
            argv.push("-i");
            argv.push(&args.input);
            argv.extend(["-vn", "-c:a", "aac", "-b:a", "192k"]);
        }
        "flac" => {
            argv.push("-i");
            argv.push(&args.input);
            argv.extend(["-vn", "-c:a", "flac"]);
        }
        "ogg" | "opus" => {
            argv.push("-i");
            argv.push(&args.input);
            argv.extend(["-vn", "-c:a", "libopus"]);
        }
        "srt" | "ass" | "vtt" => {
            argv.push("-i");
            argv.push(&args.input);
            argv.extend(["-map", "0:s:0"]);
        }
        other => {
            return Err(Error::input(format!(
                "extract: unknown output extension .{other}; use wav/mp3/m4a/png/srt"
            )));
        }
    }
    argv.push(&args.output);
    engine::write_job("extract", &[&args.input], &args.output, vec![argv], g)
}

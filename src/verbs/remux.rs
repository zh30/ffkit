use serde_json::json;

use crate::cli::{Globals, RemuxArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub fn run(args: RemuxArgs, g: &Globals) -> Result<Contract, Error> {
    let ext = args
        .output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if ext.is_empty() {
        return Err(Error::input(
            "output needs an extension (mp4, mkv, mov, m4a…)",
        ));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_video && !probe.has_audio {
        return Err(Error::input("remux: input has no media streams"));
    }
    if let Some(f) = args.from {
        if !f.is_finite() || f < 0.0 {
            return Err(Error::input("remux --from needs a time >= 0"));
        }
    }
    if let Some(t) = args.to {
        if !t.is_finite() || t <= 0.0 {
            return Err(Error::input("remux --to needs a positive time"));
        }
        if t <= args.from.unwrap_or(0.0) {
            return Err(Error::input("remux --to must be after --from"));
        }
    }

    let lang = args
        .lang
        .as_deref()
        .map(str::trim)
        .filter(|l| !l.is_empty());
    if args.cover.is_some() && lang.is_some() {
        return Err(Error::input(
            "remux --cover and --lang pick different stream sets — use them separately",
        ));
    }
    let mut argv = ffmpeg_base(g.progress);
    // Input-side -ss seeks to the nearest keyframe at/below --from — the
    // lossless-trim trade-off (cut/split re-encode for frame accuracy).
    if let Some(f) = args.from {
        argv.extend(["-ss".into(), f.to_string()]);
    }
    argv.push("-i");
    argv.push(&args.input);
    let mut ni = 1u32;
    if let Some(cover) = &args.cover {
        crate::paths::ensure_input(cover)?;
        argv.push("-i");
        argv.push(cover);
        ni += 1;
    }
    // --chapters: the same YouTube-format list deliver --chapters eats,
    // embedded as container chapters on the repack
    let mut chap_file: Option<std::path::PathBuf> = None;
    let mut chap_n = 0usize;
    if let Some(cf) = &args.chapters {
        crate::paths::ensure_input(cf)?;
        let marks = crate::verbs::chapter::parse_yt_list(cf)?;
        crate::verbs::chapter::check_marks(&marks, probe.duration, "remux --chapters")?;
        chap_n = marks.len();
        let tmp =
            std::env::temp_dir().join(format!("ffkit-remux-chap-{}.ffmeta", std::process::id()));
        std::fs::write(
            &tmp,
            crate::verbs::chapter::ffmeta_table(&marks, probe.duration),
        )
        .map_err(|e| Error::output(format!("writing chapters: {e}")))?;
        argv.extend(["-f", "ffmetadata", "-i"]);
        argv.push(&tmp);
        argv.extend(["-map_metadata", &ni.to_string()]);
        argv.extend(["-map_chapters", &ni.to_string()]);
        chap_file = Some(tmp);
    }
    if args.audio && args.video {
        return Err(Error::input("remux: --audio and --video are exclusive"));
    }
    if args.no_subs && (args.audio || args.video) {
        return Err(Error::input(
            "remux --no-subs only applies to a full repack — drop --audio/--video",
        ));
    }
    if let Some(l) = lang {
        if !l.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return Err(Error::input(
                "remux --lang wants an ISO-639 code (eng, jpn, zh-hans…)",
            ));
        }
        if args.video {
            return Err(Error::input(
                "remux --lang picks an audio track — --video drops all audio",
            ));
        }
        if !probe.has_audio {
            return Err(Error::input("remux --lang: input has no audio"));
        }
    }
    if args.audio {
        if !probe.has_audio {
            return Err(Error::input("remux --audio: input has no audio"));
        }
        // Re-encode only when the container can't hold the source codec.
        let src = probe.acodec.as_deref().unwrap_or("");
        let fits = match ext.as_str() {
            "m4a" | "mp4" | "mov" => matches!(src, "aac" | "alac" | "mp3"),
            "mp3" => src == "mp3",
            "ogg" | "oga" => matches!(src, "vorbis" | "opus" | "flac"),
            "wav" | "aif" | "aiff" | "caf" => src.starts_with("pcm"),
            _ => true,
        };
        let audio_map = match lang {
            Some(l) => format!("0:a:m:language:{l}"),
            None => "0:a".to_string(),
        };
        argv.extend(["-map", audio_map.as_str()]);
        if fits {
            argv.extend(["-c:a", "copy"]);
        } else {
            let codec = match ext.as_str() {
                "mp3" => "libmp3lame",
                "ogg" | "oga" => "libvorbis",
                "wav" | "aif" | "aiff" | "caf" => "pcm_s16le",
                _ => "aac",
            };
            argv.extend(["-c:a", codec]);
        }
    } else if args.video {
        if !probe.has_video {
            return Err(Error::input("remux --video: input has no video"));
        }
        argv.extend(["-map", "0:v", "-c:v", "copy"]);
    } else {
        if args.no_subs {
            // negative maps drop subtitle/data streams; attachments stay
            argv.extend(["-map", "0", "-map", "-0:s", "-map", "-0:d", "-c", "copy"]);
        } else if let Some(l) = lang {
            // language-filtered repack: every video + only LANG-tagged audio
            argv.extend(["-map", "0:v", "-map"]);
            argv.push(format!("0:a:m:language:{l}"));
            argv.extend(["-map", "0:s?", "-map", "0:d?", "-c", "copy"]);
        } else {
            argv.extend(["-map", "0", "-c", "copy"]);
        }
    }
    // --cover: the image joins as an attached_pic video stream. Output
    // video index 0 is the pic itself on audio-only rips, otherwise it
    // trails the content's own video.
    if args.cover.is_some() {
        argv.extend(["-map", "1:v"]);
        let pic_idx = if args.audio || (!probe.has_video && !args.video) {
            0u32
        } else {
            1u32
        };
        argv.extend([
            "-c:v:".to_string() + &pic_idx.to_string(),
            "mjpeg".to_string(),
        ]);
        argv.extend([
            "-disposition:v:".to_string() + &pic_idx.to_string(),
            "attached_pic".to_string(),
        ]);
        // .m4a/.m4b resolve to the ipod muxer, which rejects video streams
        // on ffmpeg 4.x — force mp4 (same container family)
        if matches!(ext.as_str(), "m4a" | "m4b" | "mp4" | "mov") {
            argv.extend(["-f", "mp4"]);
        }
    }
    // container tags ride the repack — fix a library's metadata without
    // re-encoding
    let mut tag_n = 0u32;
    for (k, v) in [
        ("title", &args.title),
        ("artist", &args.artist),
        ("album", &args.album),
        ("genre", &args.genre),
        ("comment", &args.comment),
        ("date", &args.date),
    ] {
        if let Some(v) = v {
            if v.trim().is_empty() {
                return Err(Error::input(format!("remux --{k}: empty value")));
            }
            argv.extend(["-metadata".to_string(), format!("{k}={v}")]);
            tag_n += 1;
        }
    }
    if let Some(n) = args.default_audio {
        if args.audio || args.video || !probe.has_audio {
            return Err(Error::input(
                "remux --default-audio needs a full repack with audio tracks",
            ));
        }
        // clear every audio default flag, then set it on the chosen track
        argv.extend(["-disposition:a", "-default"]);
        argv.extend([
            "-disposition:a:".to_string() + &n.to_string(),
            "+default".to_string(),
        ]);
    }
    if let Some(t) = args.to {
        // output duration, not a timeline position — input -ss already
        // rewound the stream to ~0
        argv.extend(["-t", &(t - args.from.unwrap_or(0.0)).to_string()]);
    }
    if let Some(a) = &args.aspect {
        if args.audio || !probe.has_video {
            return Err(Error::input("remux --aspect needs a video stream"));
        }
        if a.trim().is_empty() {
            return Err(Error::input("remux: empty --aspect"));
        }
        argv.extend(["-aspect", a.trim()]);
    }
    if args.frag {
        if !matches!(ext.as_str(), "mp4" | "mov") {
            return Err(Error::input("remux --frag needs an mp4/mov output"));
        }
        // fragmented moov — the file plays/streamable while still being written
        argv.extend(["-movflags", "frag_keyframe+empty_moov+default_base_moof"]);
    } else if matches!(ext.as_str(), "mp4" | "m4a" | "mov") {
        argv.extend(["-movflags", "+faststart"]);
    }
    argv.push(&args.output);

    let run = engine::write_job("remux", &[&args.input], &args.output, vec![argv], g);
    if let Some(tmp) = &chap_file {
        let _ = std::fs::remove_file(tmp);
    }
    let c = run?;
    Ok(c.with_extra(
        json!({ "container": ext, "audio_only": args.audio, "video_only": args.video, "fragmented": args.frag, "no_subs": args.no_subs, "from": args.from, "to": args.to, "lang": lang, "default_audio": args.default_audio, "cover": args.cover.is_some(), "chapters": chap_n, "tags": tag_n }),
    ))
}

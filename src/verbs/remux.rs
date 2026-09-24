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

    let langs: Vec<&str> = args
        .lang
        .as_deref()
        .map(|s| {
            s.split(',')
                .map(str::trim)
                .filter(|l| !l.is_empty())
                .collect()
        })
        .unwrap_or_default();
    let sub_langs: Vec<&str> = args
        .sub_lang
        .as_deref()
        .map(|s| {
            s.split(',')
                .map(str::trim)
                .filter(|l| !l.is_empty())
                .collect()
        })
        .unwrap_or_default();
    if args.no_cover && args.cover.is_some() {
        return Err(Error::input("remux --no-cover and --cover are exclusive"));
    }
    let encrypt = args.encrypt || args.key.is_some() || args.kid.is_some();
    if encrypt && !matches!(ext.as_str(), "mp4" | "mov" | "m4a" | "m4b") {
        return Err(Error::input(
            "remux --encrypt needs an ISOBMFF output (mp4/mov/m4a/m4b)",
        ));
    }
    if let Some(d) = args.audio_delay {
        if !d.is_finite() || d == 0.0 {
            return Err(Error::input(
                "remux --audio-delay needs a nonzero seconds value",
            ));
        }
        if !probe.has_audio {
            return Err(Error::input("remux --audio-delay: input has no audio"));
        }
        if args.audio || args.video || !langs.is_empty() {
            return Err(Error::input(
                "remux --audio-delay only applies to a full repack — drop --audio/--video/--lang",
            ));
        }
    }
    if let Some(d) = args.video_delay {
        if !d.is_finite() || d == 0.0 {
            return Err(Error::input(
                "remux --video-delay needs a nonzero seconds value",
            ));
        }
        if !probe.has_video {
            return Err(Error::input("remux --video-delay: input has no video"));
        }
        if args.audio || args.video || !langs.is_empty() {
            return Err(Error::input(
                "remux --video-delay only applies to a full repack — drop --audio/--video/--lang",
            ));
        }
    }
    if args.audio_delay.is_some() && args.video_delay.is_some() {
        return Err(Error::input(
            "pick one: --audio-delay shifts the audio, --video-delay shifts the video",
        ));
    }
    let mut argv = ffmpeg_base(g.progress);
    // Input-side -ss seeks to the nearest keyframe at/below --from — the
    // lossless-trim trade-off (cut/split re-encode for frame accuracy).
    if let Some(r) = args.itsscale {
        if !(r.is_finite() && r > 0.0 && r != 1.0) {
            return Err(Error::input(
                "remux --itsscale needs a positive factor != 1",
            ));
        }
        if args.from.is_some() || args.to.is_some() {
            return Err(Error::input(
                "remux --itsscale rescales the timeline — pick it or --from/--to",
            ));
        }
        if args.audio_delay.is_some() || args.video_delay.is_some() {
            return Err(Error::input(
                "remux --itsscale conflicts with --audio-delay/--video-delay (timestamp shifts)",
            ));
        }
        argv.extend(["-itsscale".into(), r.to_string()]);
    }
    if let Some(o) = args.offset {
        if !(o.is_finite() && o > 0.0) {
            return Err(Error::input("remux --offset needs positive seconds"));
        }
    }
    if let Some(f) = args.from {
        argv.extend(["-ss".into(), f.to_string()]);
    }
    let neg_shift = (-args.audio_delay.unwrap_or(0.0)).max(-args.video_delay.unwrap_or(0.0));
    if neg_shift > 0.0 {
        // advancing the shifted track = delaying everything else instead
        argv.extend(["-itsoffset".into(), neg_shift.to_string()]);
    }
    argv.push("-i");
    argv.push(&args.input);
    let mut ni = 1u32;
    let delay = args.audio_delay.or(args.video_delay);
    if let Some(d) = delay {
        if d > 0.0 {
            if let Some(f) = args.from {
                argv.extend(["-ss".into(), f.to_string()]);
            }
            argv.extend(["-itsoffset".into(), d.to_string()]);
            argv.push("-i");
            argv.push(&args.input);
            ni += 1;
        } else {
            if let Some(f) = args.from {
                argv.extend(["-ss".into(), f.to_string()]);
            }
            argv.push("-i");
            argv.push(&args.input);
            ni += 1;
        }
    }
    let cover_idx = ni;
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
    for l in &langs {
        if !l.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return Err(Error::input(
                "remux --lang wants ISO-639 codes (eng, jpn, zh-hans; comma list ok)",
            ));
        }
    }
    if !langs.is_empty() {
        if args.video {
            return Err(Error::input(
                "remux --lang picks an audio track — --video drops all audio",
            ));
        }
        if !probe.has_audio {
            return Err(Error::input("remux --lang: input has no audio"));
        }
    }
    for l in &sub_langs {
        if !l.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return Err(Error::input(
                "remux --sub-lang wants ISO-639 codes (eng, jpn, zh-hans; comma list ok)",
            ));
        }
    }
    if !sub_langs.is_empty() {
        if args.no_subs {
            return Err(Error::input(
                "remux --sub-lang keeps subtitle tracks — --no-subs drops them all",
            ));
        }
        if args.audio || args.video {
            return Err(Error::input(
                "remux --sub-lang picks a subtitle track — --audio/--video drops them all",
            ));
        }
        if args.audio_delay.is_some() || args.video_delay.is_some() {
            return Err(Error::input(
                "remux --sub-lang only applies to a full repack — drop --audio-delay/--video-delay",
            ));
        }
        if probe.subtitle_streams == 0 {
            return Err(Error::input(
                "remux --sub-lang: input has no subtitle streams",
            ));
        }
    }
    // --audio-order 1,0: keep + reorder audio tracks by per-type index.
    // Unlisted tracks are dropped (same surface as --lang's keep-list).
    let audio_order: Vec<usize> = match &args.audio_order {
        None => Vec::new(),
        Some(raw) => {
            let v: Vec<usize> = raw
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(|s| {
                    s.parse().map_err(|_| {
                        Error::input("remux --audio-order needs track indices (e.g. 1,0)")
                    })
                })
                .collect::<Result<_, _>>()?;
            if v.is_empty() {
                return Err(Error::input("remux --audio-order: no track indices given"));
            }
            let mut seen = std::collections::HashSet::new();
            for i in &v {
                if !seen.insert(*i) {
                    return Err(Error::input("remux --audio-order: duplicate track index"));
                }
            }
            v
        }
    };
    if !audio_order.is_empty() {
        if args.audio_delay.is_some() || args.video_delay.is_some() {
            return Err(Error::input(
                "remux --audio-order re-maps tracks — drop --audio-delay/--video-delay",
            ));
        }
        if args.video {
            return Err(Error::input(
                "remux --audio-order picks audio tracks — --video drops all audio",
            ));
        }
        if !langs.is_empty() {
            return Err(Error::input(
                "remux --audio-order replaces the audio map — drop --lang",
            ));
        }
        if !probe.has_audio {
            return Err(Error::input("remux --audio-order: input has no audio"));
        }
        let n_tracks = probe.streams.iter().filter(|s| s.kind == "audio").count();
        for i in &audio_order {
            if *i >= n_tracks {
                return Err(Error::input(format!(
                    "remux --audio-order {i}: input only has {n_tracks} audio track(s)"
                )));
            }
        }
    }
    // --sub-order 1,0: keep + reorder subtitle tracks by per-type index
    // (the audience's captions first on multi-sub releases; unlisted drop)
    let sub_order: Vec<usize> = match &args.sub_order {
        None => Vec::new(),
        Some(raw) => {
            let v: Vec<usize> = raw
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(|s| {
                    s.parse().map_err(|_| {
                        Error::input("remux --sub-order needs track indices (e.g. 1,0)")
                    })
                })
                .collect::<Result<_, _>>()?;
            if v.is_empty() {
                return Err(Error::input("remux --sub-order: no track indices given"));
            }
            let mut seen = std::collections::HashSet::new();
            for i in &v {
                if !seen.insert(*i) {
                    return Err(Error::input("remux --sub-order: duplicate track index"));
                }
            }
            v
        }
    };
    if !sub_order.is_empty() {
        if args.no_subs {
            return Err(Error::input(
                "remux --sub-order reorders subtitle tracks — --no-subs drops them all",
            ));
        }
        if !sub_langs.is_empty() {
            return Err(Error::input(
                "remux --sub-order re-maps subtitle tracks — drop --sub-lang",
            ));
        }
        if args.audio || args.video {
            return Err(Error::input(
                "remux --sub-order only applies to a full repack — drop --audio/--video",
            ));
        }
        if args.audio_delay.is_some() || args.video_delay.is_some() {
            return Err(Error::input(
                "remux --sub-order re-maps tracks — drop --audio-delay/--video-delay",
            ));
        }
        if probe.subtitle_streams == 0 {
            return Err(Error::input(
                "remux --sub-order: input has no subtitle streams",
            ));
        }
        for i in &sub_order {
            if *i >= probe.subtitle_streams as usize {
                return Err(Error::input(format!(
                    "remux --sub-order {i}: input only has {} subtitle track(s)",
                    probe.subtitle_streams
                )));
            }
        }
    }
    // The subtitle map set used wherever a repack lists subs explicitly:
    // ordered indices > language tags > every sub (optional so sub-free
    // sources don't fail)
    let sub_map_args: Vec<String> = if !sub_order.is_empty() {
        sub_order.iter().map(|i| format!("0:s:{i}")).collect()
    } else if !sub_langs.is_empty() {
        sub_langs
            .iter()
            .map(|l| format!("0:s:m:language:{l}"))
            .collect()
    } else {
        vec!["0:s?".to_string()]
    };
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
        if !audio_order.is_empty() {
            for i in &audio_order {
                argv.extend(["-map", format!("0:a:{i}").as_str()]);
            }
        } else if langs.is_empty() {
            argv.extend(["-map", "0:a"]);
        } else {
            for l in &langs {
                argv.extend(["-map", format!("0:a:m:language:{l}").as_str()]);
            }
        }
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
    } else if args.audio_delay.is_some() {
        // sync fix: non-audio streams from input 0, audio from the
        // itsoffset-shifted second read of the same file
        argv.extend(["-map", "0", "-map", "-0:a"]);
        if args.no_subs {
            argv.extend(["-map", "-0:s", "-map", "-0:d"]);
        }
        argv.extend(["-map", "1:a", "-c", "copy"]);
    } else if args.video_delay.is_some() {
        // same map for both signs: non-video streams from input 0, video
        // from input 1 — the sign only picks which read gets -itsoffset
        argv.extend(["-map", "0", "-map", "-0:v"]);
        if args.no_subs {
            argv.extend(["-map", "-0:s", "-map", "-0:d"]);
        }
        argv.extend(["-map", "1:v", "-c", "copy"]);
    } else {
        if args.no_subs {
            // negative maps drop subtitle/data streams; attachments stay
            argv.extend(["-map", "0", "-map", "-0:s", "-map", "-0:d", "-c", "copy"]);
        } else if !audio_order.is_empty() {
            // track-ordered repack: every video + the listed audio tracks
            // in the asked order (unlisted tracks drop out)
            argv.extend(["-map", "0:v"]);
            for i in &audio_order {
                argv.extend(["-map", format!("0:a:{i}").as_str()]);
            }
            for m in &sub_map_args {
                argv.extend(["-map", m.as_str()]);
            }
            argv.extend(["-map", "0:d?", "-c", "copy"]);
        } else if !langs.is_empty() || !sub_langs.is_empty() {
            // language-filtered repack: every video + only the tagged
            // audio/subtitle tracks (unlisted tracks drop out)
            argv.extend(["-map", "0:v?"]);
            if langs.is_empty() {
                argv.extend(["-map", "0:a?"]);
            } else {
                for l in &langs {
                    argv.extend(["-map", format!("0:a:m:language:{l}").as_str()]);
                }
            }
            for m in &sub_map_args {
                argv.extend(["-map", m.as_str()]);
            }
            argv.extend(["-map", "0:d?", "-c", "copy"]);
        } else if !sub_order.is_empty() {
            // subtitle-ordered repack: every video + all audio + the
            // listed subtitle tracks in the asked order
            argv.extend(["-map", "0:v?", "-map", "0:a?"]);
            for m in &sub_map_args {
                argv.extend(["-map", m.as_str()]);
            }
            argv.extend(["-map", "0:d?", "-c", "copy"]);
        } else {
            argv.extend(["-map", "0", "-c", "copy"]);
        }
    }
    // --no-cover: drop attached_pic video streams (album/feed art) on the
    // repack — disposition isn't a -map specifier, so each pic stream gets
    // its own negative index map (after every positive -map)
    if args.no_cover && !args.audio {
        for i in &probe.attached_pic_indices {
            argv.extend(["-map", format!("-0:{i}").as_str()]);
        }
    }
    // --cover: the image joins as an attached_pic video stream. Output
    // video index 0 is the pic itself on audio-only rips, otherwise it
    // trails the content's own video.
    if args.cover.is_some() {
        argv.extend(["-map".to_string(), format!("{cover_idx}:v")]);
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
    // privacy wipe: every inherited container tag dropped — explicit
    // --title/--artist/… pushes below still land (they come after -1)
    if args.strip_meta {
        argv.extend(["-map_metadata", "-1"]);
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
    // --default-sub: pick the default subtitle track (multi-language subs)
    if let Some(n) = args.default_sub {
        if args.audio || args.video || args.no_subs {
            return Err(Error::input(
                "remux --default-sub needs a full repack with subtitle tracks",
            ));
        }
        if probe.subtitle_streams == 0 {
            return Err(Error::input("remux --default-sub: input has no subtitles"));
        }
        if n >= probe.subtitle_streams as usize {
            return Err(Error::input(format!(
                "remux --default-sub {n}: only {} subtitle track(s)",
                probe.subtitle_streams
            )));
        }
        argv.extend(["-disposition:s", "-default"]);
        argv.extend([
            "-disposition:s:".to_string() + &n.to_string(),
            "+default".to_string(),
        ]);
    }
    // --timecode HH:MM:SS[:FF]: mov/mp4 write a tmcd track + stream tag,
    // mkv writes a TIMECODE format tag — dailies matching a camera slate
    if let Some(tc) = &args.timecode {
        if args.audio || !probe.has_video {
            return Err(Error::input("remux --timecode needs a video repack"));
        }
        let ok = tc.len() >= 8
            && tc.len() <= 11
            && tc
                .split([':', ';'])
                .all(|f| !f.is_empty() && f.chars().all(|c| c.is_ascii_digit()))
            && matches!(tc.matches([':', ';']).count(), 2 | 3);
        if !ok {
            return Err(Error::input(
                "remux --timecode: expected HH:MM:SS[:FF] (e.g. 01:00:00:00)",
            ));
        }
        argv.extend(["-timecode".to_string(), tc.clone()]);
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
    // --tag hvc1: codec-tag rewrite so HEVC mp4s play in QuickTime/Safari
    // (isom brand tag, stream copy — no re-encode)
    if let Some(t) = &args.tag {
        if !matches!(ext.as_str(), "mp4" | "mov") {
            return Err(Error::input("remux --tag needs an mp4/mov output"));
        }
        if args.audio || !probe.has_video {
            return Err(Error::input("remux --tag retags the video stream"));
        }
        if t.trim().is_empty() {
            return Err(Error::input("remux: empty --tag"));
        }
        argv.extend(["-tag:v", t.trim()]);
    }
    // --attach FILE: embed a binary attachment (matroska/webm only —
    // subtitle fonts travel inside the file with styled subs)
    for (i, att) in args.attach.iter().enumerate() {
        if !matches!(ext.as_str(), "mkv" | "webm") {
            return Err(Error::input(
                "remux --attach needs a matroska-family output (mkv/webm)",
            ));
        }
        crate::paths::ensure_input(att)?;
        argv.extend(["-attach"]);
        argv.push(att);
        let mime = match att
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase()
            .as_str()
        {
            "ttf" | "otf" => "application/x-truetype-font",
            "ttc" => "application/x-truetype-fonts",
            "srt" => "application/x-subrip",
            "ass" | "ssa" => "text/x-ssa",
            "pdf" => "application/pdf",
            _ => "application/octet-stream",
        };
        // indexed s:t:<i> — a bare s:t mimetype stamps every attachment
        argv.extend([format!("-metadata:s:t:{i}"), format!("mimetype={mime}")]);
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
    // --encrypt: CENC AES-CTR on the ISOBMFF essence — DRM prep
    // (ClearKey/Widevine/PlayReady); report key+kid so the caller can
    // wire them into their license/config
    let mut enc_kv: Option<(String, String)> = None;
    if encrypt {
        let key = match &args.key {
            Some(k) => k.clone(),
            None => crate::verbs::hls::random_key()?,
        };
        let kid = match &args.kid {
            Some(k) => k.clone(),
            None => crate::verbs::hls::random_key()?,
        };
        for v in [&key, &kid] {
            crate::verbs::hls::hex_decode(v)
                .map_err(|_| Error::input("remux --key/--kid must be 32 hex chars"))?;
        }
        argv.extend([
            "-encryption_scheme".to_string(),
            "cenc-aes-ctr".to_string(),
            "-encryption_key".to_string(),
            key.clone(),
            "-encryption_kid".to_string(),
            kid.clone(),
        ]);
        enc_kv = Some((key, kid));
    }
    if let Some(o) = args.offset {
        argv.extend(["-output_ts_offset".into(), o.to_string()]);
    }
    argv.push(&args.output);

    let run = engine::write_job("remux", &[&args.input], &args.output, vec![argv], g);
    if let Some(tmp) = &chap_file {
        let _ = std::fs::remove_file(tmp);
    }
    let c = run?;
    let mut c = c.with_extra(
        json!({ "container": ext, "audio_only": args.audio, "video_only": args.video, "fragmented": args.frag, "no_subs": args.no_subs, "from": args.from, "to": args.to, "lang": args.lang, "default_audio": args.default_audio, "cover": args.cover.is_some(), "no_cover": args.no_cover, "chapters": chap_n, "tags": tag_n, "audio_delay": args.audio_delay, "video_delay": args.video_delay, "tag": args.tag, "attached": args.attach.len(), "timecode": args.timecode, "default_sub": args.default_sub, "itsscale": args.itsscale, "offset": args.offset, "sub_order": args.sub_order }),
    );
    if let Some((key, kid)) = enc_kv {
        c = c.with_extra(json!({"encrypted": true, "key": key, "kid": kid}));
    }
    Ok(c)
}

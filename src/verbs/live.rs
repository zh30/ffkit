use std::path::{Path, PathBuf};

use serde_json::json;

use crate::cli::{Globals, LiveArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::spawn::Argv;

pub fn run(args: LiveArgs, g: &Globals) -> Result<Contract, Error> {
    let scheme = args.to.split("://").next().unwrap_or("").to_lowercase();
    if !matches!(scheme.as_str(), "rtmp" | "rtmps" | "tcp" | "udp" | "srt") {
        return Err(Error::input(
            "live --to needs an rtmp://, rtmps://, tcp://, udp://, or srt:// URL",
        ));
    }
    if args.test && args.list {
        return Err(Error::input("live --test and --list are exclusive"));
    }
    if args.test && args.input.is_some() {
        return Err(Error::input(
            "live --test streams a generated card — drop the input file",
        ));
    }
    if !args.test && args.input.is_none() {
        return Err(Error::input(
            "live needs an input file (or --test for the built-in card)",
        ));
    }
    // URL input: relay/re-stream a live source (rtmp:// pull, srt://,
    // udp://, http://, tcp://) — ffprobe/ffmpeg read it like a file.
    // File-only flags that seek or replay are refused below.
    let input_url = args
        .input
        .as_ref()
        .map(|p| p.to_string_lossy().contains("://"))
        .unwrap_or(false);
    if input_url {
        if args.list {
            return Err(Error::input(
                "live --list plays a manifest of files — not a URL input",
            ));
        }
        if args.start.is_some() {
            return Err(Error::input(
                "live --start seeks into a file — can't seek a live URL",
            ));
        }
        if args.loop_ {
            return Err(Error::input(
                "live --loop replays a file — a live URL can't loop",
            ));
        }
        if args.slate.is_some() {
            return Err(Error::input(
                "live --slate prepends a card to a file — not to a live URL",
            ));
        }
    }
    if let Some(t) = args.start {
        if !t.is_finite() || t <= 0.0 {
            return Err(Error::input("live --start needs seconds > 0"));
        }
        if args.list || args.test {
            return Err(Error::input(
                "live --start seeks a file — not a --list manifest or --test card",
            ));
        }
    }
    if args.list && args.input.is_none() {
        return Err(Error::input("live --list needs a manifest file"));
    }
    if args.slate.is_some() && args.test {
        return Err(Error::input("live --slate needs a real input — not --test"));
    }
    if args.slate.is_none() && args.slate_dur.is_some() {
        return Err(Error::input("live --slate-dur needs --slate"));
    }
    if let Some(t) = &args.title {
        if t.trim().is_empty() {
            return Err(Error::input("live --title is empty"));
        }
    }
    let slate_dur = args.slate_dur.unwrap_or(10.0);
    if args.slate.is_some() && (!slate_dur.is_finite() || slate_dur <= 0.0) {
        return Err(Error::input("live --slate-dur needs a positive duration"));
    }
    if let Some(s) = &args.slate {
        crate::paths::ensure_input(s)?;
    }

    // --list: probe the first manifest entry for stream shape (the manifest
    // itself is a text file ffprobe can't read)
    let mut list_files: Vec<PathBuf> = Vec::new();
    let (has_video, has_audio, pw, ph, pfps) = if args.test {
        (true, true, 1280, 720, 30.0)
    } else {
        let input = args.input.as_deref().unwrap_or_else(|| Path::new(""));
        let probe_src = if args.list {
            list_files = manifest_files(input)?;
            list_files[0].clone()
        } else {
            input.to_path_buf()
        };
        let probe = engine::probe_or_err(&probe_src, g)?;
        if !probe.has_video && !probe.has_audio {
            return Err(Error::input("live: input has no media streams"));
        }
        (
            probe.has_video,
            probe.has_audio,
            probe.width.unwrap_or(1280),
            probe.height.unwrap_or(720),
            probe.fps.unwrap_or(30.0),
        )
    };
    let mut has_video = has_video;
    let mut has_audio = has_audio;
    if args.audio_only && args.no_audio {
        return Err(Error::input(
            "live --audio-only and --no-audio empty the stream — pick one",
        ));
    }
    // --audio-only: strip the video path entirely (audio podcast / radio
    // push from any source — the concert file goes out aac-only)
    if args.audio_only {
        if !has_audio {
            return Err(Error::input("live --audio-only: input has no audio"));
        }
        for (flag, set) in [
            ("slate", args.slate.is_some()),
            ("card", args.card.is_some()),
            ("overlay", args.overlay.is_some()),
            ("subs", args.subs.is_some()),
            ("vertical", args.vertical),
            ("codec", args.codec.is_some()),
            ("gop", args.gop.is_some()),
            ("preset", args.preset.is_some()),
            ("scale", args.scale.is_some()),
        ] {
            if set {
                return Err(Error::input(format!(
                    "live --audio-only drops the video — --{flag} needs a video path"
                )));
            }
        }
        has_video = false;
    }
    if args.no_audio {
        if !has_video {
            return Err(Error::input("live --no-audio: input has no video"));
        }
        for (flag, set) in [
            ("card", args.card.is_some()),
            ("abitrate", args.abitrate.is_some()),
            ("volume", args.volume.is_some()),
            ("channels", args.channels.is_some()),
            ("audio-delay", args.audio_delay.is_some()),
            ("loudnorm", args.loudnorm),
        ] {
            if set {
                return Err(Error::input(format!(
                    "live --no-audio drops the audio — --{flag} needs an audio path"
                )));
            }
        }
        has_audio = false;
    }
    if let Some(v) = args.volume {
        if !has_audio {
            return Err(Error::input("live --volume: input has no audio"));
        }
        if !(0.0..=4.0).contains(&v) {
            return Err(Error::input("live --volume wants 0..=4"));
        }
    }
    if let Some(ch) = args.channels {
        if args.no_audio {
            return Err(Error::input("live --channels conflicts with --no-audio"));
        }
        if !has_audio {
            return Err(Error::input("live --channels: input has no audio"));
        }
        if ch == 0 || ch > 2 {
            return Err(Error::input("live --channels wants 1 (mono) or 2 (stereo)"));
        }
    }
    if let Some(d) = args.audio_delay {
        if args.no_audio {
            return Err(Error::input("live --audio-delay conflicts with --no-audio"));
        }
        if !has_audio {
            return Err(Error::input("live --audio-delay: input has no audio"));
        }
        if !d.is_finite() || d <= 0.0 || d > 300.0 {
            return Err(Error::input(
                "live --audio-delay wants a positive delay in seconds",
            ));
        }
    }
    if let Some(d) = args.hold {
        if !has_video {
            return Err(Error::input("live --hold: input has no video"));
        }
        if args.slate.is_some() || args.card.is_some() {
            return Err(Error::input("live --hold conflicts with --slate/--card"));
        }
        if !d.is_finite() || d <= 0.0 || d > 300.0 {
            return Err(Error::input(
                "live --hold wants a positive duration in seconds",
            ));
        }
    }
    if args.slate.is_some() && !has_video {
        return Err(Error::input(
            "live --slate needs a video stream to hold the card over",
        ));
    }
    if let Some(c) = &args.card {
        if args.test {
            return Err(Error::input(
                "live --card is a visual for a real input — not --test",
            ));
        }
        if args.slate.is_some() {
            return Err(Error::input(
                "live --card and --slate pick different cards — use one",
            ));
        }
        if has_video {
            return Err(Error::input(
                "live --card is for audio-only sources (use --overlay on video)",
            ));
        }
        if !has_audio {
            return Err(Error::input(
                "live --card with no audio is just a still — use --slate",
            ));
        }
        crate::paths::ensure_input(c)?;
        has_video = true; // the card supplies the video
    }
    if args.overlay.is_none() && (args.overlay_position.is_some() || args.overlay_opacity.is_some())
    {
        return Err(Error::input(
            "--overlay-position/--overlay-opacity need --overlay",
        ));
    }
    if let Some(op) = args.overlay_opacity {
        if !(0.0..=1.0).contains(&op) {
            return Err(Error::input("--overlay-opacity must be 0..=1"));
        }
    }
    if let Some(ov) = &args.overlay {
        if !has_video {
            return Err(Error::input(
                "live --overlay needs a video stream to pin the bug on",
            ));
        }
        crate::paths::ensure_input(ov)?;
    }

    if let Some(c) = args.crf {
        if c > 51 {
            return Err(Error::input("live --crf is 0..=51"));
        }
        if args.vbitrate.is_some() || args.maxrate.is_some() || args.bufsize.is_some() {
            return Err(Error::input(
                "live --crf is constant quality — drop --vbitrate/--maxrate/--bufsize",
            ));
        }
    }
    if let Some(t) = args.rw_timeout {
        if !t.is_finite() || t <= 0.0 {
            return Err(Error::input("live --timeout needs positive seconds"));
        }
        // -rw_timeout rides each destination's socket — the tee fan-out
        // can't carry it, so multi-destination pushes refuse it
        if args.record.is_some() || args.restream.is_some() {
            return Err(Error::input(
                "live --timeout applies to a single destination — drop --record/--restream",
            ));
        }
    }
    let vbitrate = args.vbitrate.as_deref().unwrap_or("2500k");
    let abitrate = args.abitrate.as_deref().unwrap_or("128k");
    // FLV for RTMP/plain-TCP ingest; MPEG-TS for UDP + SRT contribution links
    let fmt = if matches!(scheme.as_str(), "udp" | "srt") {
        "mpegts"
    } else {
        "flv"
    };
    let hevc = matches!(args.codec, Some(crate::cli::LiveCodec::Hevc));
    if hevc && fmt != "mpegts" {
        return Err(Error::input(
            "live --codec hevc needs an MPEG-TS transport (--to srt:// or udp://) — the FLV muxer can't carry it",
        ));
    }

    // canvas the stream renders at — slate card and content share it
    if args.vertical && args.scale.is_some() {
        return Err(Error::input(
            "--vertical already sets the canvas — drop --scale",
        ));
    }
    let (cw, ch) = if args.vertical {
        (1080, 1920)
    } else if let Some(sc) = &args.scale {
        let mut p = sc.split('x');
        match (
            p.next().and_then(|v| v.parse::<u32>().ok()),
            p.next().and_then(|v| v.parse::<u32>().ok()),
        ) {
            (Some(w), Some(h)) if w > 0 && h > 0 => (w, h),
            _ => return Err(Error::input("live --scale needs WxH like 1280x720")),
        }
    } else {
        (pw, ph)
    };
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-re");
    // slate goes first: looped still + silent bed, then the content inputs.
    // -stream_loop must precede the CONTENT -i (it binds to the next input)
    let mut ni = 0u32;
    if let Some(s) = &args.slate {
        argv.extend(["-loop", "1", "-t"]);
        argv.push(slate_dur.to_string());
        argv.push("-i");
        argv.push(s);
        ni = 1;
    }
    if let Some(c) = &args.card {
        argv.extend(["-loop", "1", "-i"]);
        argv.push(c);
        ni = 1;
    }
    if args.loop_ && !args.test {
        argv.extend(["-stream_loop", "-1"]);
    }
    if args.test {
        argv.extend([
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=1280x720:rate=30",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=1000:sample_rate=48000",
        ]);
    } else if args.list {
        argv.extend(["-f", "concat", "-safe", "0", "-i"]);
        argv.push(args.input.as_deref().unwrap_or_else(|| Path::new("")));
    } else {
        // input-side -ss: keyframe seek before -re pacing starts
        if let Some(t) = args.start {
            argv.extend(["-ss", &t.to_string()]);
        }
        argv.push("-i");
        argv.push(args.input.as_deref().unwrap_or_else(|| Path::new("")));
    }
    // channel bug rides its own looped input, last in the input list
    let li = ni + if args.test { 2 } else { 1 };
    if let Some(ov) = &args.overlay {
        argv.extend(["-loop", "1", "-i"]);
        argv.push(ov);
    }
    let use_fc = args.slate.is_some() || args.overlay.is_some() || args.hold.is_some();
    let mut subs_chain: Option<String> = None;
    if let Some(subs) = &args.subs {
        if !has_video {
            return Err(Error::input(
                "live --subs needs a video path (--card for audio-only sources)",
            ));
        }
        // The subtitles filter parses `:` `'` `,` in filenames — escape them.
        let path = subs
            .canonicalize()
            .map_err(|e| Error::input(format!("{subs:?}: {e}")))?
            .to_string_lossy()
            .replace('\\', "\\\\")
            .replace('\'', "\\'");
        subs_chain = Some(format!(",subtitles=filename='{path}'"));
    }
    let preset = match args.preset.unwrap_or_default() {
        crate::cli::X264Preset::Ultrafast => "ultrafast",
        crate::cli::X264Preset::Superfast => "superfast",
        crate::cli::X264Preset::Veryfast => "veryfast",
        crate::cli::X264Preset::Faster => "faster",
        crate::cli::X264Preset::Fast => "fast",
        crate::cli::X264Preset::Medium => "medium",
        crate::cli::X264Preset::Slow => "slow",
        crate::cli::X264Preset::Slower => "slower",
        crate::cli::X264Preset::Veryslow => "veryslow",
    };
    if has_video {
        // scale rides inside filter_complex when a graph is already in play;
        // --vertical always letterboxes onto the 1080x1920 canvas
        if !use_fc {
            let mut vf = String::new();
            if args.vertical {
                vf.push_str(&format!("scale={cw}:{ch}:force_original_aspect_ratio=decrease,pad={cw}:{ch}:(ow-iw)/2:(oh-ih)/2:black,setsar=1"));
            } else if args.scale.is_some() {
                vf.push_str(&format!("scale={cw}:{ch}"));
            }
            if let Some(sc) = &subs_chain {
                vf.push_str(sc);
            }
            let vf = vf.trim_start_matches(',').to_string();
            if !vf.is_empty() {
                argv.extend(["-vf".into(), vf]);
            }
        }
        argv.extend([
            "-c:v".to_string(),
            if hevc { "libx265" } else { "libx264" }.to_string(),
            "-preset".to_string(),
            preset.to_string(),
            "-tune".to_string(),
            "zerolatency".to_string(),
        ]);
        if let Some(c) = args.crf {
            argv.extend(["-crf".to_string(), c.to_string()]);
        } else {
            argv.extend(["-b:v".to_string(), vbitrate.to_string()]);
        }
        argv.extend(["-pix_fmt".to_string(), "yuv420p".to_string()]);
        if let Some(g) = args.gop {
            if g == 0 {
                return Err(Error::input("live --gop needs at least 1 frame"));
            }
            argv.extend(["-g".into(), g.to_string()]);
        }
        if let Some(mr) = &args.maxrate {
            if mr.is_empty() {
                return Err(Error::input("live --maxrate needs a rate like 4500k"));
            }
            argv.extend(["-maxrate".to_string(), mr.clone()]);
        }
        if args.maxrate.is_some() || args.bufsize.is_some() {
            // CBR pairing: bufsize defaults to 2x maxrate when unset
            let bs = match &args.bufsize {
                Some(b) => b.clone(),
                None => args
                    .maxrate
                    .as_deref()
                    .and_then(|r| {
                        let tail = r.chars().last().unwrap_or('0');
                        let (num, suffix) = if tail.is_alphabetic() {
                            r.split_at(r.len() - 1)
                        } else {
                            (r, "")
                        };
                        num.parse::<u64>()
                            .ok()
                            .map(|n| format!("{}{}", n * 2, suffix))
                    })
                    .ok_or_else(|| Error::input("live --bufsize needs a size like 9000k"))?,
            };
            argv.extend(["-bufsize".to_string(), bs]);
        }
        if let Some(fps) = args.fps {
            argv.extend(["-r".into(), fps.to_string()]);
        }
    } else {
        argv.push("-vn");
    }
    if has_audio {
        argv.extend([
            "-c:a".into(),
            "aac".into(),
            "-b:a".into(),
            abitrate.to_string(),
        ]);
        if let Some(ch) = args.channels {
            argv.extend(["-ac".into(), ch.to_string()]);
        }
    } else {
        argv.push("-an");
    }
    if let Some(until) = args.until {
        if !until.is_finite() || until <= 0.0 {
            return Err(Error::input(
                "live --until needs a positive duration in seconds",
            ));
        }
        argv.extend(["-t".into(), until.to_string()]);
    }
    // test mode maps its two lavfi inputs explicitly (video from input 0,
    // tone from input 1); card maps the still + the source's audio;
    // file inputs use the normal stream indexes
    let (vmap, amap) = if args.test || args.card.is_some() {
        ("0:v", "1:a")
    } else {
        ("0:v", "0:a")
    };
    // graph work: slate card concat and/or channel-bug overlay
    let mut fc = String::new();
    let fps = args.fps.map(|f| f as f64).unwrap_or(pfps);
    let vchain = format!(
        "scale={cw}:{ch}:force_original_aspect_ratio=decrease,pad={cw}:{ch}:(ow-iw)/2:(oh-ih)/2:black,setsar=1,fps={fps},format=yuv420p"
    );
    // subs burn applies to the content chain only, never the slate card;
    // tpad runs first so the held frame is the decoded first frame
    let tpad_chain = args
        .hold
        .map(|d| format!("tpad=start_duration={d}:start_mode=clone"));
    let mut vc_parts: Vec<String> = Vec::new();
    if let Some(t) = &tpad_chain {
        vc_parts.push(t.clone());
    }
    if !vchain.is_empty() {
        vc_parts.push(vchain.clone());
    }
    if let Some(sc) = subs_chain {
        vc_parts.push(sc);
    }
    let vchain_content = vc_parts.join(",");
    let vol_chain = {
        let mut s = args
            .volume
            .map(|v| format!(",volume={v}"))
            .unwrap_or_default();
        if let Some(d) = args.audio_delay {
            s.push_str(&format!(",adelay={}", (d * 1000.0).round() as u64));
        }
        if args.loudnorm {
            s.push_str(",loudnorm");
        }
        s
    };
    let mut vout = String::new();
    let mut aout: Option<String> = None;
    if args.slate.is_some() {
        fc.push_str(&format!("[0:v]{vchain}[vs];"));
        if has_audio {
            fc.push_str(&format!(
                "anullsrc=r=48000:cl=stereo,atrim=duration={slate_dur}[as];"
            ));
        }
        fc.push_str(&format!("[{ni}:v]{vchain_content}[vm];"));
        if has_audio {
            fc.push_str(&format!(
                "[{ni}:a]aresample=48000,aformat=channel_layouts=stereo{vol_chain}[am];[vs][as][vm][am]concat=n=2:v=1:a=1[vc][ac];"
            ));
            aout = Some("[ac]".to_string());
        } else {
            fc.push_str("[vs][vm]concat=n=2:v=1:a=0[vc];");
        }
        vout = "[vc]".to_string();
    } else if args.overlay.is_some() {
        // single-content base: the card's still, the test card, or the file
        let src = if args.test || args.card.is_some() {
            0
        } else {
            ni
        };
        if args.scale.is_some() || args.vertical || args.subs.is_some() || args.hold.is_some() {
            fc.push_str(&format!("[{src}:v]{vchain_content}[vc];"));
            vout = "[vc]".to_string();
        } else {
            vout = format!("[{src}:v]");
        }
    }
    if let Some(_ov) = &args.overlay {
        let op = args.overlay_opacity.unwrap_or(1.0);
        let lw = (cw / 10).max(32);
        let m = (ch * 3 / 100).max(8);
        let (x, y) = match args.overlay_position.unwrap_or(crate::cli::LogoPos::Br) {
            crate::cli::LogoPos::Tl => (format!("{m}"), format!("{m}")),
            crate::cli::LogoPos::Tr => (format!("W-w-{m}"), format!("{m}")),
            crate::cli::LogoPos::Bl => (format!("{m}"), format!("H-h-{m}")),
            crate::cli::LogoPos::Br => (format!("W-w-{m}"), format!("H-h-{m}")),
        };
        let mut lg = format!("[{li}:v]scale={lw}:-1");
        if op < 1.0 {
            lg.push_str(&format!(",format=rgba,colorchannelmixer=aa={op}"));
        }
        fc.push_str(&format!("{lg}[lg];{vout}[lg]overlay={x}:{y}[vo];"));
        vout = "[vo]".to_string();
    }
    if aout.is_none() && has_audio {
        let mut achain = String::new();
        if let Some(v) = args.volume {
            achain.push_str(&format!("volume={v}"));
        }
        if let Some(d) = args.audio_delay {
            if !achain.is_empty() {
                achain.push(',');
            }
            achain.push_str(&format!("adelay={}", (d * 1000.0).round() as u64));
        }
        // --hold pads the audio head too — a frozen picture over live audio
        // desyncs the whole stream
        if let Some(d) = args.hold {
            if !achain.is_empty() {
                achain.push(',');
            }
            achain.push_str(&format!("adelay={}", (d * 1000.0).round() as u64));
        }
        if args.loudnorm {
            if !achain.is_empty() {
                achain.push(',');
            }
            achain.push_str("loudnorm");
        }
        if !achain.is_empty() {
            fc.push_str(&format!("[{amap}]{achain}[avol];"));
            aout = Some("[avol]".to_string());
        }
    }
    // a bare --hold still needs a video graph — without slate/overlay no
    // arm above emits the content chain and the tpad would never reach argv.
    // vchain is always non-empty, so gate on --hold itself
    if args.hold.is_some() && vout.is_empty() && has_video {
        fc.push_str(&format!("[{vmap}]{vchain_content}[vc];"));
        vout = "[vc]".to_string();
    }
    if !fc.is_empty() {
        argv.extend(["-filter_complex", fc.trim_end_matches(';')]);
        // audio-only graphs leave vout empty — fall back to the raw video map
        if has_video {
            argv.extend(["-map", if vout.is_empty() { vmap } else { vout.as_str() }]);
        }
        if let Some(a) = &aout {
            argv.extend(["-map", a.as_str()]);
        } else if has_audio {
            argv.extend(["-map", amap]);
        }
    }
    if args.record.is_some() || args.restream.is_some() {
        // tee muxer doesn't do default stream selection — map explicitly,
        // then encode once and mux to every destination together
        let mut dests = format!("[f={fmt}]{}", args.to);
        if let Some(u2) = &args.restream {
            let s2 = u2.split("://").next().unwrap_or("");
            if !matches!(s2, "rtmp" | "rtmps" | "tcp" | "udp" | "srt") {
                return Err(Error::input(
                    "live --restream wants an rtmp/rtmps/tcp/udp/srt URL",
                ));
            }
            let fmt2 = if matches!(s2, "udp" | "srt") {
                "mpegts"
            } else {
                "flv"
            };
            dests.push_str(&format!("|[f={fmt2}]{u2}"));
        }
        if let Some(rec) = &args.record {
            let input_refs: Vec<&Path> = args.input.iter().map(|p| p.as_path()).collect();
            crate::paths::ensure_output_allowed(rec, &input_refs, g.overwrite)?;
            let rec_ext = rec
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            let rec_fmt = match rec_ext.as_str() {
                "mp4" | "mov" | "m4v" => "mp4",
                "mkv" => "matroska",
                "ts" | "mts" | "m2ts" => "mpegts",
                "flv" => "flv",
                _ => {
                    return Err(Error::input(
                        "live --record needs a media extension (mp4/mov/mkv/ts/flv)",
                    ));
                }
            };
            dests.push_str(&format!("|[f={rec_fmt}]{}", rec.display()));
        }
        if fc.is_empty() {
            if has_video {
                argv.extend(["-map", vmap]);
            }
            if has_audio {
                argv.extend(["-map", amap]);
            }
        }
        if let Some(t) = &args.title {
            argv.extend(["-metadata".into(), format!("title={t}")]);
        }
        argv.extend(["-f".to_string(), "tee".to_string(), dests]);
    } else {
        if args.test && fc.is_empty() {
            if has_video {
                argv.extend(["-map", vmap]);
            }
            if has_audio {
                argv.extend(["-map", amap]);
            }
        }
        if let Some(t) = &args.title {
            argv.extend(["-metadata".into(), format!("title={t}")]);
        }
        if let Some(t) = args.rw_timeout {
            argv.extend([
                "-rw_timeout".to_string(),
                ((t * 1_000_000.0) as u64).to_string(),
            ]);
        }
        argv.extend(["-f".into(), fmt.into(), args.to.clone()]);
    }

    let contract = stream_out("live", args.input.as_deref(), &args.to, vec![argv], g)?;
    Ok(contract.with_extra(json!({
        "to": args.to,
        "loop": args.loop_,
        "slate": args.slate.is_some(),
        "slate_dur": if args.slate.is_some() { slate_dur } else { 0.0 },
        "overlay": args.overlay.is_some(),
        "card": args.card.is_some(),
        "restream": args.restream,
        "vertical": args.vertical,
        "audio_only": args.audio_only,
        "no_audio": args.no_audio,
        "preset": preset,
        "codec": if hevc { "hevc" } else { "h264" },
        "gop": args.gop,
        "maxrate": args.maxrate,
        "rw_timeout": args.rw_timeout,
        "bufsize": args.bufsize,
        "crf": args.crf,
        "vbitrate": vbitrate,
        "abitrate": abitrate,
        "format": fmt,
        "record": args.record,
        "until": args.until,
        "subs": args.subs.is_some(),
        "start": args.start.unwrap_or(0.0),
        "list": args.list,
        "files": list_files.len(),
        "test": args.test,
        "title": args.title,
    })))
}

/// Parse an ffconcat manifest into resolved file paths (validated to exist).
/// Recognizes `file 'x'` / `file "x"` / `file x` lines; `#` comments skipped.
fn manifest_files(manifest: &Path) -> Result<Vec<PathBuf>, Error> {
    let text = std::fs::read_to_string(manifest)
        .map_err(|e| Error::input(format!("live --list: {manifest:?}: {e}")))?;
    let dir = manifest.parent().unwrap_or_else(|| Path::new("."));
    let mut files = Vec::new();
    for line in text.lines() {
        let l = line.trim();
        if l.is_empty() || l.starts_with('#') {
            continue;
        }
        if let Some(rest) = l.strip_prefix("file") {
            let name = rest.trim().trim_matches(|c| c == '\'' || c == '"');
            if !name.is_empty() {
                let p = PathBuf::from(name);
                files.push(if p.is_absolute() { p } else { dir.join(p) });
            }
        }
    }
    if files.is_empty() {
        return Err(Error::input(
            "live --list: no `file …` entries in the manifest",
        ));
    }
    for f in &files {
        crate::paths::ensure_input(f)?;
    }
    Ok(files)
}

/// Stream-output shared path for verbs that push a URL instead of writing a
/// file — write_job's exists/probe verification is file-only, so a stream is
/// verified by ffmpeg's exit status alone. Honors --dry-run.
pub(crate) fn stream_out(
    tool: &str,
    input: Option<&Path>,
    to: &str,
    argvs: Vec<Argv>,
    g: &Globals,
) -> Result<Contract, Error> {
    if let Some(i) = input {
        // URL inputs (rtmp/srt/udp/http/tcp) aren't files — ffmpeg opens them
        if !i.to_string_lossy().contains("://") {
            crate::paths::ensure_input(i)?;
        }
    }
    let inputs: Vec<&Path> = input.into_iter().collect();
    crate::paths::ensure_output_allowed(Path::new(to), &inputs, g.overwrite)?;
    let commands = engine::commands_of(&argvs);
    if g.dry_run {
        let p = input.and_then(|i| crate::probe::probe(i, std::time::Duration::from_secs(60)).ok());
        return Ok(Contract::dry_run(tool, Some(to.to_string()), p).with_commands(commands));
    }
    if let Err(e) = engine::run_argvs(&argvs, g) {
        return Ok(Contract::failed(tool, &e).with_commands(commands));
    }
    Ok(Contract::ok(tool, Some(to.to_string()), None).with_commands(commands))
}

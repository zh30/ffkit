use std::path::{Path, PathBuf};

use serde_json::json;

use crate::cli::{Globals, HlsArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::paths;

pub fn run(args: HlsArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_video && !probe.has_audio {
        return Err(Error::input("input has no media streams"));
    }
    if !(0.5..=30.0).contains(&args.seg) {
        return Err(Error::input("--seg must be 0.5..=30 seconds"));
    }
    paths::ensure_input(&args.input)?;

    // -o is the playlist name (or a directory → <dir>/index.m3u8).
    let (dir, playlist): (PathBuf, PathBuf) = if args.output.extension().is_some() {
        let dir = args
            .output
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();
        (dir, args.output.clone())
    } else {
        (args.output.clone(), args.output.join("index.m3u8"))
    };
    std::fs::create_dir_all(&dir)
        .map_err(|e| Error::output(format!("create {}: {e}", paths::display(&dir))))?;
    if args.time_names && args.single {
        return Err(Error::input(
            "hls --time-names doesn't apply to --single (one file, no names)",
        ));
    }
    if args.independent && args.copy {
        return Err(Error::input(
            "hls --independent can't force keyframes on a --copy repack",
        ));
    }
    if args.independent && !args.ladder.is_empty() {
        return Err(Error::input(
            "hls --independent doesn't apply to --ladder (per-rung GOPs)",
        ));
    }
    if let Some(u) = &args.base_url {
        let u = u.trim();
        if u.is_empty() || u.contains(char::is_whitespace) {
            return Err(Error::input("hls --base-url needs a URL prefix"));
        }
    }
    if let Some(n) = &args.name {
        let n = n.trim();
        if n.is_empty()
            || !n
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err(Error::input(
                "hls --name takes letters/digits/-/_ only (no pattern chars or path separators)",
            ));
        }
    }
    let name_pfx = args
        .name
        .as_deref()
        .map(|n| format!("{n}-"))
        .unwrap_or_default();
    let seg_ext = if args.fmp4 { "m4s" } else { "ts" };
    let seg_tpl = if args.single {
        dir.join(format!("{name_pfx}seg.{seg_ext}"))
    } else if args.time_names {
        dir.join(format!("{name_pfx}seg_%Y%m%d-%H%M%S.{seg_ext}"))
    } else {
        dir.join(format!("{name_pfx}seg_%03d.{seg_ext}"))
    };

    // --encrypt/--key: AES-128 segment encryption via -hls_key_info_file.
    let key_info = if args.encrypt || args.key.is_some() {
        let key_hex = match &args.key {
            Some(k) => {
                let k = k.trim().to_lowercase();
                if k.len() != 32 || !k.chars().all(|c| c.is_ascii_hexdigit()) {
                    return Err(Error::input("--key must be 32 hex chars"));
                }
                k
            }
            None => random_key()?,
        };
        let key_path = dir.join("key.bin");
        let raw = hex_decode(&key_hex)?;
        std::fs::write(&key_path, &raw)
            .map_err(|e| Error::output(format!("write {}: {e}", paths::display(&key_path))))?;
        let info_path = dir.join("key.info");
        let uri = args.key_uri.as_deref().unwrap_or("key.bin");
        std::fs::write(
            &info_path,
            format!("{uri}\n{}\n", paths::display(&key_path)),
        )
        .map_err(|e| Error::output(format!("write {}: {e}", paths::display(&info_path))))?;
        Some((info_path, uri.to_string()))
    } else {
        if args.key_uri.is_some() {
            return Err(Error::input("--key-uri needs --encrypt or --key"));
        }
        if args.rekey {
            return Err(Error::input(
                "hls --rekey rotates key material — it needs --encrypt or --key",
            ));
        }
        None
    };
    let key_args: Vec<String> = key_info
        .as_ref()
        .map(|(p, _)| vec!["-hls_key_info_file".to_string(), p.display().to_string()])
        .unwrap_or_default();
    if args.live_window.is_some() && !args.live {
        return Err(Error::input("--live-window needs --live"));
    }
    if args.epoch && args.start.is_some() {
        return Err(Error::input(
            "--epoch derives the start index itself — drop --start",
        ));
    }
    if args.live {
        if args.single {
            return Err(Error::input(
                "--live slides a multi-segment window — can't combine with --single",
            ));
        }
        if !args.ladder.is_empty() {
            return Err(Error::input(
                "--live keeps one sliding window — drop --ladder",
            ));
        }
        if let Some(w) = args.live_window {
            if w == 0 {
                return Err(Error::input("--live-window needs at least 1 segment"));
            }
        }
    }

    let mut argv = ffmpeg_base(g.progress);
    argv.extend(["-i".to_string(), args.input.display().to_string()]);
    // --program N: package one service out of a multi-program transport
    // stream. Members come from probe.programs[] — stream presence is
    // then scoped to the service, not the file (a radio service has no
    // video members even when the mux carries others).
    let prog_members = if let Some(n) = args.program {
        if n == 0 {
            return Err(Error::input(
                "hls --program: program numbers are 1-based (see probe.programs[])",
            ));
        }
        if probe.programs.is_empty() {
            return Err(Error::input(
                "hls --program: input carries no programs (see probe.programs[])",
            ));
        }
        let m = probe.program_members(n).ok_or_else(|| {
            let avail = probe
                .programs
                .iter()
                .map(|p| p.num.to_string())
                .collect::<Vec<_>>()
                .join(",");
            Error::input(format!(
                "hls --program {n} not in input (programs: {avail})"
            ))
        })?;
        Some(m)
    } else {
        None
    };
    let sel_has_video = match &prog_members {
        Some((v, _, _)) => !v.is_empty(),
        None => probe.has_video,
    };
    let sel_has_audio = match &prog_members {
        Some((_, a, _)) => !a.is_empty(),
        None => probe.has_audio,
    };
    if let Some(n) = args.program {
        if args.ladder.is_empty() {
            argv.extend(["-map".to_string(), format!("0:p:{n}")]);
        }
    }
    if args.audio_only {
        if args.copy || !args.ladder.is_empty() {
            return Err(Error::input(
                "--audio-only doesn't combine with --copy/--ladder",
            ));
        }
        if !sel_has_audio {
            return Err(Error::input("--audio-only needs an audio stream"));
        }
    }
    if args.video_only {
        if args.audio_only {
            return Err(Error::input("--video-only conflicts with --audio-only"));
        }
        if !sel_has_video {
            return Err(Error::input("--video-only needs a video stream"));
        }
    }
    if args.master.is_some() && args.ladder.is_empty() {
        return Err(Error::input(
            "--master names the --ladder master playlist — single playlists are already named by -o",
        ));
    }
    if !args.ladder.is_empty() {
        // ABR ladder: N variants at tiered bitrates, one audio, master.m3u8.
        if args.copy || args.single {
            return Err(Error::input(
                "--ladder doesn't combine with --copy/--single",
            ));
        }
        if !sel_has_video {
            return Err(Error::input("--ladder needs a video stream"));
        }
        if args.output.extension().is_some() {
            return Err(Error::input("--ladder needs a directory -o"));
        }
        let mut hs = args.ladder.clone();
        hs.sort_unstable_by(|a, b| b.cmp(a));
        hs.dedup();
        if hs.len() < 2 || hs.len() > 6 {
            return Err(Error::input("--ladder needs 2..=6 heights"));
        }
        let n = hs.len();
        let bitrate = |h: u32| -> u32 {
            if h >= 1080 {
                4500
            } else if h >= 720 {
                2800
            } else if h >= 480 {
                1400
            } else {
                800
            }
        };
        let mut fc = String::new();
        let vsrc = match &prog_members {
            // feed the ladder from the service's own video member, not
            // the file's default program
            Some((v, _, _)) => format!("0:{}", v[0]),
            None => "0:v".to_string(),
        };
        fc.push_str(&format!(
            "[{vsrc}]split={n}{}",
            (0..n).map(|i| format!("[sp{i}]")).collect::<String>()
        ));
        for (i, h) in hs.iter().enumerate() {
            fc.push_str(&format!(";[sp{i}]scale=-2:{h}[lv{i}]"));
        }
        argv.extend(["-filter_complex".to_string(), fc]);
        for (i, h) in hs.iter().enumerate() {
            let r = bitrate(*h);
            argv.extend([
                "-map".to_string(),
                format!("[lv{i}]"),
                format!("-c:v:{i}"),
                "libx264".to_string(),
                format!("-preset:v:{i}"),
                "veryfast".to_string(),
                format!("-b:v:{i}"),
                format!("{r}k"),
                format!("-maxrate:v:{i}"),
                format!("{r}k"),
                format!("-bufsize:v:{i}"),
                format!("{}k", r * 2),
                format!("-pix_fmt:v:{i}"),
                "yuv420p".to_string(),
            ]);
        }
        if sel_has_audio && !args.video_only {
            // ffmpeg <7 hls: an elementary stream may appear in only one
            // variant group — so each variant gets its own aac encode.
            let asrc = match &prog_members {
                // the service's first audio member (main program audio)
                Some((_, a, _)) => format!("0:{}", a[0]),
                None => "0:a".to_string(),
            };
            for i in 0..n {
                argv.extend([
                    "-map".to_string(),
                    asrc.clone(),
                    format!("-c:a:{i}"),
                    "aac".to_string(),
                    format!("-b:a:{i}"),
                    "128k".to_string(),
                ]);
            }
        }
        let varmap = if sel_has_audio && !args.video_only {
            (0..n)
                .map(|i| format!("v:{i},a:{i}"))
                .collect::<Vec<_>>()
                .join(" ")
        } else {
            (0..n)
                .map(|i| format!("v:{i}"))
                .collect::<Vec<_>>()
                .join(" ")
        };
        let master_name = args
            .master
            .clone()
            .unwrap_or_else(|| "master.m3u8".to_string());
        argv.extend([
            "-f".to_string(),
            "hls".to_string(),
            "-hls_time".to_string(),
            format!("{:.3}", args.seg),
            "-hls_playlist_type".to_string(),
            "vod".to_string(),
            "-hls_segment_filename".to_string(),
            dir.join(format!("{name_pfx}seg_%v_%03d.{seg_ext}"))
                .display()
                .to_string(),
            "-master_pl_name".to_string(),
            master_name.clone(),
            "-var_stream_map".to_string(),
            varmap,
        ]);
        argv.extend(key_args.iter().cloned());
        argv.push(dir.join("v%v.m3u8").display().to_string());

        let commands = engine::commands_of(std::slice::from_ref(&argv));
        if g.dry_run {
            return Ok(Contract::dry_run(
                "hls",
                Some(paths::display(&dir.join(&master_name))),
                Some(probe),
            )
            .with_commands(commands));
        }
        if let Err(e) = engine::run_argvs(&[argv], g) {
            return Ok(Contract::failed("hls", &e).with_commands(commands));
        }
        let master = dir.join(&master_name);
        if !master.is_file() {
            return Err(Error::verification(format!(
                "hls --ladder finished but {master_name} is missing"
            )));
        }
        let variants = hs
            .iter()
            .enumerate()
            .map(|(i, h)| format!("v{i}.m3u8 ({h}p)"))
            .collect::<Vec<_>>();
        let mut c = Contract::ok(
            "hls",
            Some(paths::display(&master)),
            crate::probe::probe(&master, std::time::Duration::from_secs(60)).ok(),
        )
        .with_commands(commands);
        let mut extra = json!({
            "playlist": paths::display(&master),
            "variants": variants,
            "segment_seconds": args.seg,
            "master": master_name,
        });
        if let Some((p, uri)) = &key_info {
            extra["key_uri"] = json!(uri);
            extra["key_info"] = json!(paths::display(p));
        }
        c = c.with_extra(extra);
        return Ok(c);
    }
    if args.copy {
        argv.extend([
            "-c:v".to_string(),
            "copy".to_string(),
            "-bsf:v".to_string(),
            "h264_mp4toannexb".to_string(),
        ]);
        if sel_has_audio && !args.video_only {
            argv.extend(["-c:a".to_string(), "copy".to_string()]);
        }
    } else {
        if sel_has_video && !args.audio_only {
            argv.extend([
                "-vf".to_string(),
                "scale=trunc(iw/2)*2:trunc(ih/2)*2".to_string(),
                "-c:v".to_string(),
                "libx264".to_string(),
                "-preset".to_string(),
                "veryfast".to_string(),
                "-crf".to_string(),
                "20".to_string(),
                "-pix_fmt".to_string(),
                "yuv420p".to_string(),
            ]);
        }
        if sel_has_audio && !args.video_only {
            argv.extend([
                "-c:a".to_string(),
                "aac".to_string(),
                "-b:a".to_string(),
                "128k".to_string(),
            ]);
        }
        if args.audio_only {
            argv.extend(["-vn".to_string()]);
        }
    }
    if args.video_only {
        argv.extend(["-an".to_string()]);
    }
    argv.extend([
        "-f".to_string(),
        "hls".to_string(),
        "-hls_time".to_string(),
        format!("{:.3}", args.seg),
        "-hls_playlist_type".to_string(),
        if args.live { "event" } else { "vod" }.to_string(),
        "-hls_segment_filename".to_string(),
        seg_tpl.display().to_string(),
    ]);
    // -hls_flags is single-valued — collect every flag and +-join them
    let mut flags: Vec<&str> = Vec::new();
    if args.single {
        flags.push("single_file");
    }
    if args.live {
        // sliding window: players see only the newest N segments and no
        // end tag — the playlist stays joinable mid-write
        flags.push("delete_segments");
        flags.push("omit_endlist");
    }
    if args.date {
        flags.push("program_date_time");
    }
    if args.discontinuity {
        flags.push("discont_start");
    }
    if args.independent {
        flags.push("independent_segments");
    }
    if args.iframes {
        flags.push("iframes_only");
    }
    if args.rekey {
        flags.push("periodic_rekey");
    }
    if args.temp {
        flags.push("temp_file");
    }
    if args.round_durations {
        flags.push("round_durations");
    }
    if !flags.is_empty() {
        argv.extend(["-hls_flags".to_string(), flags.join("+")]);
    }
    if args.time_names {
        argv.extend(["-strftime".to_string(), "1".to_string()]);
    }
    if args.independent {
        // every segment must start on a keyframe for the tag to be true
        argv.extend([
            "-force_key_frames".to_string(),
            format!("expr:gte(t,n_forced*{})", args.seg),
        ]);
    }
    if args.live {
        let win = args.live_window.unwrap_or(6);
        argv.extend(["-hls_list_size".to_string(), win.to_string()]);
    }
    if let Some(n) = args.start {
        argv.extend(["-start_number".to_string(), n.to_string()]);
    }
    if args.epoch {
        argv.extend(["-hls_start_number_source".to_string(), "epoch".to_string()]);
    }
    if args.fmp4 {
        argv.extend(["-hls_segment_type".to_string(), "fmp4".to_string()]);
    }
    if let Some(u) = &args.base_url {
        argv.extend(["-hls_base_url".to_string(), u.trim().to_string()]);
    }
    argv.extend(key_args.iter().cloned());
    argv.push(playlist.display().to_string());

    if args.poster_at.is_some() && !args.poster {
        return Err(Error::input("--poster-at needs --poster"));
    }
    let poster_path = dir.join("poster.jpg");
    let mut argvs = vec![argv];
    if args.poster {
        if !probe.has_video {
            return Err(Error::input("--poster needs a video stream"));
        }
        let at = match &args.poster_at {
            Some(raw) => crate::time::resolve_frame_at(raw.trim(), probe.duration)?,
            None => probe.duration / 2.0,
        }
        .min((probe.duration - 0.1).max(0.0));
        let mut pargv = ffmpeg_base(g.progress);
        pargv.extend(["-ss".to_string(), format!("{at:.3}")]);
        pargv.extend(["-i".to_string(), args.input.display().to_string()]);
        pargv.extend(["-frames:v".to_string(), "1".to_string()]);
        pargv.extend(["-q:v".to_string(), "3".to_string()]);
        pargv.push(poster_path.display().to_string());
        argvs.push(pargv);
    }

    // Outputs are a dir of segments + a playlist — manual contract like split.
    let commands = engine::commands_of(&argvs);
    if g.dry_run {
        return Ok(
            Contract::dry_run("hls", Some(paths::display(&playlist)), Some(probe))
                .with_commands(commands),
        );
    }
    if let Err(e) = engine::run_argvs(&argvs, g) {
        return Ok(Contract::failed("hls", &e).with_commands(commands));
    }
    if args.poster && !poster_path.is_file() {
        return Err(Error::verification(
            "hls finished but poster.jpg was not written",
        ));
    }
    let nseg = std::fs::read_dir(&dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .filter(|e| {
                    let n = e.file_name().to_string_lossy().into_owned();
                    (n.starts_with(&format!("{name_pfx}seg_"))
                        || n == format!("{name_pfx}seg.ts")
                        || n == format!("{name_pfx}seg.m4s"))
                        && e.path()
                            .extension()
                            .is_some_and(|x| x == "ts" || x == "m4s")
                })
                .count()
        })
        .unwrap_or(0);
    if nseg == 0 || !playlist.is_file() {
        return Err(Error::verification(
            "hls finished but no segments/playlist were written",
        ));
    }
    let mut c = Contract::ok(
        "hls",
        Some(paths::display(&playlist)),
        crate::probe::probe(&playlist, std::time::Duration::from_secs(60)).ok(),
    )
    .with_commands(commands);
    c = c.with_extra(json!({
        "playlist": paths::display(&playlist),
        "segments": nseg,
        "segment_seconds": args.seg,
        "live": args.live,
        "start": args.start,
        "epoch": args.epoch,
        "date": args.date,
        "discontinuity": args.discontinuity,
        "time_names": args.time_names,
        "independent": args.independent,
        "iframes": args.iframes,
        "video_only": args.video_only,
        "base_url": args.base_url,
        "live_window": if args.live { args.live_window.unwrap_or(6) } else { 0 },
    }));
    if let Some((p, uri)) = &key_info {
        c = c.with_extra(json!({"key_uri": uri, "key_info": paths::display(p)}));
    }
    Ok(c)
}

pub(crate) fn random_key() -> Result<String, Error> {
    use std::io::Read;
    let mut buf = [0u8; 16];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut buf))
        .map_err(|e| Error::output(format!("--encrypt cannot read /dev/urandom: {e}")))?;
    Ok(buf.iter().map(|b| format!("{b:02x}")).collect())
}

pub(crate) fn hex_decode(s: &str) -> Result<Vec<u8>, Error> {
    (0..s.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&s[i..i + 2], 16)
                .map_err(|_| Error::input("--key must be 32 hex chars"))
        })
        .collect()
}

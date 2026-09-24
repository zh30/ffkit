use serde_json::json;

use crate::cli::{Globals, ScanArgs};
use crate::contract::Contract;
use crate::engine;
use crate::error::Error;
use crate::paths;
use crate::spawn::{self, Argv};

/// QC pass: report black stretches, frozen frames and per-black-frame hits.
/// Report-only — writes no media, findings land in `extras`.
pub fn run(args: ScanArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_video && !probe.has_audio {
        return Err(Error::input("scan: input has no media streams"));
    }
    // audio-only inputs still get the audio QC legs (loud/deadair/levels);
    // explicitly-requested video legs stay an error so a mistyped file
    // doesn't silently pass QC
    if !probe.has_video
        && (args.scenes
            || args.motion
            || args.timecode
            || args.bbox
            || args.text
            || args.gop
            || args.dupe.is_some())
    {
        return Err(Error::input("scan: input has no video stream"));
    }
    let freeze_min = args.freeze_min.unwrap_or(1.0);
    let black_min = args.black_min.unwrap_or(0.3);
    let thresh = args.thresh.unwrap_or(32.0).clamp(0.0, 255.0);

    // photosensitivity rides the same pass: bypass=1 keeps frames intact and
    // emits lavfi.photosensitivity.* metadata; metadata=print mirrors it to the
    // log where we count flash-flagged frames (badness > 0)
    let scdet_leg = if args.scenes { ",scdet=t=8" } else { "" };
    // --motion: vmafmotion writes lavfi.vmafmotion.score per frame — bitrate
    // budget QC (static ≈0, busy action ≈7+)
    let motion_leg = if args.motion { ",vmafmotion" } else { "" };
    // --timecode: readvitc reads embedded VITC lines — lavfi.readvitc.found
    // / .tc_str per frame (broadcast master QC)
    let vitc_leg = if args.timecode { ",readvitc" } else { "" };
    // --bbox: bbox marks the non-background content box per frame —
    // lavfi.bbox.* on the frame metadata; min_val=20 keeps noise floor
    // pixels from counting as content (cropdetect only catches BLACK
    // borders — bbox sees content on any uniform background)
    let bbox_leg = if args.bbox { ",bbox=min_val=20" } else { "" };
    // vfrdet closes the chain: it consumes every frame's timestamps and
    // prints one `VFR:<ratio> (<n>/<N>)` line to stderr at EOF — screen
    // recordings / edit-joined captures come back nonzero
    let vf = format!(
        "blackdetect=d={black_min}:pic_th=0.98,blackframe=thresh={thresh:.0}:amount=98,freezedetect=d={freeze_min},photosensitivity=bypass=1,idet,signalstats,entropy=mode=diff,bitplanenoise{scdet_leg},readeia608,cropdetect=limit=24:round=2{bbox_leg}{motion_leg}{vitc_leg},metadata=print:file=-,vfrdet"
    );
    let mut argv = Argv::ffmpeg();
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-vf", &vf, "-an", "-f", "null", "-"]);
    let spawned = if probe.has_video {
        let sp = spawn::run(&argv, g.timeout, false)?;
        Some(spawn::require_ok(&argv, sp)?)
    } else {
        None
    };
    // detect logs land on stderr; metadata=print:file=- writes to stdout
    let log = spawned
        .as_ref()
        .map(|s| {
            format!(
                "{}\n{}",
                spawn::stderr_str(s),
                spawn::stdout_str(s).unwrap_or("")
            )
        })
        .unwrap_or_default();

    let mut black_ranges: Vec<serde_json::Value> = Vec::new();
    let mut freeze_starts: Vec<f64> = Vec::new();
    let mut freeze_ends: Vec<f64> = Vec::new();
    let mut black_frames = 0usize;
    let mut flash_frames = 0usize;
    let mut idet_counts = (0usize, 0usize, 0usize, 0usize); // tff, bff, prog, undet
    let mut flash_max = 0.0f64;
    let mut entropy_vals: Vec<f64> = Vec::new();
    let mut luma_min = f64::MAX;
    let mut luma_max = f64::MIN;
    let mut sat_vals: Vec<f64> = Vec::new();
    let mut hue_vals: Vec<f64> = Vec::new();
    let mut y_vals: Vec<f64> = Vec::new();
    let mut scene_cuts: Vec<f64> = Vec::new();
    let mut noise_vals: Vec<f64> = Vec::new();
    let mut cc_lines = 0usize;
    let mut motion_vals: Vec<f64> = Vec::new();
    let mut vitc_frames = 0usize;
    let mut vitc_tc: Option<String> = None;
    let mut vfr_ratio: Option<f64> = None;
    let mut vfr_frames = 0usize;
    let (mut cd_x1, mut cd_x2, mut cd_y1, mut cd_y2) = (-1i64, -1i64, -1i64, -1i64);
    // bbox union bounds: x1/y1 start huge and shrink, x2/y2 start tiny and grow
    let (mut bb_x1, mut bb_x2, mut bb_y1, mut bb_y2) = (i64::MAX, i64::MIN, i64::MAX, i64::MIN);
    for line in log.lines() {
        if let Some(rest) = line.split("VFR:").nth(1) {
            // `VFR:0.013514 (1/73)` — fraction of frames at non-CFR intervals
            vfr_ratio = rest
                .split_whitespace()
                .next()
                .and_then(|t| t.parse::<f64>().ok());
            vfr_frames = rest
                .split('(')
                .nth(1)
                .and_then(|r| r.split('/').next())
                .and_then(|t| t.trim().parse::<usize>().ok())
                .unwrap_or(0);
        }
        if let Some(rest) = line.split("black_start:").nth(1) {
            let s = rest
                .split_whitespace()
                .next()
                .unwrap_or("")
                .parse::<f64>()
                .unwrap_or(0.0);
            let e = rest
                .split("black_end:")
                .nth(1)
                .and_then(|r| r.split_whitespace().next())
                .and_then(|t| t.parse::<f64>().ok())
                .unwrap_or(s);
            black_ranges.push(json!({"start": s, "end": e, "duration": e - s}));
        }
        if let Some(rest) = line.split("freeze_start:").nth(1) {
            if let Ok(v) = rest.trim().parse::<f64>() {
                freeze_starts.push(v);
            }
        }
        if let Some(rest) = line.split("freeze_end:").nth(1) {
            if let Ok(v) = rest.trim().parse::<f64>() {
                freeze_ends.push(v);
            }
        }
        if let Some(rest) = line
            .split("lavfi.scd.time=")
            .nth(1)
            .or_else(|| line.split("lavfi.scd.time:").nth(1))
        {
            if let Ok(t) = rest.trim().split(' ').next().unwrap_or("").parse::<f64>() {
                scene_cuts.push(t);
            }
        }
        if let Some(rest) = line.split("lavfi.signalstats.YMIN=").nth(1) {
            if let Ok(v) = rest.trim().split(' ').next().unwrap_or("").parse::<f64>() {
                luma_min = luma_min.min(v);
            }
        }
        if let Some(rest) = line.split("lavfi.signalstats.YMAX=").nth(1) {
            if let Ok(v) = rest.trim().split(' ').next().unwrap_or("").parse::<f64>() {
                luma_max = luma_max.max(v);
            }
        }
        if let Some(rest) = line.split("lavfi.signalstats.SATAVG=").nth(1) {
            if let Ok(v) = rest.trim().split(' ').next().unwrap_or("").parse::<f64>() {
                sat_vals.push(v);
            }
        }
        if let Some(rest) = line.split("lavfi.signalstats.HUEAVG=").nth(1) {
            if let Ok(v) = rest.trim().split(' ').next().unwrap_or("").parse::<f64>() {
                hue_vals.push(v);
            }
        }
        if let Some(rest) = line.split("lavfi.signalstats.YAVG=").nth(1) {
            if let Ok(v) = rest.trim().split(' ').next().unwrap_or("").parse::<f64>() {
                y_vals.push(v);
            }
        }
        if let Some(rest) = line.split("normalized_entropy.diff.Y=").nth(1) {
            if let Ok(v) = rest.trim().split(' ').next().unwrap_or("").parse::<f64>() {
                entropy_vals.push(v);
            }
        }
        // bitplanenoise LSB occupancy per plane: ~0.98+ on grainy/noisy
        // footage, ~0.3-0.5 on clean — the compression-prep "will this
        // devour bitrate" signal
        if let Some(eq) = line.find("lavfi.bitplanenoise.") {
            if let Some(v) = line[eq..]
                .split('=')
                .nth(1)
                .and_then(|s| s.trim().split(' ').next())
                .and_then(|s| s.parse::<f64>().ok())
            {
                noise_vals.push(v);
            }
        }
        // EIA-608 closed captions: any lavfi.readeia608.* key means a CC
        // line was decoded this frame (broadcast master QC)
        if line.contains("lavfi.readeia608.") {
            cc_lines += 1;
        }
        if let Some(rest) = line.split("lavfi.vmafmotion.score=").nth(1) {
            if let Ok(v) = rest.trim().split(' ').next().unwrap_or("").parse::<f64>() {
                motion_vals.push(v);
            }
        }
        if let Some(rest) = line.split("lavfi.readvitc.found=").nth(1) {
            if rest.trim().starts_with('1') {
                vitc_frames += 1;
            }
        }
        if let Some(rest) = line.split("lavfi.readvitc.tc_str=").nth(1) {
            let tc = rest.trim().split(' ').next().unwrap_or("").to_string();
            if !tc.is_empty() {
                vitc_tc = Some(tc);
            }
        }
        // cropdetect letterbox QC: x1/x2/y1/y2 bounds of non-black content
        for (k, slot) in [
            ("lavfi.cropdetect.x1=", &mut cd_x1),
            ("lavfi.cropdetect.x2=", &mut cd_x2),
            ("lavfi.cropdetect.y1=", &mut cd_y1),
            ("lavfi.cropdetect.y2=", &mut cd_y2),
        ] {
            if let Some(v) = line
                .split(k)
                .nth(1)
                .and_then(|s| s.trim().split(' ').next())
                .and_then(|s| s.parse::<i64>().ok())
            {
                *slot = v;
            }
        }
        // --bbox union: grow the content box across every frame's
        // lavfi.bbox.x1/x2/y1/y2 so one number covers a moving object
        for (k, hi, slot) in [
            ("lavfi.bbox.x1=", false, &mut bb_x1),
            ("lavfi.bbox.x2=", true, &mut bb_x2),
            ("lavfi.bbox.y1=", false, &mut bb_y1),
            ("lavfi.bbox.y2=", true, &mut bb_y2),
        ] {
            if let Some(v) = line
                .split(k)
                .nth(1)
                .and_then(|s| s.trim().split(' ').next())
                .and_then(|s| s.parse::<i64>().ok())
            {
                *slot = if hi { (*slot).max(v) } else { (*slot).min(v) };
            }
        }
        if line.contains("pblack:") {
            black_frames += 1;
        }
        if line.contains("Single frame detection:") {
            let grab = |tag: &str| -> usize {
                line.split(tag)
                    .nth(1)
                    .and_then(|r| r.trim_start_matches(':').split_whitespace().next())
                    .and_then(|v| v.parse::<usize>().ok())
                    .unwrap_or(0)
            };
            idet_counts = (
                grab("TFF"),
                grab("BFF"),
                grab("Progressive"),
                grab("Undetermined"),
            );
        }
        if let Some(rest) = line.split("lavfi.photosensitivity.badness=").nth(1) {
            let b = rest.trim().parse::<f64>().unwrap_or(0.0);
            // badness ~1.8 on smoothly animated content, ~3+ on real strobes
            if b > 2.0 {
                flash_frames += 1;
                flash_max = flash_max.max(b);
            }
        }
    }
    // stereo mono-compat QC: Pearson r between L/R decoded in one extra pass.
    // Near +1 = mono-like (fine), near 0 = decorrelated, <0 = out-of-phase —
    // the last one collapses on mono speakers (podcast/phone playback).
    let phase_corr = if probe.channels == Some(2) {
        // pipe-buffer deadlock guard: PCM exceeds the 64KB pipe fast, and
        // spawn::run waits for exit before draining — write to a temp file
        let tmp = match tempfile::NamedTempFile::new() {
            Ok(t) => t,
            Err(_) => {
                return Ok(
                    Contract::ok("scan", None, Some(probe)).with_extra(json!({"phase_corr": null}))
                )
            }
        };
        let pcm_path = tmp.path().to_path_buf();
        let mut argv = Argv::ffmpeg();
        argv.push("-i");
        argv.push(&args.input);
        argv.extend([
            "-vn",
            "-f",
            "s16le",
            "-acodec",
            "pcm_s16le",
            "-t",
            "10",
            "-y",
        ]);
        argv.push(&pcm_path);
        spawn::run(&argv, g.timeout, false)
            .ok()
            .filter(|sp| sp.status_ok)
            .and_then(|_| std::fs::read(&pcm_path).ok())
            .map(|pcm| {
                let pcm = &pcm[..];
                let mut sx = 0f64;
                let mut sy = 0f64;
                let mut sxx = 0f64;
                let mut syy = 0f64;
                let mut sxy = 0f64;
                let mut n = 0usize;
                // cap at ~10s of 44.1k stereo to bound CPU
                for pair in pcm.chunks_exact(4).take(44100 * 10) {
                    let l = i16::from_le_bytes([pair[0], pair[1]]) as f64;
                    let r = i16::from_le_bytes([pair[2], pair[3]]) as f64;
                    sx += l;
                    sy += r;
                    sxx += l * l;
                    syy += r * r;
                    sxy += l * r;
                    n += 1;
                }
                if n == 0 {
                    return None;
                }
                let nf = n as f64;
                let num = sxy - sx * sy / nf;
                let den = ((sxx - sx * sx / nf) * (syy - sy * sy / nf)).sqrt();
                if den > 0.0 {
                    Some((num / den * 1000.0).round() / 1000.0)
                } else {
                    Some(1.0)
                }
            })
            .and_then(|x| x)
    } else {
        None
    };

    let freeze_ranges: Vec<serde_json::Value> = freeze_starts
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let e = freeze_ends.get(i).copied().unwrap_or(probe.duration);
            json!({"start": s, "end": e, "duration": e - s})
        })
        .collect();

    // blur QC: entropy diff-mode normalized Y < threshold reads as soft/OOF.
    let blur_th = args.blur.unwrap_or(0.45).clamp(0.0, 1.0);
    let blur_frames = entropy_vals.iter().filter(|&&v| v < blur_th).count();
    let blur_mean = if entropy_vals.is_empty() {
        None
    } else {
        Some(entropy_vals.iter().sum::<f64>() / entropy_vals.len() as f64)
    };
    let blur_min = entropy_vals.iter().cloned().reduce(f64::min);

    // volumedetect pass: peak + mean dB (clip check + cheap loudness read)
    // volumedetect + replaygain in one audio pass: peak/mean dB plus the
    // ReplayGain tags (track_gain/track_peak print at EOF)
    let (mut audio_max_db, mut audio_mean_db): (Option<f64>, Option<f64>) = (None, None);
    let (mut rg_gain_db, mut rg_peak): (Option<f64>, Option<f64>) = (None, None);
    if probe.has_audio {
        let mut argv = Argv::ffmpeg();
        argv.push("-i");
        argv.push(&args.input);
        argv.extend(["-af", "volumedetect,replaygain", "-f", "null", "-"]);
        if let Ok(sp) = spawn::run(&argv, g.timeout, false) {
            let stderr = String::from_utf8_lossy(&sp.stderr);
            for line in stderr.lines() {
                if let Some(v) = line.split("max_volume:").nth(1) {
                    audio_max_db = v.trim().trim_end_matches(" dB").parse().ok();
                }
                if let Some(v) = line.split("mean_volume:").nth(1) {
                    audio_mean_db = v.trim().trim_end_matches(" dB").parse().ok();
                }
                if let Some(v) = line.split("track_gain = ").nth(1) {
                    rg_gain_db = v.trim().trim_end_matches(" dB").parse().ok();
                }
                if let Some(v) = line.split("track_peak = ").nth(1) {
                    rg_peak = v.trim().parse().ok();
                }
            }
        }
    }
    // --loud: EBU R128 summary — I/LRA/true-peak in one pass; the platform
    // loudness spec check (podcast −16, broadcast −23, social −14)
    let (mut loud_i, mut loud_lra, mut loud_tp): (Option<f64>, Option<f64>, Option<f64>) =
        (None, None, None);
    if args.loud {
        if !probe.has_audio {
            return Err(Error::input("scan --loud needs an audio stream"));
        }
        let mut argv = Argv::ffmpeg();
        argv.push("-i");
        argv.push(&args.input);
        argv.extend(["-af", "ebur128=peak=true", "-f", "null", "-"]);
        if let Ok(sp) = spawn::run(&argv, g.timeout, false) {
            for line in String::from_utf8_lossy(&sp.stderr).lines() {
                let t = line.trim();
                if let Some(v) = t.strip_prefix("I:") {
                    loud_i = v.trim().trim_end_matches(" LUFS").trim().parse().ok();
                }
                if let Some(v) = t.strip_prefix("LRA:") {
                    loud_lra = v.trim().trim_end_matches(" LU").trim().parse().ok();
                }
                if let Some(v) = t.strip_prefix("Peak:") {
                    // 4.x prints dBFS here, 5.x+ prints dBTP — same value
                    let v = v
                        .trim()
                        .trim_end_matches(" dBFS")
                        .trim_end_matches(" dBTP")
                        .trim();
                    loud_tp = v.parse().ok();
                }
            }
        }
    }
    // --gop: keyframe-interval QC straight from packet flags — no decode,
    // so it's cheap even on long masters. Ingest specs ("keyframe every ≤2s"
    // for live platforms) verified from the packet map; gaps measured in
    // stream order so the tail stretch after the last key counts too.
    let mut key_pts: Vec<f64> = Vec::new();
    let mut key_idx: Vec<usize> = Vec::new();
    let mut packet_count = 0usize;
    if args.gop {
        let mut argv = Argv::ffprobe();
        argv.extend([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "packet=flags,dts_time",
            "-of",
            "csv=p=0",
        ]);
        argv.push(&args.input);
        if let Ok(sp) = spawn::run(&argv, g.timeout, false) {
            for (i, line) in String::from_utf8_lossy(&sp.stdout).lines().enumerate() {
                packet_count += 1;
                let mut f = line.trim().split(',');
                let dts: f64 = f.next().and_then(|v| v.parse().ok()).unwrap_or(0.0);
                if f.next().is_some_and(|fl| fl.contains('K')) {
                    key_pts.push(dts);
                    key_idx.push(i);
                }
            }
        }
    }
    // --hash: decoded-frame checksums of every stream → <input>.framemd5
    // manifest — the archive-ingest integrity record. Decodes the whole
    // file, so it stays behind a flag.
    let mut hash_file = String::new();
    let mut hash_frames = 0usize;
    if args.hash {
        hash_file = format!("{}.framemd5", args.input.display());
        let mut argv = Argv::ffmpeg();
        argv.extend(["-v", "error", "-y", "-i"]);
        argv.push(&args.input);
        argv.extend(["-map", "0", "-f", "framemd5", &hash_file]);
        spawn::require_ok(&argv, spawn::run(&argv, g.timeout, false)?)?;
        hash_frames = std::fs::read_to_string(&hash_file)
            .map(|t| t.lines().filter(|l| !l.starts_with('#')).count())
            .unwrap_or(0);
    }
    // --bitrate: video-rate curve from packet sizes — no decode. 0.5s
    // windows bucket pts→bytes so VBR spikes (scene changes) show up as
    // peak Mbps; platform ingest specs cap the peak, not the average.
    let mut br_peak_mbps = 0.0f64;
    let mut br_spike_at = 0.0f64;
    let mut br_mean_mbps = 0.0f64;
    if args.bitrate {
        let mut argv = Argv::ffprobe();
        argv.extend([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "packet=pts_time,size",
            "-of",
            "csv=p=0",
        ]);
        argv.push(&args.input);
        if let Ok(sp) = spawn::run(&argv, g.timeout, false) {
            let mut bucket_pts = 0.0f64;
            let mut bucket_bytes = 0u64;
            let mut total_bytes = 0u64;
            let mut first_pts: Option<f64> = None;
            let mut last_pts = 0.0f64;
            fn flush(at: f64, bytes: u64, total: &mut u64, peak: &mut f64, spike: &mut f64) {
                *total += bytes;
                let mbps = bytes as f64 * 8.0 / 0.5 / 1e6;
                if mbps > *peak {
                    *peak = mbps;
                    *spike = at;
                }
            }
            for line in String::from_utf8_lossy(&sp.stdout).lines() {
                let mut f = line.trim().split(',');
                let Some(Ok(pts)) = f.next().map(|v| v.parse::<f64>()) else {
                    continue;
                };
                let Some(Ok(size)) = f.next().map(|v| v.parse::<u64>()) else {
                    continue;
                };
                if first_pts.is_none() {
                    first_pts = Some(pts);
                    bucket_pts = pts;
                }
                last_pts = pts;
                if pts - bucket_pts >= 0.5 {
                    flush(
                        bucket_pts,
                        bucket_bytes,
                        &mut total_bytes,
                        &mut br_peak_mbps,
                        &mut br_spike_at,
                    );
                    bucket_pts = pts;
                    bucket_bytes = 0;
                }
                bucket_bytes += size;
            }
            if bucket_bytes > 0 {
                flush(
                    bucket_pts,
                    bucket_bytes,
                    &mut total_bytes,
                    &mut br_peak_mbps,
                    &mut br_spike_at,
                );
            }
            let span = last_pts - first_pts.unwrap_or(last_pts);
            if span > 0.0 {
                br_mean_mbps = total_bytes as f64 * 8.0 / span / 1e6;
            }
        }
    }
    // --deadair DB: dead-air map for podcast/talking-head QC — reuses the
    // silence detector so one `scan` reports pauses alongside video faults
    let mut deadair_ranges: Vec<serde_json::Value> = Vec::new();
    let mut deadair_secs = 0.0f64;
    if let Some(db) = args.deadair {
        if !probe.has_audio {
            return Err(Error::input("scan --deadair needs an audio stream"));
        }
        for (s, e) in crate::silence::detect(&args.input, db, 1.0, g.timeout, true)? {
            deadair_secs += e - s;
            deadair_ranges.push(json!({"start": s, "end": e, "duration": e - s}));
        }
    }
    // --dupe REF: MPEG-7 signature match — is this clip inside REF (or a
    // re-upload of it)? nb_inputs=2 + detectmode=full logs "matching of
    // video 0 at T and 1 at T2, N frames matching" / "whole video matching"
    // / "no matching" — needs a few seconds of footage to build its words
    let mut dupe_segments = 0usize;
    let mut dupe_frames = 0usize;
    if let Some(dref) = &args.dupe {
        paths::ensure_input(dref)?;
        let mut argv = Argv::ffmpeg();
        argv.push("-i");
        argv.push(&args.input);
        argv.push("-i");
        argv.push(dref);
        argv.extend([
            "-filter_complex",
            "[0:v][1:v]signature=nb_inputs=2:detectmode=full",
            "-f",
            "null",
            "-",
        ]);
        if let Ok(sp) = spawn::run(&argv, g.timeout, false) {
            for line in String::from_utf8_lossy(&sp.stderr).lines() {
                if line.contains("matching of video") {
                    dupe_segments += 1;
                    if let Some(n) = line
                        .split(',')
                        .nth(1)
                        .and_then(|s| s.trim().split(' ').next())
                        .and_then(|s| s.parse::<usize>().ok())
                    {
                        dupe_frames += n;
                    }
                }
            }
        }
    }
    // --text: OCR burned-in text (tesseract, if this ffmpeg links it) at
    // 2fps — slow per frame, so a dedicated low-rate pass. Reports the
    // first hit, hit frame count and best word confidence
    let mut ocr_text: Option<String> = None;
    let mut ocr_frames = 0usize;
    let mut ocr_conf = 0.0f64;
    if args.text {
        let mut argv = Argv::ffmpeg();
        argv.push("-i");
        argv.push(&args.input);
        argv.extend([
            "-vf",
            "fps=2,ocr,metadata=mode=print:file=-",
            "-an",
            "-f",
            "null",
            "-",
        ]);
        if let Ok(sp) = spawn::run(&argv, g.timeout, false) {
            if sp.status_ok {
                let mut saw_text = false;
                for line in spawn::stdout_str(&sp).unwrap_or_default().lines() {
                    if let Some(rest) = line.split("lavfi.ocr.text=").nth(1) {
                        let t = rest.trim();
                        if !t.is_empty() {
                            saw_text = true;
                            if ocr_text.is_none() {
                                ocr_text = Some(t.to_string());
                            }
                        }
                    }
                    if saw_text && line.contains("lavfi.ocr.confidence=") {
                        ocr_frames += 1;
                        if let Some(rest) = line.split("lavfi.ocr.confidence=").nth(1) {
                            for tok in rest.trim().split(' ') {
                                if let Ok(v) = tok.parse::<f64>() {
                                    ocr_conf = ocr_conf.max(v);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    let crop_hint = if cd_x1 < 0 || cd_x2 <= cd_x1 || cd_y2 <= cd_y1 {
        None
    } else {
        Some(format!(
            "{}x{}:{}:{}",
            cd_x2 - cd_x1 + 1,
            cd_y2 - cd_y1 + 1,
            cd_x1,
            cd_y1
        ))
    };
    let letterboxed = cd_x1 >= 0
        && (cd_x1 > 0
            || cd_y1 > 0
            || cd_x2 + 1 < probe.width.unwrap_or(0) as i64
            || cd_y2 + 1 < probe.height.unwrap_or(0) as i64);

    // scdet reports a cut on both boundary frames — collapse near-dupes
    scene_cuts.dedup_by(|a, b| (*a - *b).abs() < 0.05);
    let mut extra = json!({
        "freeze_min": freeze_min,
        "black_min": black_min,
        "luma_threshold": thresh,
        "black_ranges": black_ranges,
        "freeze_ranges": freeze_ranges,
        "black_frames": black_frames,
        // photosensitive-epilepsy QC: frames where luminance oscillates enough
        // to flag (Harding-style heuristic; ship with a warning card if >0)
        "flash_frames": flash_frames,
        "scene_cuts": scene_cuts,
        "flash_max_badness": flash_max,
        // interlace verdict from idet single-frame detection
        "interlaced": idet_counts.0 + idet_counts.1 > idet_counts.2,
        "frames_tff": idet_counts.0,
        "frames_bff": idet_counts.1,
        "frames_progressive": idet_counts.2,
        "frames_undetermined": idet_counts.3,
        // L/R phase correlation, stereo inputs only: <0 collapses in mono
        "phase_corr": phase_corr,
        // peak level (>= -0.5 dB clips on most encoders) + programme mean
        // signalstats worst-of QC: broadcast-legal luma is 16-235 (8-bit);
        // a clip dipping under 16 or peaking over 235 fails legalize specs
        "luma_min": if luma_min == f64::MAX { None } else { Some(luma_min) },
        "luma_max": if luma_max == f64::MIN { None } else { Some(luma_max) },
        "illegal_luma": luma_min < 16.0 || luma_max > 235.0,
        "audio_max_db": audio_max_db,
        "audio_mean_db": audio_mean_db,
        // replaygain tags (music libraries): the gain a player should apply
        // to hit reference loudness — negative = this track is loud, needs
        // turning down; positive = quiet, needs a boost
        "rg_gain_db": rg_gain_db,
        "rg_peak": rg_peak,
        // duplicate detection (--dupe REF): matching segments the signature
        // pass found — 0 = not in REF, >0 = (partially) re-uploaded content
        "dupe": dupe_segments > 0,
        "dupe_segments": if args.dupe.is_some() { Some(dupe_segments) } else { None },
        "dupe_frames": if args.dupe.is_some() { Some(dupe_frames) } else { None },
        // OCR (--text): burned-in text found, frames containing it, and the
        // best word confidence 0-100
        "text": ocr_text,
        "text_frames": if args.text { Some(ocr_frames) } else { None },
        "text_confidence": if args.text { Some(ocr_conf) } else { None },
        // entropy blur QC: normalized luma-diff entropy per frame —
        // soft/out-of-focus stretches sink under blur_threshold
        "blur_threshold": blur_th,
        "blur_frames": blur_frames,
        "blur_mean": blur_mean,
        "blur_min": blur_min,
        // noise-floor QC: mean lowest-bit-plane occupancy across frames/planes
        // — >~0.8 means real sensor noise/grain (budget bitrate accordingly)
        "noise_floor": if noise_vals.is_empty() {
            None
        } else {
            Some((noise_vals.iter().sum::<f64>() / noise_vals.len() as f64 * 1000.0).round() / 1000.0)
        },
        "noisy": !noise_vals.is_empty()
            && noise_vals.iter().sum::<f64>() / noise_vals.len() as f64 > 0.8,
        // broadcast QC: EIA-608 closed-caption lines decoded (has_cc gates
        // delivery specs that require CC on air masters)
        "has_cc": cc_lines > 0,
        "cc_lines": cc_lines,
        // VFR QC: fraction of frames arriving at variable intervals — >0.05
        // means a real variable-rate source (screen captures ~0.5); a few
        // stray frames just flag the container boundary
        "vfr": vfr_ratio.map(|r| r > 0.05).unwrap_or(false),
        "vfr_ratio": vfr_ratio,
        "vfr_frames": vfr_frames,
        // letterbox QC: inner content bounds from cropdetect — a clip that
        // is letterboxed has a hint smaller than the frame (crop it, or
        // deliver --platform which re-pads cleanly)
        "crop_hint": crop_hint,
        "letterboxed": letterboxed,
    });
    // --motion: VMAF motion feature — bitrate-budget QC (static ≈0,
    // busy action ≈7+; a >5 mean wants real bitrate at delivery)
    extra["motion_avg"] = if args.motion && !motion_vals.is_empty() {
        json!(motion_vals.iter().sum::<f64>() / motion_vals.len() as f64)
    } else {
        json!(null)
    };
    extra["motion_max"] = if args.motion {
        motion_vals
            .iter()
            .cloned()
            .reduce(f64::max)
            .map_or(json!(null), |v| json!(v))
    } else {
        json!(null)
    };
    // --timecode: broadcast-master VITC readout — any frame with a
    // decoded code reports the latest timecode string + hit count
    extra["vitc"] = json!(vitc_frames > 0);
    extra["vitc_tc"] = json!(vitc_tc);
    extra["vitc_frames"] = if args.timecode {
        json!(vitc_frames)
    } else {
        json!(null)
    };
    // --bbox: union of every frame's content box — where in the frame
    // the actual subject lives (fill <1 means borders worth cropping;
    // unlike cropdetect it is not limited to black backgrounds)
    let detected = bb_x2 >= bb_x1 && bb_y2 >= bb_y1;
    extra["content_detected"] = json!(detected);
    extra["content_box"] = if detected {
        json!(format!(
            "{},{},{},{}",
            bb_x1,
            bb_y1,
            bb_x2 - bb_x1 + 1,
            bb_y2 - bb_y1 + 1
        ))
    } else {
        json!(null)
    };
    extra["content_fill"] = if detected {
        let fw = probe.width.unwrap_or(0).max(1) as f64;
        let fh = probe.height.unwrap_or(0).max(1) as f64;
        json!(
            ((bb_x2 - bb_x1 + 1) as f64 * (bb_y2 - bb_y1 + 1) as f64 / (fw * fh) * 1000.0).round()
                / 1000.0
        )
    } else {
        json!(null)
    };
    // signalstats tone QC (same pass, zero extra decodes): sat_mean reads
    // washed-out/oversaturated footage, hue_mean (deg) flags a color cast,
    // y_mean is programme brightness on the broadcast 16-235 scale
    extra["sat_mean"] = if sat_vals.is_empty() {
        json!(null)
    } else {
        json!((sat_vals.iter().sum::<f64>() / sat_vals.len() as f64 * 100.0).round() / 100.0)
    };
    extra["hue_mean"] = if hue_vals.is_empty() {
        json!(null)
    } else {
        json!((hue_vals.iter().sum::<f64>() / hue_vals.len() as f64 * 100.0).round() / 100.0)
    };
    extra["y_mean"] = if y_vals.is_empty() {
        json!(null)
    } else {
        json!((y_vals.iter().sum::<f64>() / y_vals.len() as f64 * 100.0).round() / 100.0)
    };
    // --deadair: silent stretches ≥1s at/below the threshold — publish-gate
    // for podcasts and talking-head cuts (seconds + per-range map)
    extra["deadair_secs"] = if args.deadair.is_some() {
        json!((deadair_secs * 100.0).round() / 100.0)
    } else {
        json!(null)
    };
    extra["deadair_ranges"] = if args.deadair.is_some() {
        json!(deadair_ranges)
    } else {
        json!(null)
    };
    // --loud: integrated loudness (LUFS), loudness range (LU), true peak
    // (dBTP) — publish gates: podcast ≈−16 I, broadcast ≈−23 I
    extra["loud_i"] = if args.loud {
        json!(loud_i)
    } else {
        json!(null)
    };
    extra["loud_lra"] = if args.loud {
        json!(loud_lra)
    } else {
        json!(null)
    };
    extra["loud_tp"] = if args.loud {
        json!(loud_tp)
    } else {
        json!(null)
    };
    if args.gop {
        let frames_gap = key_idx
            .windows(2)
            .map(|w| w[1] - w[0])
            .chain(key_idx.last().map(|l| packet_count - *l))
            .max()
            .unwrap_or(0);
        let secs: Vec<f64> = key_pts.windows(2).map(|w| w[1] - w[0]).collect();
        extra["keyframes"] = json!(key_idx.len());
        // every keyframe's pts — check them against chapter marks / ingest
        // specs that pin a keyframe at each seek point
        extra["keyframe_times"] = json!(key_pts);
        extra["gop_max_sec"] = json!(secs.iter().cloned().fold(0.0f64, f64::max));
        extra["gop_avg_sec"] = json!(if secs.is_empty() {
            0.0
        } else {
            secs.iter().sum::<f64>() / secs.len() as f64
        });
        extra["gop_max_frames"] = json!(frames_gap);
    }
    // HDR / wide-gamut QC straight from the container's colour metadata —
    // zero extra decode. HDR = PQ (smpte2084) or HLG transfer; wide gamut =
    // BT.2020 primaries. Untagged masters report the raw "unknown" strings.
    let trc = probe
        .color_transfer
        .clone()
        .unwrap_or_else(|| "unknown".into());
    let prm = probe
        .color_primaries
        .clone()
        .unwrap_or_else(|| "unknown".into());
    extra["hdr"] = json!(matches!(
        trc.as_str(),
        "smpte2084" | "arib-std-b67" | "bt2020-10" | "bt2020-12"
    ));
    extra["wide_gamut"] = json!(prm.contains("2020"));
    extra["color_space"] = json!(probe.color_space.clone());
    extra["color_primaries"] = json!(prm);
    extra["color_transfer"] = json!(trc);
    if args.hash {
        extra["hash_file"] = json!(hash_file);
        extra["hash_frames"] = json!(hash_frames);
    }
    if args.bitrate {
        extra["bitrate_mean_mbps"] = json!((br_mean_mbps * 100.0).round() / 100.0);
        extra["bitrate_peak_mbps"] = json!((br_peak_mbps * 100.0).round() / 100.0);
        extra["bitrate_spike_at"] = json!(br_spike_at);
    }
    if let Some(ms) = probe.av_desync_ms {
        extra["av_desync_ms"] = json!(ms);
    }
    // Container timecode QC (mov tmcd stream tag / mkv TIMECODE format tag):
    // masters expected to carry a slate-matching TC, zero decode.
    if let Some(tc) = &probe.timecode {
        extra["timecode"] = json!(tc);
    }
    Ok(Contract::ok("scan", None, Some(probe)).with_extra(extra))
}

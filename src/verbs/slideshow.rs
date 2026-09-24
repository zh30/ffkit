use std::path::{Path, PathBuf};

use serde_json::json;

use crate::cli::{Globals, SlideMotion, SlideshowArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::paths;

/// Still images → video montage: each still scaled/padded onto a common
/// canvas, crossfaded (or hard-cut) to the next, with an optional music bed
/// that fades out at the end. Audio track is always present (silent bed when
/// none is given) so players and socials never see a soundless file.
pub fn run(args: SlideshowArgs, g: &Globals) -> Result<Contract, Error> {
    let mut args = args;
    if let Some(list) = &args.list {
        if !args.inputs.is_empty() {
            return Err(Error::input(
                "slideshow --list takes stills from the file — drop the positional inputs",
            ));
        }
        paths::ensure_input(list)?;
        let text = std::fs::read_to_string(list)
            .map_err(|e| Error::input(format!("can't read {}: {e}", list.display())))?;
        let dir = list.parent().unwrap_or_else(|| Path::new(""));
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let p = PathBuf::from(line);
            args.inputs
                .push(if p.is_absolute() { p } else { dir.join(p) });
        }
        if args.inputs.is_empty() {
            return Err(Error::input(format!(
                "{} has no usable image paths",
                list.display()
            )));
        }
    }
    if args.inputs.len() < 2 {
        return Err(Error::input("slideshow needs at least two images"));
    }
    if args.dur.is_none() && !(0.5..=60.0).contains(&args.per) {
        return Err(Error::input("--per must be 0.5..=60 seconds"));
    }
    if !(0.0..=10.0).contains(&args.fade) {
        return Err(Error::input("--fade must be 0..=10 seconds"));
    }
    if args.fit && args.audio.is_none() {
        return Err(Error::input(
            "slideshow --fit needs --audio — the bed sets the montage length",
        ));
    }
    if args.fit && args.dur.is_some() {
        return Err(Error::input(
            "--fit and --dur are exclusive — --fit derives the length from the audio",
        ));
    }
    // Probe the music bed up front: --fit borrows its duration for the
    // per-still solve below.
    if args.audio_loop {
        if args.audio.is_none() {
            return Err(Error::input("--audio-loop needs --audio"));
        }
        if args.fit {
            return Err(Error::input(
                "--audio-loop makes the bed endless — --fit already ends on the bed",
            ));
        }
    }
    if let Some(off) = args.audio_offset {
        if args.audio.is_none() {
            return Err(Error::input("--audio-offset needs --audio"));
        }
        if !off.is_finite() || off < 0.0 {
            return Err(Error::input("--audio-offset must be >= 0 seconds"));
        }
    }
    let bed_probe = if let Some(bed) = &args.audio {
        paths::ensure_input(bed)?;
        let p = engine::probe_or_err(bed, g)?;
        if !p.has_audio {
            return Err(Error::input("slideshow: --audio file has no audio stream"));
        }
        if let Some(off) = args.audio_offset {
            if off >= p.duration {
                return Err(Error::input(format!(
                    "--audio-offset {off}s lands past the bed's end ({:.1}s)",
                    p.duration
                )));
            }
        }
        Some(p)
    } else {
        None
    };
    // --sort: deterministic ordering before the optional reroll — mtime
    // puts a camera-dump folder into shoot order
    let mut ordered: Vec<PathBuf> = args.inputs.clone();
    match args.sort {
        Some(crate::cli::SlideSort::Name) => ordered.sort(),
        Some(crate::cli::SlideSort::Mtime) => {
            ordered.sort_by_key(|p| {
                std::fs::metadata(p)
                    .and_then(|m| m.modified())
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
            });
        }
        None => {}
    }
    // --shuffle: deterministic reroll — same seed, same order.
    if let Some(seed) = args.shuffle {
        let mut state = seed | 1;
        for i in (1..ordered.len()).rev() {
            // xorshift64*
            state ^= state >> 12;
            state ^= state << 25;
            state ^= state >> 27;
            let r = (state.wrapping_mul(0x2545F4914F6CDD1D) >> 33) as usize;
            ordered.swap(i, r % (i + 1));
        }
    }
    let n = ordered.len();
    // --dur/--fit: solve for the per-still length that lands the montage on
    // the target runtime (total = n*per - (n-1)*fade). --fit takes the
    // target from the audio bed's length.
    let dur_target = args.dur.or(if args.fit {
        bed_probe
            .as_ref()
            .map(|p| p.duration - args.audio_offset.unwrap_or(0.0))
    } else {
        None
    });
    let per = match dur_target {
        Some(d) => {
            if !(1.0..=600.0).contains(&d) {
                return Err(Error::input("--dur must be 1..=600 seconds"));
            }
            let p = (d + (n as f64 - 1.0) * args.fade) / n as f64;
            if p <= args.fade.max(0.2) {
                return Err(Error::input(format!(
                    "--dur {d}s is too short for {n} images at --fade {}",
                    args.fade
                )));
            }
            p
        }
        None => args.per,
    };
    if args.fade >= per {
        return Err(Error::input("--fade must be shorter than --per"));
    }
    if !(1.0..=120.0).contains(&args.fps) {
        return Err(Error::input("--fps must be 1..=120"));
    }
    let (w, h) = parse_size(&args.size)?;
    let bg = args
        .bg
        .as_deref()
        .map(crate::color::lavfi)
        .unwrap_or_else(|| "black".to_string());
    for p in &ordered {
        paths::ensure_input(p)?;
    }
    if args.volume.is_some() && args.audio.is_none() {
        return Err(Error::input("--volume needs --audio"));
    }
    let bed_vol = match args.volume {
        Some(v) if (0.0..=4.0).contains(&v) => v,
        Some(_) => return Err(Error::input("--volume must be 0..=4")),
        None => 1.0,
    };
    let bed_tail = args.audio_fade.unwrap_or(0.8);
    if let Some(af) = args.audio_fade {
        if args.audio.is_none() {
            return Err(Error::input("--audio-fade needs --audio"));
        }
        if !af.is_finite() || af <= 0.0 {
            return Err(Error::input("--audio-fade must be > 0 seconds"));
        }
    }
    if let Some(af) = args.audio_fade_in {
        if args.audio.is_none() {
            return Err(Error::input("--audio-fade-in needs --audio"));
        }
        if !af.is_finite() || af <= 0.0 {
            return Err(Error::input("--audio-fade-in must be > 0 seconds"));
        }
    }
    let total = n as f64 * per - (n as f64 - 1.0) * args.fade;
    if bed_tail >= total {
        return Err(Error::input(
            "--audio-fade must be shorter than the montage",
        ));
    }
    if let Some(af) = args.audio_fade_in {
        if af >= total - bed_tail {
            return Err(Error::input(
                "--audio-fade-in must leave room before the tail fade",
            ));
        }
    }
    let fps = args.fps;

    // --titles: bottom caption strip on each still, slots in the final
    // slide order — an empty comma entry leaves that slide uncaptioned.
    let titles: Vec<Option<String>> = match &args.titles {
        Some(raw) => {
            let v: Vec<String> = raw.split(',').map(|s| s.trim().to_string()).collect();
            if v.len() > n {
                return Err(Error::input(format!(
                    "--titles has {} entries for {n} stills — one comma slot per slide",
                    v.len()
                )));
            }
            (0..n)
                .map(|i| v.get(i).filter(|t| !t.is_empty()).cloned())
                .collect()
        }
        None => vec![None; n],
    };
    let cap_count = titles.iter().flatten().count();
    let font_bytes = if cap_count > 0 {
        let fp = crate::font::resolve(None)?;
        Some(std::fs::read(&fp)?)
    } else {
        None
    };
    let tmp = tempfile::tempdir().map_err(|e| Error::output(e.to_string()))?;

    let mut argv = ffmpeg_base(g.progress);
    for p in &ordered {
        if args.motion == SlideMotion::Kenburns {
            // One input frame each; zoompan expands it into `per` seconds.
            argv.extend(["-i"]);
            argv.push(p);
        } else {
            argv.extend(["-loop", "1", "-t"]);
            argv.push(format!("{:.3}", per));
            argv.extend(["-i"]);
            argv.push(p);
        }
    }
    // Caption PNGs ride as extra inputs after the stills (a single-frame
    // overlay input repeats on eof, which is exactly a per-slide sticker).
    let mut cap_input: Vec<Option<usize>> = vec![None; n];
    let mut next_input = n;
    for (i, text) in titles.iter().enumerate() {
        if let Some(text) = text {
            let bytes = font_bytes.as_deref().unwrap_or(&[]);
            let ow = ((w as f32 / 8.0 * 0.5) * 0.06).round().clamp(2.0, 24.0) as u32;
            let img = crate::raster::render_title_outlined(
                text,
                bytes,
                w,
                [255, 255, 255],
                0.5,
                ([0, 0, 0], ow),
            )?;
            let png = tmp.path().join(format!("cap{i}.png"));
            img.save(&png)
                .map_err(|e| Error::output(format!("write caption png: {e}")))?;
            argv.extend(["-i"]);
            argv.push(&png);
            cap_input[i] = Some(next_input);
            next_input += 1;
        }
    }
    let bed_idx = if let Some(bed) = &args.audio {
        if args.audio_loop {
            argv.extend(["-stream_loop", "-1"]);
        }
        if let Some(off) = args.audio_offset {
            argv.extend(["-ss", format!("{off}").as_str()]);
        }
        argv.extend(["-i"]);
        argv.push(bed);
        Some(next_input)
    } else {
        argv.extend(["-f", "lavfi", "-i", "anullsrc=r=48000:cl=stereo"]);
        None
    };

    // Normalize every still to the canvas first.
    let mut fc = String::new();
    let zoom_frames = (per * fps).round() as u32;
    for (i, cap) in cap_input.iter().enumerate() {
        // Titled slides normalize to n{i} first so the caption overlays
        // onto the finished canvas (bar-letterboxed or zooming alike).
        let pin = if cap.is_some() {
            format!("n{i}")
        } else {
            format!("s{i}")
        };
        if args.motion == SlideMotion::Kenburns {
            // Fill the canvas so the zoom window never catches bars, then
            // drift: even stills push in, odd stills pull out.
            let z = if i % 2 == 0 {
                "min(max(zoom,pzoom)+0.001,1.25)"
            } else {
                "max(1.25-0.001*on,1.0)"
            };
            fc.push_str(&format!(
                "[{i}:v]scale={w}:{h}:force_original_aspect_ratio=increase,\
                 crop={w}:{h},setsar=1,\
                 zoompan=z='{z}':x='iw/2-(iw/zoom/2)':y='ih/2-(ih/zoom/2)':d={zoom_frames}:s={w}x{h}:fps={fps:.3},\
                 format=yuv420p[{pin}];"
            ));
        } else {
            fc.push_str(&format!(
                "[{i}:v]scale={w}:{h}:force_original_aspect_ratio=decrease,\
                 pad={w}:{h}:(ow-iw)/2:(oh-ih)/2:{bg},setsar=1,fps={fps:.3},format=yuv420p[{pin}];"
            ));
        }
        if let Some(ci) = cap {
            fc.push_str(&format!("[{pin}][{ci}:v]overlay=(W-w)/2:H*0.94-h[s{i}];"));
        }
    }
    if args.fade > 0.0 {
        // xfade chain; transition k starts at k*(per - fade).
        let t_name = args.transition.xfade_name();
        let mut prev = "s0".to_string();
        for i in 1..n {
            let out = if i == n - 1 {
                "vout".to_string()
            } else {
                format!("x{i}")
            };
            let off = i as f64 * (per - args.fade);
            fc.push_str(&format!(
                "[{prev}][s{i}]xfade=transition={t_name}:duration={:.3}:offset={off:.3}[{out}];",
                args.fade
            ));
            prev = out;
        }
    } else {
        let seq: String = (0..n).map(|i| format!("[s{i}]")).collect();
        fc.push_str(&format!("{seq}concat=n={n}:v=1:a=0[vout];"));
    }

    // Caption PNGs shifted the input numbering — the silent bed (when
    // no --audio is given) is the last input, at index next_input.
    let audio_in = bed_idx.unwrap_or(next_input);
    let fade_in = match args.audio_fade_in {
        Some(d) => format!(",afade=t=in:st=0:d={d:.3}"),
        None => String::new(),
    };
    if bed_idx.is_some() {
        // Bed: pad/trim to the montage length, fade the tail (--audio-fade).
        fc.push_str(&format!(
            "[{audio_in}:a]apad,atrim=duration={total:.3},volume={bed_vol:.3},afade=t=out:st={:.3}:d={bed_tail:.3}{fade_in},aresample=48000[aout]",
            total - bed_tail
        ));
    } else {
        fc.push_str(&format!(
            "[{audio_in}:a]atrim=duration={total:.3},volume={bed_vol:.3},aresample=48000[aout]"
        ));
    }

    argv.extend([
        "-filter_complex",
        &fc,
        "-map",
        "[vout]",
        "-map",
        "[aout]",
        "-c:v",
        "libx264",
        "-preset",
        "fast",
        "-crf",
        "20",
        "-pix_fmt",
        "yuv420p",
        "-c:a",
        "aac",
        "-movflags",
        "+faststart",
    ]);
    argv.push(&args.output);

    let mut inputs: Vec<&Path> = ordered.iter().map(|p| p.as_path()).collect();
    if let Some(bed) = &args.audio {
        inputs.push(bed);
    }
    let mut c = engine::write_job("slideshow", &inputs, &args.output, vec![argv], g)?;
    c = c.with_extra(json!({
        "frames": n,
        "per": per,
        "fade": args.fade,
        "transition": args.transition.xfade_name(),
        "motion": match args.motion {
            SlideMotion::None => "none",
            SlideMotion::Kenburns => "kenburns",
        },
        "canvas": format!("{w}x{h}"),
        "music_bed": args.audio.is_some(),
        "audio_loop": args.audio_loop,
        "audio_fade": bed_tail,
        "audio_fade_in": args.audio_fade_in,
        "fit": args.fit,
        "seed": args.shuffle,
        "sort": args.sort.map(|s| match s {
            crate::cli::SlideSort::Name => "name",
            crate::cli::SlideSort::Mtime => "mtime",
        }),
        "order": ordered.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
        "titles": titles
            .iter()
            .map(|t| t.as_deref().unwrap_or(""))
            .collect::<Vec<_>>(),
        "expected_duration": total,
    }));
    Ok(c)
}

fn parse_size(s: &str) -> Result<(u32, u32), Error> {
    let (w, h) = s
        .split_once('x')
        .and_then(|(a, b)| a.parse::<u32>().ok().zip(b.parse::<u32>().ok()))
        .ok_or_else(|| Error::input("--size must look like 1920x1080"))?;
    if w < 16 || h < 16 || w > 8192 || h > 8192 || w % 2 == 1 || h % 2 == 1 {
        return Err(Error::input("--size must be even WxH, 16..8192"));
    }
    Ok((w, h))
}

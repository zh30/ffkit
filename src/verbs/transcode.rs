use serde_json::json;

use crate::cli::{Globals, TranscodeArgs, TranscodePreset};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::spawn::Argv;
pub fn run(args: TranscodeArgs, g: &Globals) -> Result<Contract, Error> {
    if args.gop.is_some() && args.copy_video {
        return Err(Error::input(
            "transcode --gop needs a re-encode — drop --copy-video",
        ));
    }
    if args.gop.is_some()
        && matches!(
            args.preset,
            Some(TranscodePreset::Mp3)
                | Some(TranscodePreset::Aac)
                | Some(TranscodePreset::Wav)
                | Some(TranscodePreset::Flac)
                | Some(TranscodePreset::Opus)
                | Some(TranscodePreset::Ogg)
                | Some(
                    TranscodePreset::Alac
                        | TranscodePreset::Ac3
                        | TranscodePreset::Eac3
                        | TranscodePreset::Tta
                        | TranscodePreset::Dca
                        | TranscodePreset::Aiff
                        | TranscodePreset::Pcm24
                        | TranscodePreset::Pcm32f
                        | TranscodePreset::Mulaw
                        | TranscodePreset::Adx
                        | TranscodePreset::Adpcm
                        | TranscodePreset::Alaw
                        | TranscodePreset::Speex
                        | TranscodePreset::Pcm8
                        | TranscodePreset::Adpcmms
                        | TranscodePreset::G722
                        | TranscodePreset::Ra144
                        | TranscodePreset::Nelly
                        | TranscodePreset::Wv
                        | TranscodePreset::Mp2
                        | TranscodePreset::Caf
                        | TranscodePreset::W64
                        | TranscodePreset::Voc
                        | TranscodePreset::Aptx
                        | TranscodePreset::Sbc
                        | TranscodePreset::G723
                        | TranscodePreset::Truehd
                        | TranscodePreset::Mlp
                )
        )
    {
        return Err(Error::input(
            "transcode --gop is a video encode flag — audio presets have no keyframes",
        ));
    }
    if (args.ar.is_some() || args.channels.is_some())
        && matches!(
            args.preset,
            Some(TranscodePreset::Mulaw)
                | Some(TranscodePreset::Alaw)
                | Some(TranscodePreset::G722)
                | Some(TranscodePreset::Ra144)
                | Some(TranscodePreset::G723)
        )
    {
        return Err(Error::input(
            "G.711/G.722/G.723/RealAudio are pinned-rate telephony specs — drop --ar/--channels",
        ));
    }
    let ext = args
        .output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let preset = args.preset.unwrap_or(match ext.as_str() {
        "gif" => TranscodePreset::Gif,
        "webm" => TranscodePreset::Webm,
        _ => TranscodePreset::H264,
    });

    let x264spec =
        args.profile.is_some() || args.level.is_some() || args.bf.is_some() || args.tune.is_some();
    if x264spec && args.copy_video {
        return Err(Error::input(
            "transcode --profile/--level/--bf/--tune need a re-encode — drop --copy-video",
        ));
    }
    if x264spec && !matches!(preset, TranscodePreset::H264 | TranscodePreset::Proxy) {
        return Err(Error::input(
            "transcode --profile/--level/--bf/--tune are x264 encode flags — h264/proxy presets only",
        ));
    }
    if args.alpha
        && !matches!(
            preset,
            TranscodePreset::Webm
                | TranscodePreset::Prores
                | TranscodePreset::Qtrle
                | TranscodePreset::Hap
        )
    {
        return Err(Error::input(
            "--alpha needs a webm, prores, qtrle or hap output (h264/hevc/av1 can't carry alpha)",
        ));
    }

    if args.range.is_some()
        && matches!(
            preset,
            TranscodePreset::Gif
                | TranscodePreset::Mp3
                | TranscodePreset::Aac
                | TranscodePreset::Wav
                | TranscodePreset::Flac
                | TranscodePreset::Opus
                | TranscodePreset::Ogg
                | TranscodePreset::Alac
                | TranscodePreset::Ac3
                | TranscodePreset::Eac3
                | TranscodePreset::Tta
                | TranscodePreset::Dca
                | TranscodePreset::Aiff
                | TranscodePreset::Pcm24
                | TranscodePreset::Pcm32f
                | TranscodePreset::Mulaw
                | TranscodePreset::Adx
                | TranscodePreset::Adpcm
                | TranscodePreset::Alaw
                | TranscodePreset::Speex
                | TranscodePreset::Pcm8
                | TranscodePreset::Adpcmms
                | TranscodePreset::G722
                | TranscodePreset::Ra144
                | TranscodePreset::Nelly
                | TranscodePreset::Wv
                | TranscodePreset::Mp2
                | TranscodePreset::Caf
                | TranscodePreset::W64
                | TranscodePreset::Voc
                | TranscodePreset::Aptx
                | TranscodePreset::Sbc
                | TranscodePreset::G723
                | TranscodePreset::Truehd
                | TranscodePreset::Mlp
        )
    {
        return Err(Error::input("--range applies to video presets only"));
    }
    if args.field_order.is_some() && args.interlaced {
        return Err(Error::input(
            "--field-order relabels parity — --interlaced weaves real fields; pick one",
        ));
    }
    if args.field_order.is_some()
        && matches!(
            preset,
            TranscodePreset::Gif
                | TranscodePreset::Mp3
                | TranscodePreset::Aac
                | TranscodePreset::Wav
                | TranscodePreset::Flac
                | TranscodePreset::Opus
                | TranscodePreset::Ogg
                | TranscodePreset::Alac
                | TranscodePreset::Ac3
                | TranscodePreset::Eac3
                | TranscodePreset::Tta
                | TranscodePreset::Dca
                | TranscodePreset::Aiff
                | TranscodePreset::Pcm24
                | TranscodePreset::Pcm32f
                | TranscodePreset::Mulaw
                | TranscodePreset::Adx
                | TranscodePreset::Adpcm
                | TranscodePreset::Alaw
                | TranscodePreset::Speex
                | TranscodePreset::Pcm8
                | TranscodePreset::Adpcmms
                | TranscodePreset::G722
                | TranscodePreset::Ra144
                | TranscodePreset::Nelly
                | TranscodePreset::Wv
                | TranscodePreset::Mp2
                | TranscodePreset::Caf
                | TranscodePreset::W64
                | TranscodePreset::Voc
                | TranscodePreset::Aptx
                | TranscodePreset::Sbc
                | TranscodePreset::G723
                | TranscodePreset::Truehd
                | TranscodePreset::Mlp
        )
    {
        return Err(Error::input("--field-order applies to video presets only"));
    }
    if args.vbitrate.is_some()
        && matches!(
            preset,
            TranscodePreset::Mp3
                | TranscodePreset::Aac
                | TranscodePreset::Wav
                | TranscodePreset::Flac
                | TranscodePreset::Opus
                | TranscodePreset::Ogg
                | TranscodePreset::Alac
                | TranscodePreset::Ac3
                | TranscodePreset::Eac3
                | TranscodePreset::Tta
                | TranscodePreset::Dca
                | TranscodePreset::Aiff
                | TranscodePreset::Pcm24
                | TranscodePreset::Pcm32f
                | TranscodePreset::Mulaw
                | TranscodePreset::Adx
                | TranscodePreset::Adpcm
                | TranscodePreset::Alaw
                | TranscodePreset::Speex
                | TranscodePreset::Pcm8
                | TranscodePreset::Adpcmms
                | TranscodePreset::G722
                | TranscodePreset::Ra144
                | TranscodePreset::Nelly
                | TranscodePreset::Wv
                | TranscodePreset::Mp2
                | TranscodePreset::Caf
                | TranscodePreset::W64
                | TranscodePreset::Voc
                | TranscodePreset::Aptx
                | TranscodePreset::Sbc
                | TranscodePreset::G723
                | TranscodePreset::Truehd
                | TranscodePreset::Mlp
                | TranscodePreset::Gif
                | TranscodePreset::Prores
                | TranscodePreset::Dnxhd
        )
    {
        return Err(Error::input(
            "--vbitrate applies to video presets (h264/hevc/webm/av1)",
        ));
    }
    if args.copy_video {
        if matches!(preset, TranscodePreset::Gif)
            || matches!(
                preset,
                TranscodePreset::Mp3
                    | TranscodePreset::Aac
                    | TranscodePreset::Wav
                    | TranscodePreset::Flac
                    | TranscodePreset::Opus
                    | TranscodePreset::Ogg
                    | TranscodePreset::Alac
                    | TranscodePreset::Ac3
                    | TranscodePreset::Eac3
                    | TranscodePreset::Tta
                    | TranscodePreset::Dca
                    | TranscodePreset::Aiff
                    | TranscodePreset::Pcm24
                    | TranscodePreset::Pcm32f
                    | TranscodePreset::Mulaw
                    | TranscodePreset::Adx
                    | TranscodePreset::Adpcm
                    | TranscodePreset::Alaw
                    | TranscodePreset::Speex
                    | TranscodePreset::Pcm8
                    | TranscodePreset::Adpcmms
                    | TranscodePreset::G722
                    | TranscodePreset::Ra144
                    | TranscodePreset::Nelly
                    | TranscodePreset::Wv
                    | TranscodePreset::Mp2
                    | TranscodePreset::Caf
                    | TranscodePreset::W64
                    | TranscodePreset::Voc
                    | TranscodePreset::Aptx
                    | TranscodePreset::Sbc
                    | TranscodePreset::G723
                    | TranscodePreset::Truehd
                    | TranscodePreset::Mlp
            )
        {
            return Err(Error::input(
                "--copy-video applies to video presets (h264/hevc/webm/av1/prores/dnxhd/proxy)",
            ));
        }
        if args.fps.is_some()
            || args.range.is_some()
            || args.interlaced
            || args.field_order.is_some()
            || args.alpha
            || args.vbitrate.is_some()
        {
            return Err(Error::input(
                "--copy-video stream-copies the picture — drop --fps/--range/--interlaced/--field-order/--alpha/--vbitrate",
            ));
        }
    }

    match preset {
        TranscodePreset::Mp3
        | TranscodePreset::Aac
        | TranscodePreset::Wav
        | TranscodePreset::Flac
        | TranscodePreset::Opus
        | TranscodePreset::Ogg
        | TranscodePreset::Alac
        | TranscodePreset::Ac3
        | TranscodePreset::Eac3
        | TranscodePreset::Tta
        | TranscodePreset::Dca
        | TranscodePreset::Aiff
        | TranscodePreset::Pcm24
        | TranscodePreset::Pcm32f
        | TranscodePreset::Mulaw
        | TranscodePreset::Adx
        | TranscodePreset::Adpcm
        | TranscodePreset::Alaw
        | TranscodePreset::Speex
        | TranscodePreset::Pcm8
        | TranscodePreset::Adpcmms
        | TranscodePreset::G722
        | TranscodePreset::Ra144
        | TranscodePreset::Nelly
        | TranscodePreset::Wv
        | TranscodePreset::Mp2
        | TranscodePreset::Caf
        | TranscodePreset::W64
        | TranscodePreset::Voc
        | TranscodePreset::Aptx
        | TranscodePreset::Sbc
        | TranscodePreset::G723
        | TranscodePreset::Truehd
        | TranscodePreset::Mlp => audio_only(&args, g, preset),
        TranscodePreset::Gif => gif(&args, g),
        TranscodePreset::H264 => h264(&args, g),
        TranscodePreset::Hevc => hevc(&args, g),
        TranscodePreset::Webm => webm(&args, g),
        TranscodePreset::Prores => prores(&args, g),
        TranscodePreset::Dnxhd => dnxhd(&args, g),
        TranscodePreset::Av1 => av1(&args, g),
        TranscodePreset::Proxy => proxy(&args, g),
        TranscodePreset::Ffv1 => ffv1(&args, g),
        TranscodePreset::Apng => apng(&args, g),
        TranscodePreset::Mpeg2 => mpeg2(&args, g),
        TranscodePreset::Mpeg1 => mpeg1(&args, g),
        TranscodePreset::Xvid => xvid(&args, g),
        TranscodePreset::Wmv => wmv(&args, g),
        TranscodePreset::Msmpeg4 => msmpeg4(&args, g),
        TranscodePreset::Gpp => gpp(&args, g),
        TranscodePreset::Flv => flv(&args, g),
        TranscodePreset::Theora => theora(&args, g),
        TranscodePreset::Dv => dv(&args, g),
        TranscodePreset::Mjpeg => mjpeg(&args, g),
        TranscodePreset::Amv => amv(&args, g),
        TranscodePreset::Qtrle => qtrle(&args, g),
        TranscodePreset::V210 => v210(&args, g),
        TranscodePreset::Huffyuv => lossless(&args, g, "huffyuv", &["avi", "mkv"], None),
        TranscodePreset::Utvideo => lossless(&args, g, "utvideo", &["avi"], None),
        TranscodePreset::Ffvhuff => lossless(&args, g, "ffvhuff", &["mkv"], None),
        TranscodePreset::V410 => lossless(&args, g, "v410", &["mov"], Some("yuv444p10le")),
        TranscodePreset::Ayuv => lossless(&args, g, "ayuv", &["mov"], Some("yuva444p")),
        TranscodePreset::Cinepak => qt_era(
            &args,
            g,
            "cinepak",
            &["mov", "avi"],
            None,
            Some("pcm_s16le"),
        ),
        TranscodePreset::Svq1 => qt_era(&args, g, "svq1", &["mov"], None, Some("pcm_s16le")),
        TranscodePreset::Zmbv => {
            qt_era(&args, g, "zmbv", &["avi"], Some("rgb24"), Some("pcm_s16le"))
        }
        TranscodePreset::Rv10 => qt_era(&args, g, "rv10", &["rm"], None, Some("real_144")),
        TranscodePreset::Rv20 => qt_era(&args, g, "rv20", &["rm"], None, Some("real_144")),
        TranscodePreset::R210 => lossless(&args, g, "r210", &["mov"], Some("gbrp10le")),
        TranscodePreset::V308 => lossless(&args, g, "v308", &["mov"], Some("yuv444p")),
        TranscodePreset::Rpza => qt_era(
            &args,
            g,
            "rpza",
            &["mov"],
            Some("rgb555le"),
            Some("pcm_s16le"),
        ),
        TranscodePreset::Speedhq => qt_era(
            &args,
            g,
            "speedhq",
            &["mov", "avi"],
            Some("yuv422p"),
            Some("pcm_s16le"),
        ),
        TranscodePreset::Roq => roq(&args, g),
        TranscodePreset::Snow => qt_era(
            &args,
            g,
            "snow",
            &["mkv"],
            Some("yuv420p"),
            Some("pcm_s16le"),
        ),
        TranscodePreset::Flashsv => qt_era(
            &args,
            g,
            "flashsv",
            &["flv"],
            Some("bgr24"),
            Some("libmp3lame"),
        ),
        TranscodePreset::Flashsv2 => qt_era(
            &args,
            g,
            "flashsv2",
            &["flv"],
            Some("bgr24"),
            Some("libmp3lame"),
        ),
        TranscodePreset::Msvideo1 => qt_era(
            &args,
            g,
            "msvideo1",
            &["avi"],
            Some("rgb555le"),
            Some("libmp3lame"),
        ),
        TranscodePreset::Cljr => qt_era(
            &args,
            g,
            "cljr",
            &["mov"],
            Some("yuv411p"),
            Some("pcm_s16le"),
        ),
        TranscodePreset::Hap => hap(&args, g, "hap"),
        TranscodePreset::Hapq => hap(&args, g, "hap_q"),
        TranscodePreset::Cfhd => qt_era(
            &args,
            g,
            "cfhd",
            &["mov", "avi"],
            Some("yuv422p10le"),
            Some("pcm_s16le"),
        ),
        TranscodePreset::Vc2 => qt_era(&args, g, "vc2", &["mov"], None, Some("pcm_s16le")),
        TranscodePreset::Magicyuv => lossless(&args, g, "magicyuv", &["avi"], None),
        TranscodePreset::R10k => lossless(&args, g, "r10k", &["mov"], Some("gbrp10le")),
        TranscodePreset::H261 => raw_telecom(&args, g, "h261", &[(176, 144), (352, 288)]),
        TranscodePreset::H263 => raw_telecom(
            &args,
            g,
            "h263",
            &[(128, 96), (176, 144), (352, 288), (704, 576), (1408, 1152)],
        ),
        TranscodePreset::Avui => avui(&args, g),
        TranscodePreset::Ts => qt_era(&args, g, "libx264", &["ts", "m2ts"], None, Some("aac")),
        TranscodePreset::Mxf => mxf(&args, g),
        TranscodePreset::Ivf => qt_era(&args, g, "libvpx-vp9", &["ivf"], None, None),
        TranscodePreset::Raw => lossless(&args, g, "rawvideo", &["avi", "mkv"], None),
    }
}

/// Edit proxy: capped at 540p high, veryfast x264 — smooth NLE scrubbing on
/// long takes / multicam dailies, never a delivery format.
fn proxy(args: &TranscodeArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_video {
        return Err(Error::input("proxy preset: input has no video"));
    }
    let crf = args.crf.unwrap_or(28);
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    if args.copy_video {
        argv.extend(["-c:v", "copy", "-movflags", "+faststart"]);
    } else {
        argv.extend([
            "-c:v",
            "libx264",
            "-preset",
            "veryfast",
            "-crf",
            &crf.to_string(),
            "-pix_fmt",
            "yuv420p",
            "-movflags",
            "+faststart",
        ]);
        x264spec_push(&mut argv, args);
        let mut vf = String::from("scale=w='min(960,iw)':h=-2");
        if let Some(fps) = args.fps {
            vf.push_str(&format!(",fps={fps}"));
        }
        vf.push_str(range_tag(args));
        vf.push_str(&interlace_tag(args));
        vf.push_str(field_tag(args));
        argv.extend(["-vf", &vf]);
    }
    if probe.has_audio {
        if args.copy_audio {
            argv.extend(["-c:a", "copy"]);
        } else {
            argv.extend(["-c:a", "aac", "-b:a", abitrate(args, "96k")]);
        }
    }
    cap_bitrate(&mut argv, &args.vbitrate);
    ar_ac(&mut argv, args);
    gop_push(&mut argv, args);
    argv.push(&args.output);
    let mut c = engine::write_job("transcode", &[&args.input], &args.output, vec![argv], g)?;
    c = c.with_extra(json!({"proxy": true, "copy_video": args.copy_video}));
    Ok(c)
}

fn range_tag(args: &TranscodeArgs) -> &'static str {
    match args.range {
        Some(crate::cli::TranscodeRange::Limited) => ",setparams=range=tv",
        Some(crate::cli::TranscodeRange::Full) => ",setparams=range=pc",
        None => "",
    }
}

fn field_tag(args: &TranscodeArgs) -> &'static str {
    // setparams only relabels the field_order flag — unlike setfield /
    // fieldorder it needs no flagged input, so it also FIXES wrong tags
    match args.field_order {
        Some(crate::cli::FieldOrder::Tff) => ",setparams=field_mode=tff",
        Some(crate::cli::FieldOrder::Bff) => ",setparams=field_mode=bff",
        Some(crate::cli::FieldOrder::Prog) => ",setparams=field_mode=prog",
        None => "",
    }
}

fn interlace_tag(args: &TranscodeArgs) -> String {
    // il interleaves fields (progressive → interlaced), setfield tags tff —
    // broadcast/air-master delivery specs want both the picture woven and
    // the container flag set. weave pairs real consecutive frames into
    // fields — true 2x-rate-to-interlace conversion (halves the fps).
    let mut s = String::new();
    if args.interlaced {
        s.push_str(match args.interlace_mode {
            // tinterlace interleave_top: consecutive frames → alternating
            // top/bottom fields — real temporal interlacing (60p→30i), NOT
            // ffmpeg's weave filter (that one stacks full frames vertically)
            Some(crate::cli::InterlaceKind::Weave) => {
                ",tinterlace=mode=interleave_top,setfield=tff"
            }
            _ => ",il=luma_mode=i:chroma_mode=i,setfield=tff",
        });
    }
    s
}

fn h264(args: &TranscodeArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    let crf = args.crf.unwrap_or(23);
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    if probe.has_video {
        if args.copy_video {
            argv.extend(["-c:v", "copy"]);
        } else {
            argv.extend([
                "-c:v",
                "libx264",
                "-preset",
                "medium",
                "-crf",
                &crf.to_string(),
                "-pix_fmt",
                "yuv420p",
                "-movflags",
                "+faststart",
            ]);
            x264spec_push(&mut argv, args);
            let mut vf = String::from("scale=trunc(iw/2)*2:trunc(ih/2)*2");
            if let Some(fps) = args.fps {
                vf.push_str(&format!(",fps={fps}"));
            }
            vf.push_str(range_tag(args));
            vf.push_str(&interlace_tag(args));
            vf.push_str(field_tag(args));
            argv.extend(["-vf", &vf]);
        }
    }
    if probe.has_audio {
        if args.copy_audio {
            argv.extend(["-c:a", "copy"]);
        } else {
            argv.extend(["-c:a", "aac", "-b:a", abitrate(args, "192k")]);
        }
    }
    cap_bitrate(&mut argv, &args.vbitrate);
    ar_ac(&mut argv, args);
    gop_push(&mut argv, args);
    argv.push(&args.output);
    engine::write_job("transcode", &[&args.input], &args.output, vec![argv], g)
}

fn hevc(args: &TranscodeArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    let crf = args.crf.unwrap_or(28);
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    if probe.has_video {
        if args.copy_video {
            argv.extend(["-c:v", "copy"]);
        } else {
            argv.extend([
                "-c:v",
                "libx265",
                "-preset",
                "medium",
                "-crf",
                &crf.to_string(),
                "-pix_fmt",
                "yuv420p",
                "-tag:v",
                "hvc1",
            ]);
            let mut vf = String::from("scale=trunc(iw/2)*2:trunc(ih/2)*2");
            if let Some(fps) = args.fps {
                vf.push_str(&format!(",fps={fps}"));
            }
            vf.push_str(range_tag(args));
            vf.push_str(&interlace_tag(args));
            vf.push_str(field_tag(args));
            argv.extend(["-vf", &vf]);
        }
    }
    if probe.has_audio {
        if args.copy_audio {
            argv.extend(["-c:a", "copy"]);
        } else {
            argv.extend(["-c:a", "aac", "-b:a", abitrate(args, "192k")]);
        }
    }
    cap_bitrate(&mut argv, &args.vbitrate);
    ar_ac(&mut argv, args);
    gop_push(&mut argv, args);
    argv.push(&args.output);
    engine::write_job("transcode", &[&args.input], &args.output, vec![argv], g)
}

fn webm(args: &TranscodeArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    let crf = args.crf.unwrap_or(32);
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    if probe.has_video {
        if args.copy_video {
            argv.extend(["-c:v", "copy"]);
        } else {
            argv.extend([
                "-c:v",
                "libvpx-vp9",
                "-b:v",
                "0",
                "-crf",
                &crf.to_string(),
                "-pix_fmt",
                if args.alpha { "yuva420p" } else { "yuv420p" },
            ]);
            let mut vf = String::from("scale=trunc(iw/2)*2:trunc(ih/2)*2");
            if let Some(fps) = args.fps {
                vf.push_str(&format!(",fps={fps}"));
            }
            vf.push_str(range_tag(args));
            vf.push_str(&interlace_tag(args));
            vf.push_str(field_tag(args));
            argv.extend(["-vf", &vf]);
        }
    }
    if probe.has_audio {
        if args.copy_audio {
            argv.extend(["-c:a", "copy"]);
        } else {
            argv.extend(["-c:a", "libopus", "-b:a", abitrate(args, "128k")]);
        }
    }
    cap_bitrate(&mut argv, &args.vbitrate);
    ar_ac(&mut argv, args);
    gop_push(&mut argv, args);
    argv.push(&args.output);
    engine::write_job("transcode", &[&args.input], &args.output, vec![argv], g)
}

/// AV1 delivery: libsvtav1 on ffmpeg ≥7 (fast), libaom on 4.x (row-mt +
/// cpu-used for usable speed). YouTube/web prefers AV1 for small files.
fn av1(args: &TranscodeArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    let crf = args.crf.unwrap_or(35).to_string();
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    if probe.has_video {
        if args.copy_video {
            argv.extend(["-c:v", "copy"]);
        } else {
            if crate::engine::ffmpeg_major().unwrap_or(7) >= 7 {
                argv.extend(["-c:v", "libsvtav1", "-preset", "6"]);
            } else {
                argv.extend(["-c:v", "libaom-av1", "-cpu-used", "4", "-row-mt", "1"]);
            }
            argv.extend(["-crf", &crf, "-b:v", "0", "-pix_fmt", "yuv420p"]);
            let mut vf = String::from("scale=trunc(iw/2)*2:trunc(ih/2)*2");
            if let Some(fps) = args.fps {
                vf.push_str(&format!(",fps={fps}"));
            }
            vf.push_str(range_tag(args));
            vf.push_str(&interlace_tag(args));
            vf.push_str(field_tag(args));
            argv.extend(["-vf", &vf]);
        }
    }
    if probe.has_audio {
        if args.copy_audio {
            argv.extend(["-c:a", "copy"]);
        } else {
            argv.extend(["-c:a", "libopus", "-b:a", abitrate(args, "128k")]);
        }
    }
    cap_bitrate(&mut argv, &args.vbitrate);
    ar_ac(&mut argv, args);
    gop_push(&mut argv, args);
    argv.push(&args.output);
    engine::write_job("transcode", &[&args.input], &args.output, vec![argv], g)
}

fn gif(args: &TranscodeArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "transcode")?;
    let palette = tempfile::Builder::new()
        .suffix(".png")
        .tempfile()
        .map_err(|e| Error::output(e.to_string()))?;
    let palette_path = palette.path().to_path_buf();

    let fps = args.fps.unwrap_or(10).clamp(1, 30);
    let width = args.width.unwrap_or(480).clamp(16, 1920);
    let scale = format!("fps={fps},scale={width}:-2:flags=lanczos");
    let mut gen = ffmpeg_base(g.progress);
    gen.push("-i");
    gen.push(&args.input);
    let max_colors = args.colors.unwrap_or(256).clamp(2, 256);
    gen.extend([
        "-vf",
        &format!("{scale},palettegen=stats_mode=full:max_colors={max_colors}"),
    ]);
    gen.push(&palette_path);

    let mut use_p = ffmpeg_base(g.progress);
    use_p.push("-i");
    use_p.push(&args.input);
    use_p.push("-i");
    use_p.push(&palette_path);
    use_p.extend([
        "-lavfi",
        &format!("{scale}[x];[x][1:v]paletteuse=dither=bayer"),
        "-an",
    ]);
    use_p.push(&args.output);

    let result = engine::write_job(
        "transcode",
        &[&args.input],
        &args.output,
        vec![gen, use_p],
        g,
    );
    drop(palette);
    result
}

/// Audio-only delivery: -vn + one codec. `--copy-audio` stream-copies instead
/// of re-encoding (e.g. mp4 → mp3 keeps nothing — copy only fits same-codec).
fn audio_only(
    args: &TranscodeArgs,
    g: &Globals,
    preset: TranscodePreset,
) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_audio {
        return Err(Error::input(
            "audio preset (mp3/aac/wav/flac/opus/ogg/alac/ac3/eac3/tta/dca/aiff/pcm24/pcm32f/mulaw/adx/adpcm/alaw/speex/pcm8/adpcmms/g722/ra144/nelly/wv/mp2/caf/w64/voc/aptx/sbc/g723/truehd/mlp): input has no audio",
        ));
    }
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.push("-vn");
    if args.copy_audio {
        argv.extend(["-c:a", "copy"]);
    } else {
        match preset {
            TranscodePreset::Mp3 => {
                argv.extend(["-c:a", "libmp3lame", "-b:a", abitrate(args, "192k")])
            }
            TranscodePreset::Wav => argv.extend(["-c:a", "pcm_s16le"]),
            TranscodePreset::Flac => argv.extend(["-c:a", "flac"]),
            TranscodePreset::Opus => {
                argv.extend(["-c:a", "libopus", "-b:a", abitrate(args, "128k")])
            }
            TranscodePreset::Ogg => {
                argv.extend(["-c:a", "libvorbis", "-b:a", abitrate(args, "192k")])
            }
            TranscodePreset::Alac => argv.extend(["-c:a", "alac"]),
            TranscodePreset::Ac3 => argv.extend(["-c:a", "ac3", "-b:a", abitrate(args, "192k")]),
            TranscodePreset::Eac3 => argv.extend(["-c:a", "eac3", "-b:a", abitrate(args, "192k")]),
            TranscodePreset::Tta => argv.extend(["-c:a", "tta"]),
            TranscodePreset::Dca => argv.extend([
                "-c:a",
                "dca",
                "-b:a",
                abitrate(args, "768k"),
                "-strict",
                "-2",
            ]),
            TranscodePreset::Aiff => argv.extend(["-c:a", "pcm_s16be"]),
            TranscodePreset::Pcm24 => argv.extend(["-c:a", "pcm_s24le"]),
            TranscodePreset::Pcm32f => argv.extend(["-c:a", "pcm_f32le"]),
            TranscodePreset::Mulaw => argv.extend(["-c:a", "pcm_mulaw", "-ar", "8000", "-ac", "1"]),
            TranscodePreset::Adx => argv.extend(["-c:a", "adpcm_adx"]),
            TranscodePreset::Adpcm => argv.extend(["-c:a", "adpcm_ima_wav"]),
            TranscodePreset::Alaw => argv.extend(["-c:a", "pcm_alaw", "-ar", "8000", "-ac", "1"]),
            TranscodePreset::Speex => argv.extend(["-c:a", "libspeex"]),
            TranscodePreset::Pcm8 => argv.extend(["-c:a", "pcm_u8"]),
            TranscodePreset::Adpcmms => argv.extend(["-c:a", "adpcm_ms"]),
            TranscodePreset::G722 => {
                argv.extend(["-c:a", "adpcm_g722", "-ar", "16000", "-ac", "1"])
            }
            TranscodePreset::Ra144 => argv.extend(["-c:a", "real_144", "-ar", "8000", "-ac", "1"]),
            TranscodePreset::Nelly => argv.extend(["-c:a", "nellymoser"]),
            TranscodePreset::Wv => argv.extend(["-c:a", "wavpack"]),
            TranscodePreset::Mp2 => argv.extend(["-c:a", "mp2", "-b:a", abitrate(args, "192k")]),
            TranscodePreset::Caf => argv.extend(["-c:a", "pcm_s16be"]),
            TranscodePreset::W64 => argv.extend(["-c:a", "pcm_s24le"]),
            TranscodePreset::Voc => argv.extend(["-c:a", "pcm_s16le"]),
            TranscodePreset::Aptx => argv.extend(["-c:a", "aptx"]),
            TranscodePreset::Sbc => argv.extend(["-c:a", "sbc"]),
            TranscodePreset::G723 => argv.extend(["-c:a", "g723_1", "-ar", "8000", "-ac", "1"]),
            TranscodePreset::Truehd => argv.extend(["-c:a", "truehd", "-strict", "-2"]),
            TranscodePreset::Mlp => argv.extend(["-c:a", "mlp", "-strict", "-2"]),
            _ => argv.extend(["-c:a", "aac", "-b:a", abitrate(args, "192k")]),
        }
    }
    ar_ac(&mut argv, args);
    gop_push(&mut argv, args);
    argv.push(&args.output);
    let c = engine::write_job("transcode", &[&args.input], &args.output, vec![argv], g)?;
    Ok(c.with_extra(json!({"audio_only": true})))
}

fn prores(args: &TranscodeArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    let ext = args
        .output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if ext != "mov" {
        return Err(Error::input(
            "prores preset wants a .mov output (ProRes + PCM in MOV)",
        ));
    }
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    if probe.has_video {
        if args.copy_video {
            argv.extend(["-c:v", "copy"]);
        } else {
            if args.alpha {
                argv.extend([
                    "-c:v",
                    "prores_ks",
                    "-profile:v",
                    "4",
                    "-pix_fmt",
                    "yuva444p10le",
                    "-vendor",
                    "apl0",
                ]);
            } else {
                argv.extend([
                    "-c:v",
                    "prores_ks",
                    "-profile:v",
                    "3",
                    "-pix_fmt",
                    "yuv422p10le",
                ]);
            }
            let fps_vf = args.fps.map(|fps| format!("fps={fps}")).unwrap_or_default();
            let prores_vf = format!(
                "{fps_vf}{}{}{}",
                range_tag(args),
                interlace_tag(args),
                field_tag(args)
            );
            if !prores_vf.is_empty() {
                argv.extend(["-vf", prores_vf.trim_start_matches(',')]);
            }
        }
    }
    if probe.has_audio {
        if args.copy_audio {
            argv.extend(["-c:a", "copy"]);
        } else {
            argv.extend(["-c:a", "pcm_s16le"]);
        }
    }
    cap_bitrate(&mut argv, &args.vbitrate);
    ar_ac(&mut argv, args);
    gop_push(&mut argv, args);
    argv.push(&args.output);
    engine::write_job("transcode", &[&args.input], &args.output, vec![argv], g)
}

/// DNxHR HQ in .mov — the Avid/Resolve-side edit handoff (ProRes on the
/// Apple side). dnxhr profiles take any resolution; pcm_s16le like prores.
fn dnxhd(args: &TranscodeArgs, g: &Globals) -> Result<Contract, Error> {
    let probe = engine::probe_or_err(&args.input, g)?;
    let ext = args
        .output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if ext != "mov" {
        return Err(Error::input(
            "dnxhd preset wants a .mov output (DNxHR + PCM in MOV)",
        ));
    }
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    if probe.has_video {
        if args.copy_video {
            argv.extend(["-c:v", "copy"]);
        } else {
            argv.extend([
                "-c:v",
                "dnxhd",
                "-profile:v",
                "dnxhr_hq",
                "-pix_fmt",
                "yuv422p",
            ]);
            let mut vf = String::from("scale=trunc(iw/2)*2:trunc(ih/2)*2");
            if let Some(fps) = args.fps {
                vf.push_str(&format!(",fps={fps}"));
            }
            vf.push_str(range_tag(args));
            vf.push_str(&interlace_tag(args));
            vf.push_str(field_tag(args));
            argv.extend(["-vf", &vf]);
        }
    }
    if probe.has_audio {
        if args.copy_audio {
            argv.extend(["-c:a", "copy"]);
        } else {
            argv.extend(["-c:a", "pcm_s16le"]);
        }
    }
    ar_ac(&mut argv, args);
    gop_push(&mut argv, args);
    argv.push(&args.output);
    engine::write_job("transcode", &[&args.input], &args.output, vec![argv], g)
}

/// Vidvox HAP in .mov/.avi — the DXT-texture live-visual codec (Resolume,
/// TouchDesigner, VJ ingest). `format` is the encoder's -format variant:
/// hap ships DXT1 (upgraded to hap_alpha/DXT5 by --alpha), hap_q is the
/// higher-quality YCoCg variant.
fn hap(args: &TranscodeArgs, g: &Globals, format: &str) -> Result<Contract, Error> {
    let ext = args
        .output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if !["mov", "avi"].contains(&ext.as_str()) {
        return Err(Error::input(format!(
            "transcode --preset {format} needs a .mov/.avi target, not .{ext}"
        )));
    }
    if args.crf.is_some() || args.abitrate.is_some() {
        return Err(Error::input(format!(
            "transcode --preset {format} has no crf/abitrate knobs — use --vbitrate"
        )));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_video {
        return Err(Error::input(format!("{format} preset: input has no video")));
    }
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-map", "0:v?"]);
    if probe.has_audio {
        argv.extend(["-map", "0:a?"]);
    }
    argv.extend(["-c:v", "hap"]);
    let format = if format == "hap" && args.alpha {
        "hap_alpha"
    } else {
        format
    };
    argv.extend(["-format", format]);
    if let Some(b) = &args.vbitrate {
        argv.extend(["-b:v", b]);
    }
    if let Some(n) = args.gop {
        argv.extend(["-g", &n.to_string()]);
    }
    if probe.has_audio {
        argv.extend(["-c:a", "pcm_s16le"]);
        if let Some(r) = args.ar {
            argv.extend(["-ar", &r.to_string()]);
        }
        if let Some(ch) = args.channels {
            argv.extend(["-ac", &ch.to_string()]);
        }
    }
    if let Some(fps) = args.fps {
        argv.extend(["-r", &fps.to_string()]);
    }
    argv.push(&args.output);
    let mut c = engine::write_job("transcode", &[&args.input], &args.output, vec![argv], g)?;
    c = c.with_extra(json!({ "preset": format }));
    Ok(c)
}

/// Raw elementary H.261/H.263 — QCIF/CIF-era videoconference test
/// vectors (H.324 terminals, codec-conformance suites). Both codecs
/// encode only a fixed set of picture sizes so the picture snaps to the
/// legal canvas letterboxed (same snap as gpp's .3gp path), and a raw
/// stream carries no audio — audio-shaped flags refuse instead of
/// silently dropping.
fn raw_telecom(
    args: &TranscodeArgs,
    g: &Globals,
    codec: &str,
    legal: &[(u32, u32)],
) -> Result<Contract, Error> {
    let ext = args
        .output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if ext != codec {
        return Err(Error::input(format!(
            "transcode --preset {codec} needs a .{codec} target, not .{ext}"
        )));
    }
    if args.crf.is_some() || args.abitrate.is_some() || args.ar.is_some() || args.channels.is_some()
    {
        return Err(Error::input(format!(
            "transcode --preset {codec} is a raw video stream — no crf/audio flags (drop --ar/--channels/--abitrate too)"
        )));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_video {
        return Err(Error::input(format!("{codec} preset: input has no video")));
    }
    let area = (probe.width.unwrap_or(176) as u64) * (probe.height.unwrap_or(144) as u64);
    let (w, h) = legal
        .iter()
        .min_by_key(|(lw, lh)| ((lw * lh) as u64).abs_diff(area))
        .copied()
        .unwrap();
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-map", "0:v?"]);
    argv.extend([
        "-vf",
        &format!("scale=w={w}:h={h}:force_original_aspect_ratio=decrease,pad={w}:{h}:(ow-iw)/2:(oh-ih)/2"),
    ]);
    argv.extend(["-c:v", codec]);
    if let Some(b) = &args.vbitrate {
        argv.extend(["-b:v", b]);
    }
    if let Some(n) = args.gop {
        argv.extend(["-g", &n.to_string()]);
    }
    if let Some(fps) = args.fps {
        argv.extend(["-r", &fps.to_string()]);
    }
    argv.push(&args.output);
    let mut c = engine::write_job("transcode", &[&args.input], &args.output, vec![argv], g)?;
    c = c.with_extra(json!({ "preset": codec }));
    Ok(c)
}

/// Avid Meridien uncompressed + PCM in .mov — the broadcast capture-card
/// ingest spec (Avid Unity/ISIS-era decks). Meridien accepts only
/// 720x486 (NTSC) and 720x576 (PAL) so the picture snaps to the NTSC
/// canvas letterboxed; the encoder is experimental on 4.4 so the preset
/// ships -strict -2. Uncompressed: bitrate/gop flags refuse.
fn avui(args: &TranscodeArgs, g: &Globals) -> Result<Contract, Error> {
    let ext = args
        .output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if ext != "mov" {
        return Err(Error::input(format!(
            "transcode --preset avui needs a .mov target, not .{ext}"
        )));
    }
    if args.vbitrate.is_some()
        || args.crf.is_some()
        || args.abitrate.is_some()
        || args.gop.is_some()
    {
        return Err(Error::input(
            "transcode --preset avui is uncompressed intra-frame — bitrate/crf/gop flags don't apply",
        ));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_video {
        return Err(Error::input("avui preset: input has no video"));
    }
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-map", "0:v?"]);
    if probe.has_audio {
        argv.extend(["-map", "0:a?"]);
    }
    argv.extend([
        "-vf",
        "scale=w=720:h=486:force_original_aspect_ratio=decrease,pad=720:486:(ow-iw)/2:(oh-ih)/2",
    ]);
    argv.extend(["-c:v", "avui", "-strict", "-2"]);
    if probe.has_audio {
        argv.extend(["-c:a", "pcm_s16le"]);
        if let Some(r) = args.ar {
            argv.extend(["-ar", &r.to_string()]);
        }
        if let Some(ch) = args.channels {
            argv.extend(["-ac", &ch.to_string()]);
        }
    }
    if let Some(fps) = args.fps {
        argv.extend(["-r", &fps.to_string()]);
    }
    argv.push(&args.output);
    let mut c = engine::write_job("transcode", &[&args.input], &args.output, vec![argv], g)?;
    c = c.with_extra(json!({ "preset": "avui" }));
    Ok(c)
}

// Broadcast master in MXF — XDCAM/OP1a: mpeg2video 4:2:2 + 48kHz stereo
// PCM are the spec constants (the muxer refuses other audio); vbitrate/
// gop/fps stay tunable so delivery specs can pin them
fn mxf(args: &TranscodeArgs, g: &Globals) -> Result<Contract, Error> {
    let ext = args
        .output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if ext != "mxf" {
        return Err(Error::input(format!(
            "transcode --preset mxf needs a .mxf target, not .{ext}"
        )));
    }
    if args.crf.is_some() || args.abitrate.is_some() || args.ar.is_some() || args.channels.is_some()
    {
        return Err(Error::input(
            "transcode --preset mxf is spec-pinned — XDCAM audio is always 48kHz stereo PCM (drop --ar/--channels/--crf/--abitrate)",
        ));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_video {
        return Err(Error::input("mxf preset: input has no video"));
    }
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-map", "0:v?"]);
    if probe.has_audio {
        argv.extend(["-map", "0:a?"]);
    }
    argv.extend(["-c:v", "mpeg2video", "-pix_fmt", "yuv422p"]);
    if let Some(b) = &args.vbitrate {
        argv.extend(["-b:v", b]);
    }
    if let Some(n) = args.gop {
        argv.extend(["-g", &n.to_string()]);
    }
    if probe.has_audio {
        argv.extend(["-c:a", "pcm_s16le", "-ar", "48000", "-ac", "2"]);
    }
    if let Some(fps) = args.fps {
        argv.extend(["-r", &fps.to_string()]);
    }
    argv.push(&args.output);
    let mut c = engine::write_job("transcode", &[&args.input], &args.output, vec![argv], g)?;
    c = c.with_extra(json!({ "preset": "mxf" }));
    Ok(c)
}

fn abitrate<'a>(args: &'a TranscodeArgs, default: &'a str) -> &'a str {
    args.abitrate.as_deref().unwrap_or(default)
}

// --ar/--channels apply only on a real re-encode; a stream copy has no
// encoder to resample/remap
fn ar_ac(argv: &mut Argv, args: &TranscodeArgs) {
    if args.copy_audio {
        return;
    }
    if let Some(ar) = args.ar {
        argv.extend(["-ar", &ar.to_string()]);
    }
    if let Some(ch) = args.channels {
        argv.extend(["-ac", &ch.to_string()]);
    }
}

/// FFV1 lossless archival master — mathematically lossless video + flac
/// audio in .mkv (the museum/NLE-safe intermediate; only mkv carries ffv1).
fn ffv1(args: &TranscodeArgs, g: &Globals) -> Result<Contract, Error> {
    let ext = args
        .output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if ext != "mkv" {
        return Err(Error::input(format!(
            "transcode --preset ffv1 needs a .mkv target (only mkv carries ffv1), not .{ext}"
        )));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_video {
        return Err(Error::input("ffv1 preset: input has no video"));
    }
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-map", "0:v?"]);
    if probe.has_audio {
        argv.extend(["-map", "0:a?"]);
    }
    argv.extend(["-c:v", "ffv1"]);
    if let Some(n) = args.gop {
        argv.extend(["-g", &n.to_string()]);
    }
    if probe.has_audio {
        argv.extend(["-c:a", "flac"]);
    }
    if let Some(fps) = args.fps {
        argv.extend(["-r", &fps.to_string()]);
    }
    argv.push(&args.output);
    let mut c = engine::write_job("transcode", &[&args.input], &args.output, vec![argv], g)?;
    c = c.with_extra(json!({ "preset": "ffv1" }));
    Ok(c)
}

/// MPEG-2 + MP2 — DVD/broadcast legacy master (.mpg/.mpeg/.vob): set-top
/// players, TV ingest, archival interop with decades-old systems.
fn mpeg2(args: &TranscodeArgs, g: &Globals) -> Result<Contract, Error> {
    let ext = args
        .output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if !matches!(ext.as_str(), "mpg" | "mpeg" | "vob") {
        return Err(Error::input(format!(
            "transcode --preset mpeg2 needs a .mpg/.mpeg/.vob target, not .{ext}"
        )));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_video {
        return Err(Error::input("mpeg2 preset: input has no video"));
    }
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-map", "0:v?"]);
    if probe.has_audio {
        argv.extend(["-map", "0:a?"]);
    }
    argv.extend(["-c:v", "mpeg2video"]);
    if let Some(n) = args.gop {
        argv.extend(["-g", &n.to_string()]);
    }
    if probe.has_audio {
        argv.extend(["-c:a", "mp2"]);
    }
    if let Some(fps) = args.fps {
        argv.extend(["-r", &fps.to_string()]);
    }
    argv.push(&args.output);
    let mut c = engine::write_job("transcode", &[&args.input], &args.output, vec![argv], g)?;
    c = c.with_extra(json!({ "preset": "mpeg2" }));
    Ok(c)
}

/// MPEG-1 + MP2 — VCD-era legacy master (.mpg): the oldest digital video
/// format still in playback circulation, max interop with ancient gear.
fn mpeg1(args: &TranscodeArgs, g: &Globals) -> Result<Contract, Error> {
    let ext = args
        .output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if !matches!(ext.as_str(), "mpg" | "mpeg") {
        return Err(Error::input(format!(
            "transcode --preset mpeg1 needs a .mpg/.mpeg target, not .{ext}"
        )));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_video {
        return Err(Error::input("mpeg1 preset: input has no video"));
    }
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-map", "0:v?"]);
    if probe.has_audio {
        argv.extend(["-map", "0:a?"]);
    }
    argv.extend(["-c:v", "mpeg1video"]);
    if let Some(n) = args.gop {
        argv.extend(["-g", &n.to_string()]);
    }
    if probe.has_audio {
        argv.extend(["-c:a", "mp2"]);
    }
    if let Some(fps) = args.fps {
        argv.extend(["-r", &fps.to_string()]);
    }
    argv.push(&args.output);
    let mut c = engine::write_job("transcode", &[&args.input], &args.output, vec![argv], g)?;
    c = c.with_extra(json!({ "preset": "mpeg1" }));
    Ok(c)
}

/// Xvid/MPEG-4 part 2 + MP3 in .avi — the legacy-rip master: DivX-era
/// players, projectors, and set-top boxes that only read .avi containers.
fn xvid(args: &TranscodeArgs, g: &Globals) -> Result<Contract, Error> {
    let ext = args
        .output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if ext != "avi" {
        return Err(Error::input(format!(
            "transcode --preset xvid needs a .avi target, not .{ext}"
        )));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_video {
        return Err(Error::input("xvid preset: input has no video"));
    }
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-map", "0:v?"]);
    if probe.has_audio {
        argv.extend(["-map", "0:a?"]);
    }
    argv.extend(["-c:v", "libxvid", "-vtag", "xvid"]);
    if let Some(n) = args.gop {
        argv.extend(["-g", &n.to_string()]);
    }
    if probe.has_audio {
        argv.extend(["-c:a", "libmp3lame"]);
    }
    if let Some(fps) = args.fps {
        argv.extend(["-r", &fps.to_string()]);
    }
    argv.push(&args.output);
    let mut c = engine::write_job("transcode", &[&args.input], &args.output, vec![argv], g)?;
    c = c.with_extra(json!({ "preset": "xvid" }));
    Ok(c)
}

/// MS-MPEG4 v2 + MP3 in .avi — the pre-DivX Windows codec (MP42 tag):
/// Windows ME-era screen captures and players older than the Xvid era.
/// H.263 + AMR-NB in .3gp — the feature-phone master: MMS-era mobile
/// video and J2ME handsets. AMR-NB speech audio is 8kHz mono — anything
/// else fails the codec's own constraint, so it's forced not probed.
/// FLV1 + MP3 in .flv — the Flash-era web master: YouTube 2005-era uploads,
/// Flash video archives, players that still only speak .flv.
fn flv(args: &TranscodeArgs, g: &Globals) -> Result<Contract, Error> {
    let ext = args
        .output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if ext != "flv" {
        return Err(Error::input(format!(
            "transcode --preset flv needs a .flv target, not .{ext}"
        )));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_video {
        return Err(Error::input("flv preset: input has no video"));
    }
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-map", "0:v?"]);
    if probe.has_audio {
        argv.extend(["-map", "0:a?"]);
    }
    argv.extend(["-c:v", "flv"]);
    if let Some(n) = args.gop {
        argv.extend(["-g", &n.to_string()]);
    }
    if probe.has_audio {
        argv.extend(["-c:a", "libmp3lame"]);
    }
    if let Some(fps) = args.fps {
        argv.extend(["-r", &fps.to_string()]);
    }
    argv.push(&args.output);
    let mut c = engine::write_job("transcode", &[&args.input], &args.output, vec![argv], g)?;
    c = c.with_extra(json!({ "preset": "flv" }));
    Ok(c)
}

/// Theora + Vorbis in .ogv — the open-web master: pre-WebM HTML5 video,
/// Wikipedia/Wikimedia embeds, FLOSS-only playback chains.
fn theora(args: &TranscodeArgs, g: &Globals) -> Result<Contract, Error> {
    let ext = args
        .output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if !matches!(ext.as_str(), "ogv" | "ogg") {
        return Err(Error::input(format!(
            "transcode --preset theora needs a .ogv/.ogg target, not .{ext}"
        )));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_video {
        return Err(Error::input("theora preset: input has no video"));
    }
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-map", "0:v?"]);
    if probe.has_audio {
        argv.extend(["-map", "0:a?"]);
    }
    argv.extend(["-c:v", "libtheora", "-q:v", "5"]);
    if let Some(n) = args.gop {
        argv.extend(["-g", &n.to_string()]);
    }
    if probe.has_audio {
        argv.extend(["-c:a", "libvorbis"]);
    }
    if let Some(fps) = args.fps {
        argv.extend(["-r", &fps.to_string()]);
    }
    argv.push(&args.output);
    let mut c = engine::write_job("transcode", &[&args.input], &args.output, vec![argv], g)?;
    c = c.with_extra(json!({ "preset": "theora" }));
    Ok(c)
}

fn gpp(args: &TranscodeArgs, g: &Globals) -> Result<Contract, Error> {
    let ext = args
        .output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if ext != "3gp" && ext != "3g2" {
        return Err(Error::input(format!(
            "transcode --preset gpp needs a .3gp/.3g2 target, not .{ext}"
        )));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_video {
        return Err(Error::input("gpp preset: input has no video"));
    }
    // H.263 encodes only five fixed picture sizes — snap to the nearest
    // legal canvas, letterboxed so the frame never distorts
    const LEGAL: [(u32, u32); 5] = [(128, 96), (176, 144), (352, 288), (704, 576), (1408, 1152)];
    let area = (probe.width.unwrap_or(176) as u64) * (probe.height.unwrap_or(144) as u64);
    let (w, h) = LEGAL
        .iter()
        .min_by_key(|(lw, lh)| ((lw * lh) as u64).abs_diff(area))
        .copied()
        .unwrap();
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-map", "0:v?"]);
    if probe.has_audio {
        argv.extend(["-map", "0:a?"]);
    }
    argv.extend([
        "-vf",
        &format!("scale=w={w}:h={h}:force_original_aspect_ratio=decrease,pad={w}:{h}:(ow-iw)/2:(oh-ih)/2"),
    ]);
    argv.extend(["-c:v", "h263"]);
    if let Some(n) = args.gop {
        argv.extend(["-g", &n.to_string()]);
    }
    if probe.has_audio {
        argv.extend(["-c:a", "libopencore_amrnb", "-ar", "8000", "-ac", "1"]);
    }
    if let Some(fps) = args.fps {
        argv.extend(["-r", &fps.to_string()]);
    }
    argv.push(&args.output);
    let mut c = engine::write_job("transcode", &[&args.input], &args.output, vec![argv], g)?;
    c = c.with_extra(json!({ "preset": "gpp" }));
    Ok(c)
}

/// DV25 in .dv/.avi — the camcorder-tape master: MiniDV/DVCAM archives,
/// NLE-era broadcast decks. DV is a fixed spec, not a tuning surface:
/// NTSC 720x480@30000/1001 yuv411p video + PCM 48kHz stereo audio, so the
/// preset snaps the picture to the legal canvas letterboxed and refuses
/// every flag that would break the spec (like gpp's legal-canvas snap).
fn dv(args: &TranscodeArgs, g: &Globals) -> Result<Contract, Error> {
    let ext = args
        .output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if !matches!(ext.as_str(), "dv" | "avi") {
        return Err(Error::input(format!(
            "transcode --preset dv needs a .dv/.avi target, not .{ext}"
        )));
    }
    if args.fps.is_some()
        || args.gop.is_some()
        || args.range.is_some()
        || args.field_order.is_some()
        || args.interlace_mode.is_some()
        || args.interlaced
        || args.ar.is_some()
        || args.channels.is_some()
        || args.vbitrate.is_some()
        || args.abitrate.is_some()
        || args.crf.is_some()
        || args.width.is_some()
        || args.colors.is_some()
    {
        return Err(Error::input(
            "transcode --preset dv is a fixed spec (720x480@30000/1001 yuv411p + PCM 48kHz stereo) — tuning flags don't apply",
        ));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_video {
        return Err(Error::input("dv preset: input has no video"));
    }
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-map", "0:v?"]);
    if probe.has_audio {
        argv.extend(["-map", "0:a?"]);
    }
    argv.extend([
        "-vf",
        "scale=w=720:h=480:force_original_aspect_ratio=decrease,pad=720:480:(ow-iw)/2:(oh-ih)/2,fps=30000/1001",
    ]);
    argv.extend(["-c:v", "dvvideo", "-pix_fmt", "yuv411p"]);
    if probe.has_audio {
        argv.extend(["-c:a", "pcm_s16le", "-ar", "48000", "-ac", "2"]);
    }
    argv.push(&args.output);
    let mut c = engine::write_job("transcode", &[&args.input], &args.output, vec![argv], g)?;
    c = c.with_extra(json!({ "preset": "dv" }));
    Ok(c)
}

/// Motion JPEG + MP3 in .avi/.mov — the NLE-era editing format: Digital
/// Betacam captures and frame-accurate scrub masters (every frame is an
/// intra JPEG — random access with zero decode dependencies).
fn mjpeg(args: &TranscodeArgs, g: &Globals) -> Result<Contract, Error> {
    let ext = args
        .output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if !matches!(ext.as_str(), "avi" | "mov") {
        return Err(Error::input(format!(
            "transcode --preset mjpeg needs a .avi/.mov target, not .{ext}"
        )));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_video {
        return Err(Error::input("mjpeg preset: input has no video"));
    }
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-map", "0:v?"]);
    if probe.has_audio {
        argv.extend(["-map", "0:a?"]);
    }
    argv.extend(["-c:v", "mjpeg"]);
    if let Some(b) = &args.vbitrate {
        argv.extend(["-b:v", b]);
    }
    if let Some(n) = args.gop {
        argv.extend(["-g", &n.to_string()]);
    }
    if probe.has_audio {
        argv.extend(["-c:a", "libmp3lame", "-b:a", abitrate(args, "192k")]);
        if let Some(r) = args.ar {
            argv.extend(["-ar", &r.to_string()]);
        }
        if let Some(ch) = args.channels {
            argv.extend(["-ac", &ch.to_string()]);
        }
    }
    if let Some(fps) = args.fps {
        argv.extend(["-r", &fps.to_string()]);
    }
    argv.push(&args.output);
    let mut c = engine::write_job("transcode", &[&args.input], &args.output, vec![argv], g)?;
    c = c.with_extra(json!({ "preset": "mjpeg" }));
    Ok(c)
}

/// AMV in .amv — the Chinese handheld-player master: MP3/MP4 players
/// circa 2006. Another fixed spec like dv: AMV video is 160x120 (the muxer
/// needs -vstrict -1 for the non-16-multiple height) + adpcm_ima_amv at
/// 22050Hz mono, so the preset snaps to the legal canvas letterboxed and
/// refuses every tuning flag.
fn amv(args: &TranscodeArgs, g: &Globals) -> Result<Contract, Error> {
    let ext = args
        .output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if ext != "amv" {
        return Err(Error::input(format!(
            "transcode --preset amv needs a .amv target, not .{ext}"
        )));
    }
    if args.fps.is_some()
        || args.gop.is_some()
        || args.range.is_some()
        || args.field_order.is_some()
        || args.interlace_mode.is_some()
        || args.interlaced
        || args.ar.is_some()
        || args.channels.is_some()
        || args.vbitrate.is_some()
        || args.abitrate.is_some()
        || args.crf.is_some()
        || args.width.is_some()
        || args.colors.is_some()
    {
        return Err(Error::input(
            "transcode --preset amv is a fixed spec (160x120 + ADPCM 22050Hz mono) — tuning flags don't apply",
        ));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_video {
        return Err(Error::input("amv preset: input has no video"));
    }
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-map", "0:v?"]);
    if probe.has_audio {
        argv.extend(["-map", "0:a?"]);
    }
    argv.extend([
        "-vf",
        "scale=w=160:h=120:force_original_aspect_ratio=decrease,pad=160:120:(ow-iw)/2:(oh-ih)/2,fps=25",
    ]);
    argv.extend(["-c:v", "amv", "-vstrict", "-1"]);
    if probe.has_audio {
        // the amv muxer demands block_size == sample_rate / video_fps —
        // the chain above pins 22050Hz / 25fps, so 882 always
        argv.extend(["-c:a", "adpcm_ima_amv", "-block_size", "882"]);
        argv.extend(["-ar", "22050", "-ac", "1"]);
    }
    argv.push(&args.output);
    let mut c = engine::write_job("transcode", &[&args.input], &args.output, vec![argv], g)?;
    c = c.with_extra(json!({ "preset": "amv" }));
    Ok(c)
}

fn msmpeg4(args: &TranscodeArgs, g: &Globals) -> Result<Contract, Error> {
    let ext = args
        .output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if ext != "avi" {
        return Err(Error::input(format!(
            "transcode --preset msmpeg4 needs a .avi target, not .{ext}"
        )));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_video {
        return Err(Error::input("msmpeg4 preset: input has no video"));
    }
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-map", "0:v?"]);
    if probe.has_audio {
        argv.extend(["-map", "0:a?"]);
    }
    // -vtag mp42: legacy players fourcc-check for the MS-MPEG4 v2 tag —
    // an unlabeled stream decodes fine but gets refused by the players
    // this preset exists for
    argv.extend(["-c:v", "msmpeg4v2", "-vtag", "mp42"]);
    if let Some(n) = args.gop {
        argv.extend(["-g", &n.to_string()]);
    }
    if probe.has_audio {
        argv.extend(["-c:a", "libmp3lame"]);
    }
    if let Some(fps) = args.fps {
        argv.extend(["-r", &fps.to_string()]);
    }
    argv.push(&args.output);
    let mut c = engine::write_job("transcode", &[&args.input], &args.output, vec![argv], g)?;
    c = c.with_extra(json!({ "preset": "msmpeg4" }));
    Ok(c)
}

/// WMV2 + WMA in .wmv/.asf — Windows Media-era master: corporate training
/// archives, old PowerPoint-embedded video, Windows-only playback gear.
fn wmv(args: &TranscodeArgs, g: &Globals) -> Result<Contract, Error> {
    let ext = args
        .output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if !matches!(ext.as_str(), "wmv" | "asf") {
        return Err(Error::input(format!(
            "transcode --preset wmv needs a .wmv/.asf target, not .{ext}"
        )));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_video {
        return Err(Error::input("wmv preset: input has no video"));
    }
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-map", "0:v?"]);
    if probe.has_audio {
        argv.extend(["-map", "0:a?"]);
    }
    argv.extend(["-c:v", "wmv2"]);
    if let Some(n) = args.gop {
        argv.extend(["-g", &n.to_string()]);
    }
    if probe.has_audio {
        argv.extend(["-c:a", "wmav2"]);
    }
    if let Some(fps) = args.fps {
        argv.extend(["-r", &fps.to_string()]);
    }
    argv.push(&args.output);
    let mut c = engine::write_job("transcode", &[&args.input], &args.output, vec![argv], g)?;
    c = c.with_extra(json!({ "preset": "wmv" }));
    Ok(c)
}

/// Animated PNG — full-color looping stickers/reactions where gif's 256
/// colors band (apng plays everywhere gif does; loops forever).
fn apng(args: &TranscodeArgs, g: &Globals) -> Result<Contract, Error> {
    let ext = args
        .output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if !matches!(ext.as_str(), "apng" | "png") {
        return Err(Error::input(format!(
            "transcode --preset apng needs a .apng/.png target, not .{ext}"
        )));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_video {
        return Err(Error::input("apng preset: input has no video"));
    }
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-map", "0:v?", "-f", "apng", "-plays", "0"]);
    if let Some(fps) = args.fps {
        argv.extend(["-vf", &format!("fps={fps}")]);
    }
    argv.push(&args.output);
    let mut c = engine::write_job("transcode", &[&args.input], &args.output, vec![argv], g)?;
    c = c.with_extra(json!({ "preset": "apng", "loop": "forever" }));
    Ok(c)
}

fn x264spec_push(argv: &mut Argv, args: &TranscodeArgs) {
    if let Some(p) = args.profile {
        argv.extend(["-profile:v", transcode_profile_name(p)]);
    }
    if let Some(l) = &args.level {
        argv.extend(["-level:v", l]);
    }
    if let Some(n) = args.bf {
        argv.extend(["-bf", &n.to_string()]);
    }
    if let Some(t) = args.tune {
        argv.extend(["-tune", transcode_tune_name(t)]);
    }
}

fn transcode_tune_name(t: crate::cli::TranscodeTune) -> &'static str {
    match t {
        crate::cli::TranscodeTune::Film => "film",
        crate::cli::TranscodeTune::Animation => "animation",
        crate::cli::TranscodeTune::Grain => "grain",
        crate::cli::TranscodeTune::Zerolatency => "zerolatency",
        crate::cli::TranscodeTune::Fastdecode => "fastdecode",
        crate::cli::TranscodeTune::Stillimage => "stillimage",
        crate::cli::TranscodeTune::Psnr => "psnr",
        crate::cli::TranscodeTune::Ssim => "ssim",
    }
}

pub(crate) fn transcode_profile_name(p: crate::cli::TranscodeProfile) -> &'static str {
    match p {
        crate::cli::TranscodeProfile::Baseline => "baseline",
        crate::cli::TranscodeProfile::Main => "main",
        crate::cli::TranscodeProfile::High => "high",
    }
}

fn gop_push(argv: &mut Argv, args: &TranscodeArgs) {
    if let Some(n) = args.gop {
        argv.extend(["-g", &n.to_string()]);
    }
}

fn cap_bitrate(argv: &mut crate::spawn::Argv, rate: &Option<String>) {
    let Some(r) = rate else { return };
    argv.extend(["-maxrate", r.as_str(), "-bufsize", &double_rate(r)]);
}

fn double_rate(r: &str) -> String {
    let (num, suf) = match r.chars().last() {
        Some(c) if c.is_ascii_alphabetic() => (&r[..r.len() - 1], &r[r.len() - 1..]),
        _ => (r, ""),
    };
    let n: f64 = num.parse().unwrap_or(0.0);
    format!("{}{}", (n * 2.0).round() as i64, suf)
}

/// QuickTime Animation RLE in .mov — the lossless animation/screencast
/// master. Every frame is intra RLE (zero decode dependency like mjpeg,
/// but lossless), and argb keeps the alpha channel for motion-graphics
/// interchange.
fn qtrle(args: &TranscodeArgs, g: &Globals) -> Result<Contract, Error> {
    let ext = args
        .output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if !matches!(ext.as_str(), "mov" | "qt") {
        return Err(Error::input(format!(
            "transcode --preset qtrle needs a .mov/.qt target, not .{ext}"
        )));
    }
    if args.vbitrate.is_some() || args.crf.is_some() || args.abitrate.is_some() {
        return Err(Error::input(
            "transcode --preset qtrle is lossless — bitrate/crf flags don't apply",
        ));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_video {
        return Err(Error::input("qtrle preset: input has no video"));
    }
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-map", "0:v?"]);
    if probe.has_audio {
        argv.extend(["-map", "0:a?"]);
    }
    argv.extend(["-c:v", "qtrle"]);
    argv.extend(["-pix_fmt", if args.alpha { "argb" } else { "rgb24" }]);
    if let Some(n) = args.gop {
        argv.extend(["-g", &n.to_string()]);
    }
    if probe.has_audio {
        argv.extend(["-c:a", "pcm_s16le"]);
        if let Some(r) = args.ar {
            argv.extend(["-ar", &r.to_string()]);
        }
        if let Some(ch) = args.channels {
            argv.extend(["-ac", &ch.to_string()]);
        }
    }
    if let Some(fps) = args.fps {
        argv.extend(["-r", &fps.to_string()]);
    }
    argv.push(&args.output);
    let mut c = engine::write_job("transcode", &[&args.input], &args.output, vec![argv], g)?;
    c = c.with_extra(json!({ "preset": "qtrle" }));
    Ok(c)
}

/// Uncompressed 10-bit 4:2:2 in .mov — the broadcast/edit-bay master.
/// v210 packs YUV 422 10-bit into 32-bit words (AJA Kona / broadcast
/// ingest spec). Bitrate flags are meaningless on an uncompressed codec;
/// audio stays PCM.
fn v210(args: &TranscodeArgs, g: &Globals) -> Result<Contract, Error> {
    let ext = args
        .output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if ext != "mov" {
        return Err(Error::input(format!(
            "transcode --preset v210 needs a .mov target, not .{ext}"
        )));
    }
    if args.vbitrate.is_some()
        || args.crf.is_some()
        || args.abitrate.is_some()
        || args.gop.is_some()
    {
        return Err(Error::input(
            "transcode --preset v210 is uncompressed — bitrate/crf/gop flags don't apply",
        ));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_video {
        return Err(Error::input("v210 preset: input has no video"));
    }
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-map", "0:v?"]);
    if probe.has_audio {
        argv.extend(["-map", "0:a?"]);
    }
    argv.extend(["-c:v", "v210", "-pix_fmt", "yuv422p10le"]);
    if probe.has_audio {
        argv.extend(["-c:a", "pcm_s16le"]);
        if let Some(r) = args.ar {
            argv.extend(["-ar", &r.to_string()]);
        }
        if let Some(ch) = args.channels {
            argv.extend(["-ac", &ch.to_string()]);
        }
    }
    if let Some(fps) = args.fps {
        argv.extend(["-r", &fps.to_string()]);
    }
    argv.push(&args.output);
    let mut c = engine::write_job("transcode", &[&args.input], &args.output, vec![argv], g)?;
    c = c.with_extra(json!({ "preset": "v210" }));
    Ok(c)
}

/// Shared lossless-intermediate path — huffyuv / utvideo / ffvhuff are
/// all intra-frame lossless codecs for edit masters (NLE-era capture and
/// VirtualDub/matroska workflows). PCM audio; bitrate/crf flags refused.
fn lossless(
    args: &TranscodeArgs,
    g: &Globals,
    codec: &str,
    exts: &[&str],
    pix_fmt: Option<&str>,
) -> Result<Contract, Error> {
    let ext = args
        .output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if !exts.contains(&ext.as_str()) {
        return Err(Error::input(format!(
            "transcode --preset {codec} needs a {} target, not .{ext}",
            exts.iter()
                .map(|e| format!(".{e}"))
                .collect::<Vec<_>>()
                .join("/")
        )));
    }
    if args.vbitrate.is_some() || args.crf.is_some() || args.abitrate.is_some() {
        return Err(Error::input(format!(
            "transcode --preset {codec} is lossless — bitrate/crf flags don't apply"
        )));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_video {
        return Err(Error::input(format!("{codec} preset: input has no video")));
    }
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-map", "0:v?"]);
    if probe.has_audio {
        argv.extend(["-map", "0:a?"]);
    }
    argv.extend(["-c:v", codec]);
    if let Some(p) = pix_fmt {
        argv.extend(["-pix_fmt", p]);
    }
    if let Some(n) = args.gop {
        argv.extend(["-g", &n.to_string()]);
    }
    if probe.has_audio {
        argv.extend(["-c:a", "pcm_s16le"]);
        if let Some(r) = args.ar {
            argv.extend(["-ar", &r.to_string()]);
        }
        if let Some(ch) = args.channels {
            argv.extend(["-ac", &ch.to_string()]);
        }
    }
    if let Some(fps) = args.fps {
        argv.extend(["-r", &fps.to_string()]);
    }
    argv.push(&args.output);
    let mut c = engine::write_job("transcode", &[&args.input], &args.output, vec![argv], g)?;
    c = c.with_extra(json!({ "preset": codec }));
    Ok(c)
}

/// Shared pre-Flash web-era codec path — cinepak (CD-ROM/Myst-era
/// archives) and svq1 (QuickTime 2-4 web video) both ride PCM audio and
/// accept rate flags; bitrate is meaningful here (unlike lossless).
fn qt_era(
    args: &TranscodeArgs,
    g: &Globals,
    codec: &str,
    exts: &[&str],
    pix_fmt: Option<&str>,
    acodec: Option<&str>,
) -> Result<Contract, Error> {
    let ext = args
        .output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if !exts.contains(&ext.as_str()) {
        return Err(Error::input(format!(
            "transcode --preset {codec} needs a {} target, not .{ext}",
            exts.iter()
                .map(|e| format!(".{e}"))
                .collect::<Vec<_>>()
                .join("/")
        )));
    }
    if (args.crf.is_some() && !matches!(codec, "libx264" | "libvpx-vp9"))
        || (args.abitrate.is_some() && !matches!(acodec, Some("libmp3lame") | Some("aac")))
    {
        return Err(Error::input(format!(
            "transcode --preset {codec} has no crf/abitrate knobs — use --vbitrate"
        )));
    }
    if acodec == Some("real_144") && (args.ar.is_some() || args.channels.is_some()) {
        return Err(Error::input(
            "RealAudio 1.0 is the 8kHz mono dial-up spec — drop --ar/--channels",
        ));
    }
    if acodec.is_none() && (args.ar.is_some() || args.channels.is_some()) {
        return Err(Error::input(format!(
            "transcode --preset {codec} is a video-only elementary stream — drop --ar/--channels"
        )));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_video {
        return Err(Error::input(format!("{codec} preset: input has no video")));
    }
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-map", "0:v?"]);
    if probe.has_audio && acodec.is_some() {
        argv.extend(["-map", "0:a?"]);
    }
    argv.extend(["-c:v", codec]);
    if let Some(p) = pix_fmt {
        argv.extend(["-pix_fmt", p]);
    }
    if let Some(b) = &args.vbitrate {
        argv.extend(["-b:v", b]);
    }
    if let Some(c) = args.crf {
        argv.extend(["-crf", &c.to_string()]);
        // vp9 CQ mode: -b:v 0 makes -crf the target quality, not a cap
        if codec == "libvpx-vp9" && args.vbitrate.is_none() {
            argv.extend(["-b:v", "0"]);
        }
    }
    if let Some(n) = args.gop {
        argv.extend(["-g", &n.to_string()]);
    }
    if probe.has_audio {
        if let Some(ac) = acodec {
            argv.extend(["-c:a", ac]);
            if ac == "libmp3lame" {
                argv.extend(["-b:a", abitrate(args, "192k")]);
            }
            if ac == "aac" && args.abitrate.is_some() {
                argv.extend(["-b:a", abitrate(args, "128k")]);
            }
            if ac == "real_144" {
                argv.extend(["-ar", "8000", "-ac", "1"]);
            }
            if let Some(r) = args.ar {
                argv.extend(["-ar", &r.to_string()]);
            }
            if let Some(ch) = args.channels {
                argv.extend(["-ac", &ch.to_string()]);
            }
        }
    }
    if let Some(fps) = args.fps {
        argv.extend(["-r", &fps.to_string()]);
    }
    argv.push(&args.output);
    let mut c = engine::write_job("transcode", &[&args.input], &args.output, vec![argv], g)?;
    c = c.with_extra(json!({ "preset": codec }));
    Ok(c)
}

/// id RoQ + RoQ DPCM in .roq — Quake III-era game video. RoQ needs
/// power-of-two picture dims (Quake refuses anything else) and its audio
/// is roq_dpcm pinned at 22050Hz, so the preset snaps each dimension to
/// the nearest power of two letterboxed and refuses every flag that would
/// break the spec (like dv/amv's fixed-spec snap).
fn roq(args: &TranscodeArgs, g: &Globals) -> Result<Contract, Error> {
    let ext = args
        .output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if ext != "roq" {
        return Err(Error::input(format!(
            "transcode --preset roq needs a .roq target, not .{ext}"
        )));
    }
    if args.gop.is_some()
        || args.range.is_some()
        || args.field_order.is_some()
        || args.interlace_mode.is_some()
        || args.interlaced
        || args.ar.is_some()
        || args.channels.is_some()
        || args.vbitrate.is_some()
        || args.abitrate.is_some()
        || args.crf.is_some()
        || args.width.is_some()
        || args.colors.is_some()
    {
        return Err(Error::input(
            "transcode --preset roq is a fixed spec (power-of-two dims yuvj444p + RoQ DPCM 22050Hz) — tuning flags don't apply",
        ));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    if !probe.has_video {
        return Err(Error::input("roq preset: input has no video"));
    }
    let pow2 = |n: u32| -> u32 {
        let n = n.max(1) as f64;
        (2f64.powf(n.log2().round()) as u32).clamp(16, 2048)
    };
    let w = pow2(probe.width.unwrap_or(256));
    let h = pow2(probe.height.unwrap_or(256));
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    argv.extend(["-map", "0:v?"]);
    if probe.has_audio {
        argv.extend(["-map", "0:a?"]);
    }
    argv.extend([
        "-vf",
        &format!("scale=w={w}:h={h}:force_original_aspect_ratio=decrease,pad={w}:{h}:(ow-iw)/2:(oh-ih)/2"),
    ]);
    argv.extend(["-c:v", "roqvideo", "-pix_fmt", "yuvj444p"]);
    if probe.has_audio {
        argv.extend(["-c:a", "roq_dpcm", "-ar", "22050"]);
    }
    if let Some(fps) = args.fps {
        argv.extend(["-r", &fps.to_string()]);
    }
    argv.push(&args.output);
    let mut c = engine::write_job("transcode", &[&args.input], &args.output, vec![argv], g)?;
    c = c.with_extra(json!({ "preset": "roq" }));
    Ok(c)
}

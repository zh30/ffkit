use serde_json::json;

use crate::cli::{DeliverArgs, DeliverPlatform, Globals, LogoPos};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::spawn::{self, Argv};
use crate::verbs::loudnorm;

const TARGET_I: f64 = -14.0;
const TARGET_TP: f64 = -1.5;
const TARGET_LRA: f64 = 11.0;
const PODCAST_I: f64 = -16.0;

pub fn run(args: DeliverArgs, g: &Globals) -> Result<Contract, Error> {
    if let Some(i) = args.lufs {
        if !(-70.0..=-5.0).contains(&i) {
            return Err(Error::input(
                "deliver --lufs must be -70..=-5 (e.g. -14, -16)",
            ));
        }
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    // --program: pick one service of a multi-program transport stream.
    // `0:p:N` maps the whole program, so scoped paths address members by
    // absolute index via probe.programs[] (see hls --program).
    let prog_members = match args.program {
        Some(0) => {
            return Err(Error::input(
                "deliver --program is 1-based (use probe.programs[] num)",
            ));
        }
        Some(_) if probe.programs.is_empty() => {
            return Err(Error::input("deliver --program: input carries no programs"));
        }
        Some(n) => match probe.program_members(n) {
            Some(m) => Some(m),
            None => {
                let avail: Vec<String> = probe.programs.iter().map(|p| p.num.to_string()).collect();
                return Err(Error::input(format!(
                    "deliver --program {n} not in input (programs: {})",
                    avail.join(",")
                )));
            }
        },
        None => None,
    };
    let sel_has_video = prog_members
        .as_ref()
        .map(|(v, _, _)| !v.is_empty())
        .unwrap_or(probe.has_video);
    let sel_has_audio = prog_members
        .as_ref()
        .map(|(_, a, _)| !a.is_empty())
        .unwrap_or(probe.has_audio);
    let first_v = prog_members
        .as_ref()
        .and_then(|(v, _, _)| v.first().copied());
    let first_a = prog_members
        .as_ref()
        .and_then(|(_, a, _)| a.first().copied());
    // Input-0 labels inside -filter_complex: `[0:v]`/`[0:a]` become the
    // service's member index when a program is picked.
    let vin = |i: u32| match (i, first_v) {
        (0, Some(idx)) => format!("0:{idx}"),
        _ => format!("{i}:v"),
    };
    let ain = |i: u32| match (i, first_a) {
        (0, Some(idx)) => format!("0:{idx}"),
        _ => format!("{i}:a"),
    };
    if args.logo.is_none() && (args.logo_position.is_some() || args.logo_opacity.is_some()) {
        return Err(Error::input("--logo-position/--logo-opacity need --logo"));
    }
    if let Some(op) = args.logo_opacity {
        if !(0.0..=1.0).contains(&op) {
            return Err(Error::input("--logo-opacity must be 0..=1"));
        }
    }
    let audio_pack = matches!(
        args.platform,
        DeliverPlatform::Podcast | DeliverPlatform::Audiobook
    );
    if args.cover.is_some() && !audio_pack {
        return Err(Error::input(
            "deliver --cover only applies to --platform podcast/audiobook (feed art)",
        ));
    }
    if args.chapters.is_some() && !audio_pack {
        return Err(Error::input(
            "deliver --chapters only applies to --platform podcast/audiobook",
        ));
    }
    if audio_pack {
        if args.logo.is_some() || args.intro.is_some() || args.outro.is_some() {
            return Err(Error::input(
                "deliver --logo/--intro/--outro need a video platform — audio packs have no picture",
            ));
        }
        return podcast(args, &probe, prog_members.as_ref(), g);
    }
    if args.to.is_some() {
        let scheme = args
            .to
            .as_deref()
            .unwrap_or("")
            .split("://")
            .next()
            .unwrap_or("")
            .to_lowercase();
        if !matches!(scheme.as_str(), "rtmp" | "rtmps" | "tcp" | "udp") {
            return Err(Error::input(
                "deliver --to needs an rtmp://, rtmps://, tcp://, or udp:// URL",
            ));
        }
    }
    if args.program.is_some() {
        if !sel_has_video {
            return Err(Error::input(format!(
                "deliver --program {} carries no video stream",
                args.program.unwrap_or(0)
            )));
        }
    } else {
        engine::need_video(&probe, "deliver")?;
    }
    let wrap = args.intro.is_some() || args.outro.is_some();
    if wrap {
        for clip in [args.intro.as_ref(), args.outro.as_ref()]
            .into_iter()
            .flatten()
        {
            crate::paths::ensure_input(clip)?;
            let cp = engine::probe_or_err(clip, g)?;
            engine::need_video(&cp, "deliver --intro/--outro")?;
            if sel_has_audio && !cp.has_audio {
                return Err(Error::input(
                    "deliver --intro/--outro clips need an audio track to match the main audio",
                ));
            }
        }
    }

    let target_i = args.lufs.unwrap_or(TARGET_I);
    let (fw, fh) = match args.platform {
        DeliverPlatform::Youtube
        | DeliverPlatform::Bilibili
        | DeliverPlatform::Linkedin
        | DeliverPlatform::Vimeo
        | DeliverPlatform::Bluesky
        | DeliverPlatform::Amazon
        | DeliverPlatform::Rumble
        | DeliverPlatform::Kick
        | DeliverPlatform::Vk
        | DeliverPlatform::Dailymotion
        | DeliverPlatform::Odysee
        | DeliverPlatform::Trovo
        | DeliverPlatform::Substack
        | DeliverPlatform::Niconico
        | DeliverPlatform::Soop
        | DeliverPlatform::Xigua
        | DeliverPlatform::Peertube
        | DeliverPlatform::Floatplane
        | DeliverPlatform::Nebula
        | DeliverPlatform::Chzzk
        | DeliverPlatform::Douyu
        | DeliverPlatform::Huya
        | DeliverPlatform::Udemy
        | DeliverPlatform::Coursera
        | DeliverPlatform::Teachable
        | DeliverPlatform::Kajabi
        | DeliverPlatform::Patreon
        | DeliverPlatform::Skillshare
        | DeliverPlatform::Thinkific
        | DeliverPlatform::Podia
        | DeliverPlatform::Learnworlds
        | DeliverPlatform::Gumroad
        | DeliverPlatform::Wistia
        | DeliverPlatform::Domestika
        | DeliverPlatform::Steam
        | DeliverPlatform::Itch
        | DeliverPlatform::Shopee
        | DeliverPlatform::Lazada
        | DeliverPlatform::Taobao
        | DeliverPlatform::Dlive
        | DeliverPlatform::Minds
        | DeliverPlatform::Telegram
        | DeliverPlatform::Tidal
        | DeliverPlatform::Deezer
        | DeliverPlatform::Qobuz
        | DeliverPlatform::Yandexmusic
        | DeliverPlatform::Napster
        | DeliverPlatform::Joox
        | DeliverPlatform::Soundcloud
        | DeliverPlatform::Mixcloud
        | DeliverPlatform::Audiomack
        | DeliverPlatform::Bandcamp
        | DeliverPlatform::Vevo
        | DeliverPlatform::Roku
        | DeliverPlatform::Plex
        | DeliverPlatform::Iqiyi
        | DeliverPlatform::Youku
        | DeliverPlatform::Wetv
        | DeliverPlatform::Viki
        | DeliverPlatform::Crunchyroll
        | DeliverPlatform::Funimation
        | DeliverPlatform::Mgtv
        | DeliverPlatform::Bigo
        | DeliverPlatform::Nimo
        | DeliverPlatform::Tumblr
        | DeliverPlatform::Dribbble
        | DeliverPlatform::Behance
        | DeliverPlatform::Flickr
        | DeliverPlatform::Zhihu
        | DeliverPlatform::Kakao
        | DeliverPlatform::Naver
        | DeliverPlatform::Coub
        | DeliverPlatform::Imgur
        | DeliverPlatform::NineGag
        | DeliverPlatform::Streamable
        | DeliverPlatform::Viddsee
        | DeliverPlatform::Rutube
        | DeliverPlatform::Ok
        | DeliverPlatform::Zen
        | DeliverPlatform::Openrec
        | DeliverPlatform::Twitcasting
        | DeliverPlatform::Showroom
        | DeliverPlatform::Fc2
        | DeliverPlatform::Tving
        | DeliverPlatform::Wavve
        | DeliverPlatform::Watcha
        | DeliverPlatform::Vidio
        | DeliverPlatform::Mewatch
        | DeliverPlatform::Tver
        | DeliverPlatform::Abema
        | DeliverPlatform::Spotify
        | DeliverPlatform::Apple
        | DeliverPlatform::Amazonmusic
        | DeliverPlatform::Iheartradio
        | DeliverPlatform::Pandora
        | DeliverPlatform::Castbox
        | DeliverPlatform::Podbean
        | DeliverPlatform::Hotstar
        | DeliverPlatform::Jiotv
        | DeliverPlatform::Sonyliv
        | DeliverPlatform::Mxplayer
        | DeliverPlatform::Zee5
        | DeliverPlatform::Showmax
        | DeliverPlatform::Shahid
        | DeliverPlatform::Tubi
        | DeliverPlatform::Pluto
        | DeliverPlatform::Dazn
        | DeliverPlatform::Espn
        | DeliverPlatform::Hulu
        | DeliverPlatform::UNext
        | DeliverPlatform::Gyao
        | DeliverPlatform::Netflix
        | DeliverPlatform::Disney
        | DeliverPlatform::Max
        | DeliverPlatform::Peacock
        | DeliverPlatform::Paramount
        | DeliverPlatform::Appletv
        | DeliverPlatform::Primevideo
        | DeliverPlatform::Globoplay
        | DeliverPlatform::Viaplay
        | DeliverPlatform::Joyn
        | DeliverPlatform::Raiplay
        | DeliverPlatform::Atresplayer
        | DeliverPlatform::Itvx
        | DeliverPlatform::Crave
        | DeliverPlatform::Stan
        | DeliverPlatform::Mycanal
        | DeliverPlatform::Skygo
        | DeliverPlatform::Movistar
        | DeliverPlatform::Viu
        | DeliverPlatform::Voot
        | DeliverPlatform::Clarovideo
        | DeliverPlatform::Nhk
        | DeliverPlatform::Arte
        | DeliverPlatform::Tv2play
        | DeliverPlatform::Npostart
        | DeliverPlatform::Rtve
        | DeliverPlatform::Tvp
        | DeliverPlatform::Voyo
        | DeliverPlatform::Wakanim
        | DeliverPlatform::Adn
        | DeliverPlatform::Laftel
        | DeliverPlatform::Aniplus
        | DeliverPlatform::Hidive
        | DeliverPlatform::Retrocrush
        | DeliverPlatform::Bstation
        | DeliverPlatform::Ard
        | DeliverPlatform::Zdf
        | DeliverPlatform::Nrk
        | DeliverPlatform::Svt
        | DeliverPlatform::Dr
        | DeliverPlatform::Cbc
        | DeliverPlatform::Sbs
        | DeliverPlatform::Tf1
        | DeliverPlatform::Francetv
        | DeliverPlatform::Mediaset
        | DeliverPlatform::Channel4
        | DeliverPlatform::Tenplay
        | DeliverPlatform::Nowtv
        | DeliverPlatform::Srf
        | DeliverPlatform::Fubo
        | DeliverPlatform::Sling
        | DeliverPlatform::Philo
        | DeliverPlatform::Directv
        | DeliverPlatform::Xumo
        | DeliverPlatform::Vidgo
        | DeliverPlatform::Frndly
        | DeliverPlatform::Iplayer
        | DeliverPlatform::My5
        | DeliverPlatform::Britbox
        | DeliverPlatform::Acorntv
        | DeliverPlatform::Shudder
        | DeliverPlatform::Showtime
        | DeliverPlatform::Starz
        | DeliverPlatform::Reddit
        | DeliverPlatform::Zillow
        | DeliverPlatform::Ebay
        | DeliverPlatform::Walmart
        | DeliverPlatform::Kanopy
        | DeliverPlatform::Mubi
        | DeliverPlatform::Criterion
        | DeliverPlatform::Curiositystream
        | DeliverPlatform::Magellantv
        | DeliverPlatform::Brightcove
        | DeliverPlatform::Jwplayer
        | DeliverPlatform::Kaltura
        | DeliverPlatform::Sproutvideo
        | DeliverPlatform::Vidyard
        | DeliverPlatform::Uscreen
        | DeliverPlatform::Vdocipher
        | DeliverPlatform::Jellyfin
        | DeliverPlatform::Emby
        | DeliverPlatform::Kofi
        | DeliverPlatform::Buymeacoffee
        | DeliverPlatform::Buzzsprout
        | DeliverPlatform::Captivate
        | DeliverPlatform::Transistor
        | DeliverPlatform::Redcircle
        | DeliverPlatform::Sounder
        | DeliverPlatform::Acast
        | DeliverPlatform::Spreaker
        | DeliverPlatform::Bandlab
        | DeliverPlatform::Distrokid
        | DeliverPlatform::Tunecore
        | DeliverPlatform::Amuse
        | DeliverPlatform::Cdbaby
        | DeliverPlatform::Symphonic
        | DeliverPlatform::Landr
        | DeliverPlatform::Blim
        | DeliverPlatform::Vix
        | DeliverPlatform::Irokotv
        | DeliverPlatform::Starzplay
        | DeliverPlatform::Pearvideo
        | DeliverPlatform::Haokan
        | DeliverPlatform::Miaopai
        | DeliverPlatform::Acfun
        | DeliverPlatform::Toutiao
        | DeliverPlatform::Baijiahao
        | DeliverPlatform::Migu
        | DeliverPlatform::Pptv
        | DeliverPlatform::Letv
        | DeliverPlatform::Kocowa
        | DeliverPlatform::Rakuentv
        | DeliverPlatform::Iwanttfc
        | DeliverPlatform::Hoichoi
        | DeliverPlatform::Ifeng
        | DeliverPlatform::Truthsocial
        | DeliverPlatform::Gettr
        | DeliverPlatform::Parler
        | DeliverPlatform::Locals
        | DeliverPlatform::Utreon
        | DeliverPlatform::Caffeine
        | DeliverPlatform::Qq
        | DeliverPlatform::Twitch => (1920, 1080),
        DeliverPlatform::X | DeliverPlatform::Mastodon | DeliverPlatform::Discord => (1280, 720),
        DeliverPlatform::Threads | DeliverPlatform::Instagram | DeliverPlatform::Facebook => {
            (1080, 1350)
        }
        DeliverPlatform::Square
        | DeliverPlatform::Shopify
        | DeliverPlatform::Etsy
        | DeliverPlatform::Poshmark => (1080, 1080),
        DeliverPlatform::Xhs | DeliverPlatform::Lemon8 => (1080, 1440),
        DeliverPlatform::Wechat => (1080, 1260),
        DeliverPlatform::Pinterest => (1000, 1500),
        DeliverPlatform::Circle => (640, 640),
        DeliverPlatform::Canvas
        | DeliverPlatform::Snapchat
        | DeliverPlatform::Whatsapp
        | DeliverPlatform::Line
        | DeliverPlatform::Triller
        | DeliverPlatform::Likee
        | DeliverPlatform::Moj
        | DeliverPlatform::Sharechat
        | DeliverPlatform::Chingari
        | DeliverPlatform::Weishi
        | DeliverPlatform::Huoshan
        | DeliverPlatform::Quanmin
        | DeliverPlatform::Meipai
        | DeliverPlatform::Pdd
        | DeliverPlatform::Jd
        | DeliverPlatform::Vip
        | DeliverPlatform::Vmate
        | DeliverPlatform::Josh
        | DeliverPlatform::Weverse
        | DeliverPlatform::Kwai
        | DeliverPlatform::Snackvideo
        | DeliverPlatform::Whatnot
        | DeliverPlatform::Tinder
        | DeliverPlatform::Bumble
        | DeliverPlatform::Hinge
        | DeliverPlatform::Mercari
        | DeliverPlatform::Vinted
        | DeliverPlatform::Depop
        | DeliverPlatform::Carousell
        | DeliverPlatform::Olx
        | DeliverPlatform::Onlyfans
        | DeliverPlatform::Fansly
        | DeliverPlatform::Fanbox
        | DeliverPlatform::Cameo
        | DeliverPlatform::Subscribestar => (1080, 1920),
        DeliverPlatform::Weibo => (1920, 1080),
        _ => (1080, 1920),
    };
    let mut vf = format!(
        "scale={fw}:{fh}:force_original_aspect_ratio=decrease,pad={fw}:{fh}:(ow-iw)/2:(oh-ih)/2:black,setsar=1,fps={fps}",
        fps = args.fps.unwrap_or(30)
    );
    let mut subs_part: Option<String> = None;
    if let Some(subs) = &args.subs {
        // The subtitles filter parses `:` `'` `,` in filenames — escape them.
        let path = subs
            .canonicalize()
            .map_err(|e| Error::input(format!("{subs:?}: {e}")))?
            .to_string_lossy()
            .replace('\\', "\\\\")
            .replace('\'', "\\'");
        subs_part = Some(format!(",subtitles=filename='{path}'"));
        vf.push_str(subs_part.as_deref().unwrap());
    }
    vf.push_str(",format=yuv420p");
    let platform = platform_name(args.platform);

    let mut apply = ffmpeg_base(g.progress);
    apply.push("-i");
    apply.push(&args.input);
    let mut ni = 1u32;
    let intro_i = if let Some(p) = &args.intro {
        apply.push("-i");
        apply.push(p);
        let i = ni;
        ni += 1;
        Some(i)
    } else {
        None
    };
    let outro_i = if let Some(p) = &args.outro {
        apply.push("-i");
        apply.push(p);
        let i = ni;
        ni += 1;
        Some(i)
    } else {
        None
    };
    let logo_i = if let Some(logo) = &args.logo {
        crate::paths::ensure_input(logo)?;
        apply.push("-i");
        apply.push(logo);
        let i = ni;
        Some(i)
    } else {
        None
    };
    let logo_fg = |li: u32| -> String {
        let lw = (fw * 18 / 100).max(16);
        let m = (fh * 3 / 100).max(8);
        let (x, y) = match args.logo_position.unwrap_or(LogoPos::Br) {
            LogoPos::Tl => (format!("{m}"), format!("{m}")),
            LogoPos::Tr => (format!("W-w-{m}"), format!("{m}")),
            LogoPos::Bl => (format!("{m}"), format!("H-h-{m}")),
            LogoPos::Br => (format!("W-w-{m}"), format!("H-h-{m}")),
        };
        let mut lg = format!("[{li}:v]scale={lw}:-1");
        if let Some(op) = args.logo_opacity {
            lg.push_str(&format!(",format=rgba,colorchannelmixer=aa={op}"));
        }
        lg.push_str(&format!("[lg];[vc][lg]overlay={x}:{y}[vo]"));
        lg
    };
    // wrap: fc held aside so loudnorm folds into the graph — -af conflicts
    // with a complex-feed stream.
    let mut wrap_fc: Option<String> = None;
    let mut wrap_vout = "vc";
    if wrap {
        // Brand wrap: every segment normalized to the platform canvas, then
        // concat — subs/logo ride the whole deliverable (logo on top of all
        // three, captions on the main segment only).
        let segs: Vec<u32> = intro_i.into_iter().chain([0]).chain(outro_i).collect();
        let mut fc = String::new();
        for (k, i) in segs.iter().enumerate() {
            let mut chain = format!(
                "scale={fw}:{fh}:force_original_aspect_ratio=decrease,pad={fw}:{fh}:(ow-iw)/2:(oh-ih)/2:black,setsar=1,fps={fps}",
                fps = args.fps.unwrap_or(30)
            );
            if *i == 0 {
                if let Some(sp) = &subs_part {
                    chain.push_str(sp);
                }
            }
            chain.push_str(",format=yuv420p");
            fc.push_str(&format!("[{}]{chain}[v{k}];", vin(*i)));
            if sel_has_audio {
                fc.push_str(&format!(
                    "[{}]aresample=48000,aformat=channel_layouts=stereo[a{k}];",
                    ain(*i)
                ));
            }
        }
        let ins: String = (0..segs.len())
            .map(|k| {
                if sel_has_audio {
                    format!("[v{k}][a{k}]")
                } else {
                    format!("[v{k}]")
                }
            })
            .collect();
        if sel_has_audio {
            fc.push_str(&format!("{ins}concat=n={}:v=1:a=1[vc][ac];", segs.len()));
        } else {
            fc.push_str(&format!("{ins}concat=n={}:v=1:a=0[vc];", segs.len()));
        }
        if let Some(li) = logo_i {
            fc.push_str(&logo_fg(li));
            wrap_vout = "vo";
        }
        wrap_fc = Some(fc);
    } else if let Some(li) = logo_i {
        let lw = (fw * 18 / 100).max(16);
        let m = (fh * 3 / 100).max(8);
        let (x, y) = match args.logo_position.unwrap_or(LogoPos::Br) {
            LogoPos::Tl => (format!("{m}"), format!("{m}")),
            LogoPos::Tr => (format!("W-w-{m}"), format!("{m}")),
            LogoPos::Bl => (format!("{m}"), format!("H-h-{m}")),
            LogoPos::Br => (format!("W-w-{m}"), format!("H-h-{m}")),
        };
        let mut lg = format!("scale={lw}:-1");
        if let Some(op) = args.logo_opacity {
            lg.push_str(&format!(",format=rgba,colorchannelmixer=aa={op}"));
        }
        let fc = format!(
            "[{}]{vf}[base];[{li}:v]{lg}[lg];[base][lg]overlay={x}:{y}[vout]",
            vin(0)
        );
        apply.extend(["-filter_complex", &fc]);
        apply.extend(["-map", "[vout]"]);
        if sel_has_audio {
            let amap = first_a
                .map(|i| format!("0:{i}"))
                .unwrap_or_else(|| "0:a:0".to_string());
            apply.extend(["-map", &amap]);
        }
    } else {
        apply.extend(["-vf", &vf]);
        if let Some(vidx) = first_v {
            apply.extend(["-map", &format!("0:{vidx}")]);
            if let Some(aidx) = first_a {
                apply.extend(["-map", &format!("0:{aidx}")]);
            }
        }
    }
    apply.extend([
        "-c:v",
        "libx264",
        "-preset",
        "medium",
        "-crf",
        &args.crf.unwrap_or(20).clamp(0, 51).to_string(),
        "-pix_fmt",
        "yuv420p",
    ]);
    if let Some(m) = &args.maxrate {
        apply.extend(["-maxrate:v", m]);
        let buf = args.bufsize.clone().unwrap_or_else(|| double_rate(m));
        apply.extend(["-bufsize:v", &buf]);
    } else if args.bufsize.is_some() {
        return Err(Error::input(
            "deliver --bufsize pairs with --maxrate (a buffer alone isn't a rate cap)",
        ));
    }
    if args.to.is_none() {
        apply.extend(["-movflags", "+faststart"]);
    }
    push_metadata(&mut apply, &args);

    let mut measure: Option<Argv> = None;
    let mut measured: Option<serde_json::Value> = None;
    if sel_has_audio {
        let filter = loudnorm::measure_filter(target_i, TARGET_TP, TARGET_LRA);
        let mut m = Argv::ffmpeg();
        m.extend(["-nostats", "-i"]);
        m.push(&args.input);
        if let Some(aidx) = first_a {
            m.extend(["-map", &format!("0:{aidx}")]);
        }
        m.extend(["-af", &filter, "-vn", "-f", "null", "-"]);
        if !g.dry_run {
            crate::paths::ensure_output_allowed(&args.output, &[&args.input], g.overwrite)?;
            let spawned = spawn::run(&m, g.timeout, false)?;
            let spawned = spawn::require_ok(&m, spawned)?;
            let meas = loudnorm::parse_measured(&spawn::stderr_str(&spawned))?;
            let ln = loudnorm::apply_filter(target_i, TARGET_TP, TARGET_LRA, &meas, false);
            if let Some(fc) = &mut wrap_fc {
                let fcs = fc.trim_end_matches(';').to_string();
                *fc = format!("{fcs};[ac]{ln}[aout]");
            } else {
                apply.extend(["-af", &ln]);
            }
            measured = Some(meas);
        } else if let Some(fc) = &mut wrap_fc {
            let fcs = fc.trim_end_matches(';').to_string();
            *fc = format!("{fcs};[ac]{filter}[aout]");
        } else {
            apply.extend(["-af", &filter]);
        }
        apply.extend(["-c:a", "aac", "-ar", "48000", "-b:a", "192k"]);
        // Telegram circles ship mono voice; --channels still wins when given
        let ch_n = args
            .channels
            .or(if args.platform == DeliverPlatform::Circle {
                Some(1)
            } else {
                None
            });
        if let Some(ch) = ch_n {
            apply.extend(["-ac", &ch.to_string()]);
        }
        measure = Some(m);
    } else {
        apply.push("-an");
    }
    if let Some(fc) = wrap_fc {
        apply.extend(["-filter_complex", fc.trim_end_matches(';')]);
        apply.extend(["-map", &format!("[{wrap_vout}]")]);
        if sel_has_audio {
            apply.extend(["-map", "[aout]"]);
        }
    }
    // --to: rendered pack pushed straight to ingest — drop faststart (no
    // moov in FLV), add zerolatency, emit -f flv instead of a file
    let to = args.to.clone();
    if let Some(t) = args.preview {
        if !t.is_finite() || t <= 0.0 {
            return Err(Error::input("deliver --preview needs a positive duration"));
        }
        apply.extend(["-t", &t.to_string()]);
    }
    if to.is_none() {
        apply.push(&args.output);
    }

    if let Some(url) = &to {
        apply.extend(["-tune", "zerolatency", "-f", "flv"]);
        apply.push(url);
        let mut c =
            crate::verbs::live::stream_out("deliver", Some(&args.input), url, vec![apply], g)?;
        if let Some(m) = measure {
            let mut commands = engine::commands_of(&[m]);
            commands.extend(c.commands.clone());
            c.commands = commands;
        }
        return Ok(finish(
            c,
            platform,
            (fw, fh),
            measured,
            args.fps.unwrap_or(30),
            &args,
        ));
    }

    let mut argvs = Vec::new();
    if let Some(m) = measure {
        if g.dry_run {
            argvs.push(m);
        } else {
            // measure already ran; keep it in the contract command list
            let mut c = engine::write_job("deliver", &[&args.input], &args.output, vec![apply], g)?;
            let mut commands = engine::commands_of(&[m]);
            commands.extend(c.commands.clone());
            c.commands = commands;
            return Ok(finish(
                c,
                platform,
                (fw, fh),
                measured,
                args.fps.unwrap_or(30),
                &args,
            ));
        }
    }

    argvs.push(apply);
    let c = engine::write_job("deliver", &[&args.input], &args.output, argvs, g)?;
    Ok(finish(
        c,
        platform,
        (fw, fh),
        measured,
        args.fps.unwrap_or(30),
        &args,
    ))
}

fn finish(
    c: Contract,
    platform: &str,
    frame: (u32, u32),
    measured: Option<serde_json::Value>,
    fps: u32,
    args: &DeliverArgs,
) -> Contract {
    c.with_extra(json!({
        "platform": platform,
        "frame": format!("{}x{}", frame.0, frame.1),
        "fps": fps,
        "target_i": args.lufs.unwrap_or(TARGET_I),
        "target_tp": TARGET_TP,
        "measured": measured,
        "intro": args.intro.is_some(),
        "outro": args.outro.is_some(),
        "logo": args.logo.is_some(),
    }))
}

fn platform_name(p: DeliverPlatform) -> &'static str {
    match p {
        DeliverPlatform::Social => "social",
        DeliverPlatform::Reels => "reels",
        DeliverPlatform::Tiktok => "tiktok",
        DeliverPlatform::Shorts => "shorts",
        DeliverPlatform::Square => "square",
        DeliverPlatform::Youtube => "youtube",
        DeliverPlatform::Xhs => "xhs",
        DeliverPlatform::Wechat => "wechat",
        DeliverPlatform::Podcast => "podcast",
        DeliverPlatform::Audiobook => "audiobook",
        DeliverPlatform::Douyin => "douyin",
        DeliverPlatform::Kuaishou => "kuaishou",
        DeliverPlatform::Bilibili => "bilibili",
        DeliverPlatform::Pinterest => "pinterest",
        DeliverPlatform::X => "x",
        DeliverPlatform::Linkedin => "linkedin",
        DeliverPlatform::Vimeo => "vimeo",
        DeliverPlatform::Bluesky => "bluesky",
        DeliverPlatform::Threads => "threads",
        DeliverPlatform::Instagram => "instagram",
        DeliverPlatform::Facebook => "facebook",
        DeliverPlatform::Mastodon => "mastodon",
        DeliverPlatform::Circle => "circle",
        DeliverPlatform::Canvas => "canvas",
        DeliverPlatform::Snapchat => "snapchat",
        DeliverPlatform::Weibo => "weibo",
        DeliverPlatform::Whatsapp => "whatsapp",
        DeliverPlatform::Twitch => "twitch",
        DeliverPlatform::Discord => "discord",
        DeliverPlatform::Shopify => "shopify",
        DeliverPlatform::Amazon => "amazon",
        DeliverPlatform::Etsy => "etsy",
        DeliverPlatform::Rumble => "rumble",
        DeliverPlatform::Kick => "kick",
        DeliverPlatform::Line => "line",
        DeliverPlatform::Vk => "vk",
        DeliverPlatform::Dailymotion => "dailymotion",
        DeliverPlatform::Odysee => "odysee",
        DeliverPlatform::Trovo => "trovo",
        DeliverPlatform::Substack => "substack",
        DeliverPlatform::Triller => "triller",
        DeliverPlatform::Lemon8 => "lemon8",
        DeliverPlatform::Niconico => "niconico",
        DeliverPlatform::Soop => "soop",
        DeliverPlatform::Xigua => "xigua",
        DeliverPlatform::Likee => "likee",
        DeliverPlatform::Moj => "moj",
        DeliverPlatform::Josh => "josh",
        DeliverPlatform::Peertube => "peertube",
        DeliverPlatform::Floatplane => "floatplane",
        DeliverPlatform::Nebula => "nebula",
        DeliverPlatform::Chzzk => "chzzk",
        DeliverPlatform::Douyu => "douyu",
        DeliverPlatform::Huya => "huya",
        DeliverPlatform::Weverse => "weverse",
        DeliverPlatform::Kwai => "kwai",
        DeliverPlatform::Snackvideo => "snackvideo",
        DeliverPlatform::Udemy => "udemy",
        DeliverPlatform::Coursera => "coursera",
        DeliverPlatform::Teachable => "teachable",
        DeliverPlatform::Kajabi => "kajabi",
        DeliverPlatform::Patreon => "patreon",
        DeliverPlatform::Skillshare => "skillshare",
        DeliverPlatform::Thinkific => "thinkific",
        DeliverPlatform::Podia => "podia",
        DeliverPlatform::Learnworlds => "learnworlds",
        DeliverPlatform::Gumroad => "gumroad",
        DeliverPlatform::Wistia => "wistia",
        DeliverPlatform::Domestika => "domestika",
        DeliverPlatform::Steam => "steam",
        DeliverPlatform::Itch => "itch",
        DeliverPlatform::Shopee => "shopee",
        DeliverPlatform::Lazada => "lazada",
        DeliverPlatform::Taobao => "taobao",
        DeliverPlatform::Dlive => "dlive",
        DeliverPlatform::Minds => "minds",
        DeliverPlatform::Telegram => "telegram",
        DeliverPlatform::Tidal => "tidal",
        DeliverPlatform::Deezer => "deezer",
        DeliverPlatform::Qobuz => "qobuz",
        DeliverPlatform::Yandexmusic => "yandexmusic",
        DeliverPlatform::Napster => "napster",
        DeliverPlatform::Joox => "joox",
        DeliverPlatform::Soundcloud => "soundcloud",
        DeliverPlatform::Mixcloud => "mixcloud",
        DeliverPlatform::Audiomack => "audiomack",
        DeliverPlatform::Bandcamp => "bandcamp",
        DeliverPlatform::Vevo => "vevo",
        DeliverPlatform::Roku => "roku",
        DeliverPlatform::Plex => "plex",
        DeliverPlatform::Iqiyi => "iqiyi",
        DeliverPlatform::Youku => "youku",
        DeliverPlatform::Wetv => "wetv",
        DeliverPlatform::Viki => "viki",
        DeliverPlatform::Crunchyroll => "crunchyroll",
        DeliverPlatform::Funimation => "funimation",
        DeliverPlatform::Mgtv => "mgtv",
        DeliverPlatform::Bigo => "bigo",
        DeliverPlatform::Nimo => "nimo",
        DeliverPlatform::Tumblr => "tumblr",
        DeliverPlatform::Dribbble => "dribbble",
        DeliverPlatform::Behance => "behance",
        DeliverPlatform::Flickr => "flickr",
        DeliverPlatform::Zhihu => "zhihu",
        DeliverPlatform::Kakao => "kakao",
        DeliverPlatform::Naver => "naver",
        DeliverPlatform::Coub => "coub",
        DeliverPlatform::Imgur => "imgur",
        DeliverPlatform::NineGag => "9gag",
        DeliverPlatform::Streamable => "streamable",
        DeliverPlatform::Viddsee => "viddsee",
        DeliverPlatform::Rutube => "rutube",
        DeliverPlatform::Ok => "ok",
        DeliverPlatform::Zen => "zen",
        DeliverPlatform::Openrec => "openrec",
        DeliverPlatform::Twitcasting => "twitcasting",
        DeliverPlatform::Showroom => "showroom",
        DeliverPlatform::Fc2 => "fc2",
        DeliverPlatform::Tving => "tving",
        DeliverPlatform::Wavve => "wavve",
        DeliverPlatform::Watcha => "watcha",
        DeliverPlatform::Vidio => "vidio",
        DeliverPlatform::Mewatch => "mewatch",
        DeliverPlatform::Tver => "tver",
        DeliverPlatform::Abema => "abema",
        DeliverPlatform::Spotify => "spotify",
        DeliverPlatform::Apple => "apple",
        DeliverPlatform::Amazonmusic => "amazonmusic",
        DeliverPlatform::Iheartradio => "iheartradio",
        DeliverPlatform::Pandora => "pandora",
        DeliverPlatform::Castbox => "castbox",
        DeliverPlatform::Podbean => "podbean",
        DeliverPlatform::Hotstar => "hotstar",
        DeliverPlatform::Jiotv => "jiotv",
        DeliverPlatform::Sonyliv => "sonyliv",
        DeliverPlatform::Mxplayer => "mxplayer",
        DeliverPlatform::Zee5 => "zee5",
        DeliverPlatform::Showmax => "showmax",
        DeliverPlatform::Shahid => "shahid",
        DeliverPlatform::Tubi => "tubi",
        DeliverPlatform::Pluto => "pluto",
        DeliverPlatform::Dazn => "dazn",
        DeliverPlatform::Espn => "espn",
        DeliverPlatform::Hulu => "hulu",
        DeliverPlatform::UNext => "u-next",
        DeliverPlatform::Gyao => "gyao",
        DeliverPlatform::Netflix => "netflix",
        DeliverPlatform::Disney => "disney",
        DeliverPlatform::Max => "max",
        DeliverPlatform::Peacock => "peacock",
        DeliverPlatform::Paramount => "paramount",
        DeliverPlatform::Appletv => "appletv",
        DeliverPlatform::Primevideo => "primevideo",
        DeliverPlatform::Globoplay => "globoplay",
        DeliverPlatform::Viaplay => "viaplay",
        DeliverPlatform::Joyn => "joyn",
        DeliverPlatform::Raiplay => "raiplay",
        DeliverPlatform::Atresplayer => "atresplayer",
        DeliverPlatform::Itvx => "itvx",
        DeliverPlatform::Crave => "crave",
        DeliverPlatform::Stan => "stan",
        DeliverPlatform::Mycanal => "mycanal",
        DeliverPlatform::Skygo => "skygo",
        DeliverPlatform::Movistar => "movistar",
        DeliverPlatform::Viu => "viu",
        DeliverPlatform::Voot => "voot",
        DeliverPlatform::Clarovideo => "clarovideo",
        DeliverPlatform::Nhk => "nhk",
        DeliverPlatform::Arte => "arte",
        DeliverPlatform::Tv2play => "tv2play",
        DeliverPlatform::Npostart => "npostart",
        DeliverPlatform::Rtve => "rtve",
        DeliverPlatform::Tvp => "tvp",
        DeliverPlatform::Voyo => "voyo",
        DeliverPlatform::Wakanim => "wakanim",
        DeliverPlatform::Adn => "adn",
        DeliverPlatform::Laftel => "laftel",
        DeliverPlatform::Aniplus => "aniplus",
        DeliverPlatform::Hidive => "hidive",
        DeliverPlatform::Retrocrush => "retrocrush",
        DeliverPlatform::Bstation => "bstation",
        DeliverPlatform::Ard => "ard",
        DeliverPlatform::Zdf => "zdf",
        DeliverPlatform::Nrk => "nrk",
        DeliverPlatform::Svt => "svt",
        DeliverPlatform::Dr => "dr",
        DeliverPlatform::Cbc => "cbc",
        DeliverPlatform::Sbs => "sbs",
        DeliverPlatform::Tf1 => "tf1",
        DeliverPlatform::Francetv => "francetv",
        DeliverPlatform::Mediaset => "mediaset",
        DeliverPlatform::Channel4 => "channel4",
        DeliverPlatform::Tenplay => "tenplay",
        DeliverPlatform::Nowtv => "nowtv",
        DeliverPlatform::Srf => "srf",
        DeliverPlatform::Fubo => "fubo",
        DeliverPlatform::Sling => "sling",
        DeliverPlatform::Philo => "philo",
        DeliverPlatform::Directv => "directv",
        DeliverPlatform::Xumo => "xumo",
        DeliverPlatform::Vidgo => "vidgo",
        DeliverPlatform::Frndly => "frndly",
        DeliverPlatform::Iplayer => "iplayer",
        DeliverPlatform::My5 => "my5",
        DeliverPlatform::Britbox => "britbox",
        DeliverPlatform::Acorntv => "acorntv",
        DeliverPlatform::Shudder => "shudder",
        DeliverPlatform::Showtime => "showtime",
        DeliverPlatform::Starz => "starz",
        DeliverPlatform::Reddit => "reddit",
        DeliverPlatform::Zillow => "zillow",
        DeliverPlatform::Ebay => "ebay",
        DeliverPlatform::Walmart => "walmart",
        DeliverPlatform::Poshmark => "poshmark",
        DeliverPlatform::Whatnot => "whatnot",
        DeliverPlatform::Kanopy => "kanopy",
        DeliverPlatform::Mubi => "mubi",
        DeliverPlatform::Criterion => "criterion",
        DeliverPlatform::Curiositystream => "curiositystream",
        DeliverPlatform::Magellantv => "magellantv",
        DeliverPlatform::Tinder => "tinder",
        DeliverPlatform::Bumble => "bumble",
        DeliverPlatform::Hinge => "hinge",
        DeliverPlatform::Brightcove => "brightcove",
        DeliverPlatform::Jwplayer => "jwplayer",
        DeliverPlatform::Kaltura => "kaltura",
        DeliverPlatform::Sproutvideo => "sproutvideo",
        DeliverPlatform::Vidyard => "vidyard",
        DeliverPlatform::Uscreen => "uscreen",
        DeliverPlatform::Vdocipher => "vdocipher",
        DeliverPlatform::Mercari => "mercari",
        DeliverPlatform::Vinted => "vinted",
        DeliverPlatform::Depop => "depop",
        DeliverPlatform::Carousell => "carousell",
        DeliverPlatform::Olx => "olx",
        DeliverPlatform::Jellyfin => "jellyfin",
        DeliverPlatform::Emby => "emby",
        DeliverPlatform::Onlyfans => "onlyfans",
        DeliverPlatform::Fansly => "fansly",
        DeliverPlatform::Fanbox => "fanbox",
        DeliverPlatform::Cameo => "cameo",
        DeliverPlatform::Subscribestar => "subscribestar",
        DeliverPlatform::Kofi => "kofi",
        DeliverPlatform::Buymeacoffee => "buymeacoffee",
        DeliverPlatform::Buzzsprout => "buzzsprout",
        DeliverPlatform::Captivate => "captivate",
        DeliverPlatform::Transistor => "transistor",
        DeliverPlatform::Redcircle => "redcircle",
        DeliverPlatform::Sounder => "sounder",
        DeliverPlatform::Acast => "acast",
        DeliverPlatform::Spreaker => "spreaker",
        DeliverPlatform::Bandlab => "bandlab",
        DeliverPlatform::Distrokid => "distrokid",
        DeliverPlatform::Tunecore => "tunecore",
        DeliverPlatform::Amuse => "amuse",
        DeliverPlatform::Cdbaby => "cdbaby",
        DeliverPlatform::Symphonic => "symphonic",
        DeliverPlatform::Landr => "landr",
        DeliverPlatform::Sharechat => "sharechat",
        DeliverPlatform::Chingari => "chingari",
        DeliverPlatform::Vmate => "vmate",
        DeliverPlatform::Blim => "blim",
        DeliverPlatform::Vix => "vix",
        DeliverPlatform::Irokotv => "irokotv",
        DeliverPlatform::Starzplay => "starzplay",
        DeliverPlatform::Pearvideo => "pearvideo",
        DeliverPlatform::Haokan => "haokan",
        DeliverPlatform::Miaopai => "miaopai",
        DeliverPlatform::Acfun => "acfun",
        DeliverPlatform::Toutiao => "toutiao",
        DeliverPlatform::Baijiahao => "baijiahao",
        DeliverPlatform::Ifeng => "ifeng",
        DeliverPlatform::Weishi => "weishi",
        DeliverPlatform::Huoshan => "huoshan",
        DeliverPlatform::Quanmin => "quanmin",
        DeliverPlatform::Meipai => "meipai",
        DeliverPlatform::Migu => "migu",
        DeliverPlatform::Pptv => "pptv",
        DeliverPlatform::Letv => "letv",
        DeliverPlatform::Pdd => "pdd",
        DeliverPlatform::Jd => "jd",
        DeliverPlatform::Vip => "vip",
        DeliverPlatform::Kocowa => "kocowa",
        DeliverPlatform::Rakuentv => "rakuentv",
        DeliverPlatform::Iwanttfc => "iwanttfc",
        DeliverPlatform::Hoichoi => "hoichoi",
        DeliverPlatform::Truthsocial => "truthsocial",
        DeliverPlatform::Gettr => "gettr",
        DeliverPlatform::Parler => "parler",
        DeliverPlatform::Locals => "locals",
        DeliverPlatform::Utreon => "utreon",
        DeliverPlatform::Caffeine => "caffeine",
        DeliverPlatform::Qq => "qq",
    }
}

/// Apple-style container tags — title/author(album artist)/album/genre/
/// comment land on every platform's output.
/// CBR pairing: `--bufsize` defaults to 2x `--maxrate` — parse "4500k"/
/// "6M"/"2000000" and double the numeric part, preserving the suffix.
fn double_rate(rate: &str) -> String {
    let (num, suffix) = match rate.strip_suffix(|c| matches!(c, 'k' | 'K' | 'M')) {
        Some(n) => (n, &rate[rate.len() - 1..]),
        None => (rate, ""),
    };
    match num.parse::<u64>() {
        Ok(n) => format!("{}{}", n * 2, suffix),
        Err(_) => rate.to_string(),
    }
}

fn push_metadata(apply: &mut Argv, args: &DeliverArgs) {
    for (k, v) in [
        ("title", &args.title),
        ("artist", &args.author),
        ("album", &args.album),
        ("genre", &args.genre),
        ("comment", &args.comment),
    ] {
        if let Some(v) = v {
            if !v.trim().is_empty() {
                apply.extend(["-metadata", &format!("{k}={v}")]);
            }
        }
    }
}

/// Audio-only feed pack: loudnorm to the podcast spec (−16 LUFS) → m4a AAC.
/// Accepts audio-only inputs — the video platforms require a video track.
fn podcast(
    args: DeliverArgs,
    probe: &crate::probe::Probe,
    prog_members: Option<&(Vec<u32>, Vec<u32>, Vec<u32>)>,
    g: &Globals,
) -> Result<Contract, Error> {
    let prog_audio = prog_members.and_then(|(_, a, _)| a.first().copied());
    let has_audio = prog_members
        .map(|(_, a, _)| !a.is_empty())
        .unwrap_or(probe.has_audio);
    if !has_audio {
        return Err(Error::input(
            "deliver --platform podcast needs an audio stream",
        ));
    }
    let podcast_i = args.lufs.unwrap_or(PODCAST_I);
    let mut apply = ffmpeg_base(g.progress);
    apply.push("-i");
    apply.push(&args.input);
    let mut ni = 1u32;
    if let Some(cover) = &args.cover {
        crate::paths::ensure_input(cover)?;
        apply.push("-i");
        apply.push(cover);
        let amap = prog_audio
            .map(|i| format!("0:{i}"))
            .unwrap_or_else(|| "0:a".to_string());
        apply.extend(["-map", &amap, "-map", "1:v"]);
        ni += 1;
    } else if let Some(aidx) = prog_audio {
        apply.extend(["-map", &format!("0:{aidx}")]);
    } else {
        apply.push("-vn");
    }
    // --chapters: YouTube-format marks ("mm:ss title", the file
    // `chapter --yt` writes) become real container chapters — Apple
    // Podcasts/Apple Books turn them into seek stops.
    let mut chap_marks: Vec<(f64, String)> = Vec::new();
    let mut chap_file: Option<std::path::PathBuf> = None;
    if let Some(cf) = &args.chapters {
        crate::paths::ensure_input(cf)?;
        let marks = crate::verbs::chapter::parse_yt_list(cf)?;
        crate::verbs::chapter::check_marks(&marks, probe.duration, "deliver --chapters")?;
        let meta = crate::verbs::chapter::ffmeta_table(&marks, probe.duration);
        let tmp =
            std::env::temp_dir().join(format!("ffkit-deliver-chap-{}.ffmeta", std::process::id()));
        std::fs::write(&tmp, meta).map_err(|e| Error::output(format!("writing chapters: {e}")))?;
        chap_marks = marks;
        apply.extend(["-f", "ffmetadata", "-i"]);
        apply.push(&tmp);
        apply.extend(["-map_metadata", &ni.to_string()]);
        apply.extend(["-map_chapters", &ni.to_string()]);
        chap_file = Some(tmp);
    }
    let mut m = Argv::ffmpeg();
    m.extend(["-nostats", "-i"]);
    m.push(&args.input);
    if let Some(aidx) = prog_audio {
        m.extend(["-map", &format!("0:{aidx}")]);
    }
    m.extend([
        "-af",
        &loudnorm::measure_filter(podcast_i, TARGET_TP, TARGET_LRA),
        "-vn",
        "-f",
        "null",
        "-",
    ]);
    let mut measured: Option<serde_json::Value> = None;
    if !g.dry_run {
        crate::paths::ensure_output_allowed(&args.output, &[&args.input], g.overwrite)?;
        let spawned = spawn::run(&m, g.timeout, false)?;
        let spawned = spawn::require_ok(&m, spawned)?;
        let meas = loudnorm::parse_measured(&spawn::stderr_str(&spawned))?;
        apply.extend([
            "-af",
            &loudnorm::apply_filter(podcast_i, TARGET_TP, TARGET_LRA, &meas, false),
        ]);
        measured = Some(meas);
    } else {
        apply.extend([
            "-af",
            &loudnorm::measure_filter(podcast_i, TARGET_TP, TARGET_LRA),
        ]);
    }
    let ab = if matches!(args.platform, DeliverPlatform::Audiobook) {
        "96k"
    } else {
        "128k"
    };
    apply.extend(["-c:a", "aac", "-ar", "48000", "-b:a", ab]);
    if args.cover.is_some() || matches!(args.platform, DeliverPlatform::Audiobook) {
        // .m4a/.m4b resolve to the ipod muxer, which rejects video streams
        // in ffmpeg 4.x — force mp4 (same container) so mjpeg attaches and
        // the chapter table writes.
        if args.cover.is_some() {
            apply.extend(["-c:v", "mjpeg", "-disposition:v:1", "attached_pic"]);
        }
        apply.extend(["-f", "mp4"]);
    }
    push_metadata(&mut apply, &args);
    if let Some(ch) = args.channels {
        apply.extend(["-ac", &ch.to_string()]);
    }
    if let Some(t) = args.preview {
        if !t.is_finite() || t <= 0.0 {
            return Err(Error::input("deliver --preview needs a positive duration"));
        }
        apply.extend(["-t", &t.to_string()]);
    }
    apply.push(&args.output);

    let m_commands = engine::commands_of(std::slice::from_ref(&m));
    let mut argvs = Vec::new();
    if g.dry_run {
        argvs.push(m);
    }
    argvs.push(apply);
    let run = engine::write_job("deliver", &[&args.input], &args.output, argvs, g);
    if let Some(tmp) = &chap_file {
        let _ = std::fs::remove_file(tmp);
    }
    let mut c = run?;
    if !g.dry_run {
        let mut commands = m_commands;
        commands.extend(c.commands.clone());
        c.commands = commands;
    }
    let platform = platform_name(args.platform);
    Ok(c.with_extra(json!({
        "platform": platform,
        "target_i": podcast_i,
        "target_tp": TARGET_TP,
        "measured": measured,
        "cover": args.cover.is_some(),
        "chapters": chap_marks.len(),
    })))
}

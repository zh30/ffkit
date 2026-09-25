---
name: ffkit
description: Help a user finish a local video or audio job. Chat about the outcome, propose a short plan, then run that plan with ffkit (pipeline of verbs, graph, or ffmpeg). Use when they mention a media file (mp4, mov, mkv, webm, wav, m4a, mp3, gif), footage, clip, Reel/Short/TikTok/YouTube, captions (mux or burn without libass), overlay, transcode, ffmpeg, rough cut, assembly, or an edit, export, or effect on files they have on disk. Requires ffmpeg, ffprobe, and ffkit on PATH matching this skill's version field.

version: 0.325.0



compatibility: Requires ffmpeg, ffprobe, and the ffkit binary on PATH.
---

# ffkit

The user talks to you. You propose a scheme. Then you use these hands to finish **that original task**.

`ffkit` is the hands. Flags: `ffkit <verb> --help`. Numbers: `--json` / `--json-brief`. Shared: `--dry-run`, `--json`, `--overwrite`, `--timeout`, `--progress`.

If `ffkit` is missing, install the GitHub Release zip that matches this skill's `version:` (not Source code), then `./install.sh`. If `ffkit version --check` fails, `ffkit install-skill` and reload. After a missing-tool failure, `ffkit doctor --json`.

## Workflow

1. **Chat** until you can restate the outcome in one sentence in their language (what they will post or keep). The original task is the **last agreed** outcome, not the first message.
2. **Propose a plan** of 3–7 steps in their language (not flag soup). If they asked to review first, stop here. If they change the plan, execute the new one.
3. **Probe** each input (`ffkit probe FILE --json`). Numbers in the plan come from that. A long take they want roughed: `ffkit rough FILE --json` first (speech islands, no write), then assemble with `-o` after they agree.
4. **Run the plan.** Two or more writing steps: pipeline JSON then `ffkit pipeline plan.json --json` — [pipeline.md](references/pipeline.md). `goal` is the restated outcome; `expect` is the numbers that make it true; `$src` is `input`, `$in` is the previous output. Adapt a recipe only after they agreed to that scheme — [recipes.md](references/recipes.md). One writing step: that verb. No matching verb: `graph`, then `ffmpeg --because`.
5. Prefer **lossless** (`cut` copy, mux captions, `loop` concat) unless pixels or samples must change.
6. Picture changed: `ffkit look`. Overlay/`caption`/`title`: `--at` a time the graphic is on. `broll`: `--at` the cutaway and a time after it. Fade: `--at 0` vs mid. Cover: inspect the PNG.
7. Never write onto the user's source.

Done when the **original task** is true (`expect` / probe match, and look if the picture changed)—not when the last process exited 0. If `verified` is false, the file exists; revise the plan.

Ask one question only when it changes the file and probe cannot answer it. Which cut is interesting, and what looks cinematic, stay with the user.

## Hands

`ffkit --help` is the full list. Match an agreed plan step to a tool. Compose several rows when the job needs several steps.

| Plan step | Hands |
|-----------|-------|
| inspect | `doctor`, `probe`, `look` (`--tiles` / `--at`) |

| trim / join | `cut` (`--black` excises blackdetect dead-air stretches ≥0.3s), `concat` (`--transition` any xfade — all 42 named 4.4 transitions (comma list picks one per joint), N clips, `--level -14` loudnorms each, `--gap N` black+silence between clips, `--audio-fade N` boundary fades at joints, `--list manifest.txt` reads clip paths from a file, `--chapters` titles each clip as a container chapter at its join — audiobook/podcast assembly (relative to the list's dir)), `split` (`--every` story chunks, `--at` chapter points, `--chapters` embedded marks, `--black` dead-air → one part per keep, `--copy` lossless stream-copy split — instant, boundaries snap to next keyframe), `rough` (list speech islands, then `-o` to assemble, `--merge N` merge close keeps, `--by-scene` split at cuts) |
| platform loudness | `loudnorm` (`--target`, `-I/--tp/--lra`; `--measure` report-only, `--gate N` fails when input tops N LUFS, `--dynamic` per-frame) |
| frame / size | `fit` (`--fit pad` / `crop` / `blur --strength`), `zoom`, `--position` top/bottom/corners |

| export | `deliver` (`--platform youtube|square|xhs|wechat|douyin|kuaishou|bilibili|pinterest|x|linkedin|vimeo|bluesky|threads|mastodon|circle|canvas|snapchat|weibo|whatsapp|twitch|discord|shopify|amazon|etsy|rumble|instagram|facebook|kick|line|vk|dailymotion|odysee|trovo|substack|triller|lemon8|niconico|soop|xigua|peertube|floatplane|nebula|chzzk|douyu|huya|likee|moj|josh|weverse|kwai|snackvideo|udemy|coursera|teachable|kajabi|patreon|skillshare|thinkific|podia|learnworlds|gumroad|wistia|domestika|steam|itch|shopee|lazada|taobao|dlive|minds|telegram|tidal|deezer|qobuz|yandexmusic|napster|joox|soundcloud|mixcloud|audiomack|bandcamp|vevo|roku|plex|iqiyi|youku|wetv|viki|crunchyroll|funimation|mgtv|bigo|nimo|tumblr|dribbble|behance|flickr|zhihu|kakao|naver|coub|imgur|9gag|streamable|viddsee|rutube|ok|zen|openrec|twitcasting|showroom|fc2|tving|wavve|watcha|vidio|mewatch|tver|abema|hotstar|jiotv|sonyliv|mxplayer|zee5|showmax|shahid|truthsocial|gettr|parler|locals|utreon|caffeine|qq|tubi|pluto|dazn|espn|hulu|u-next|gyao|netflix|disney|max|peacock|paramount|appletv|primevideo|globoplay|viaplay|joyn|raiplay|atresplayer|itvx|crave|spotify|apple|amazonmusic|iheartradio|pandora|castbox|podbean|stan|mycanal|skygo|movistar|viu|voot|clarovideo|nhk|arte|tv2play|npostart|rtve|tvp|voyo|wakanim|adn|laftel|aniplus|hidive|retrocrush|bstation|ard|zdf|nrk|svt|dr|cbc|sbs|tf1|francetv|mediaset|channel4|tenplay|nowtv|srf|fubo|sling|philo|directv|xumo|vidgo|frndly|iplayer|my5|britbox|acorntv|shudder|showtime|starz|podcast|audiobook`), `transcode` (`--copy-audio`, `--copy-video` audio-only re-encode, `--vbitrate` peak cap, `--abitrate` audio bitrate, `--preset mp3`/`aac`/`wav`/`flac`/`opus` audio-only), `compress` (`--size 10MB` two-pass, `--crf` quality one-pass, `--res` downscale), `audiogram` (`--progress`, `--mode`/`--color`), `slideshow` (`--motion kenburns`, `--transition`, `--list manifest.txt` curated order), `split`, `--subs` captions, `--preset prores`/`dnxhd`, `--target` platform sizes |
| captions / mute | `caption` (`--karaoke` word reveal (`--highlight` sung color), `--box-color` card, `--mode burn` social safe-zone, `--chunk N` word groups, `--align`, `--from/--to` window, `--opacity` ghost, or `--mode mux`), `--fade` |

| hook text | `title` (`--wrap` auto line breaks, `--align` left/right lower-thirds, `--box`/`--outline`/`--shadow`, `--margin` px corner inset) |
| dutch-angle tilt (full clip or windowed) | `rotate` (`--angle 15`, `--at`/`--dur` window — comma list for several tilts) |
| chapters already in the file | `chapter` (`--list`) or `split` (`--chapters`) |
| thumbnail candidates | `thumb` (`--scenes` grabs stills at every cut) |
| spectrum-bar audiogram | `audiogram` (`--mode spectrum`, `--fscale`/`--fps`); `--mode hist` = amplitude-histogram video (clipping QC) |
| lissajous vectorscope video | `audiogram` (`--mode scope` — trippy stereo scope) |
| news-ticker crawl | `scroll` (`--mode ticker`, `--bg` opaque bar, `--speed` px/s) |
| AV1 delivery | `transcode` (`--preset av1`) |
| podcast/voice → mp3/m4a/wav/flac/opus | `transcode` (`--preset mp3`/`aac`/`wav`/`flac`/`opus` — `-vn` audio-only) |
| podcast feed pack (−16 LUFS spec) | `deliver --platform podcast` (m4a AAC 128k/48k, loudnorm to feed spec; works on audio-only sources, `--cover art.png` embeds the Apple/Spotify cover) or `--platform audiobook` (m4b AAC 96k — Apple Books/Audible); audio packs take `--chapters marks.txt` (real container chapters from the YouTube-format `mm:ss title` list `chapter --yt` exports) and feed tags `--title/--author/--album/--genre/--comment`; any platform: `--preview SEC` renders just the pack's head for approval QC, `--logo mark.png` burns a corner watermark during the pack render (`--logo-position tl/tr/bl/br`, `--logo-opacity`), `--intro/--outro clip.mp4` bakes a channel bumper and CTA card onto every export (normalized to the platform canvas); Chinese feeds: `--platform douyin|kuaishou` (9:16 1080x1920) / `bilibili` (16:9 1920x1080) |
| resample/force channels on export | `transcode --ar 48000 --channels 1|2` (broadcast 48k stereo, podcast mono — re-encode only, `--copy-audio` skips) |
| audiogram of just the best bit(s) | `audiogram` (`--from/--to` one segment; `--at a,b --dur 30` = one clip per point → `stem_N.mp4`) |
| cover still | `cover` (`--blur` ambient pad, `--size` canvas, comma `--at` = one cover per time) |

| speech / music | `jumpcut`, `denoise`, `music`, `replace` (`--loop` short beds, `--audio` swap the track, `--at`/`--dur` windowed swap (comma `--at` lays the new track across several windows), `--mix` keep the original under it, `--video` swap the picture and keep the audio), `loudnorm` (`--target` platform preset), `volume` |
| grainy low-light footage | `vdenoise` (`--strength`, `--engine` nlmeans/hqdn3d/atadenoise/vaguedenoise/bm3d/dctdnoiz/owdenoise/median/chroma/rg/fftdnoiz/dotcrawl/edge) |
| blocky re-uploaded/screen-rec footage | `deblock` (`--strength` 0.05-0.95, `--at` window) |
| colored halo on tape/capture | `chromashift` (`--x`/`--y` px, `--edge` wrap/smear) |
| moving people/cars on a tripod shot, rain streaks | `tmedian` (temporal median; `--radius` frames of history, `--percentile`, `--at` window; drops 2*radius edge frames) |
| tourists/noise across 3+ copies of the same locked shot | `stack` `in1 in2 in3 ...` (median across inputs — each takes the object in a different spot; `--percentile`) — `--mode mean` exposure averaging (`--weights` per-input, comma list) |
| waveform PNG of audio | `waveform` (`--size`, `--color`, `--scale`, `--bg` card, `--at/--dur`, comma `--at` = one PNG per window, `--vertical` = wave runs top→bottom) — podcast art, thumbnails |
| audio spectrogram PNG | `spectrogram` (`--size`, `--color`, `--separate` per-channel, `--at/--dur`, comma `--at` = one PNG per window) — inspect hum/noise before cleanup |
| watch loudness while it plays | `meter` (`--size`, `--meter 9|18`, `--at/--dur` — EBU R128 video; podcast/voice QC) |
| mains hum / electrical buzz | `dehum` (`--at`/`--dur` window, `--mains 50|60` or `--freq HZ` custom hum, `--harmonics`) — notches the fundamental + harmonics |
| faster/slower podcast | `tempo` (`--factor 1.5` — pitch held; video inputs: use `speed`), `--at/--dur` window |
| voice all over the place | `leveler` (`--at`/`--dur` window, `--threshold`/`--ratio`/`--makeup` — `acompressor`; `--engine speechnorm` lifts quiet speech too, `--engine limit` alimiter brickwall ceiling) |
| hiss between sentences | `gate` (`--threshold`, `--preset`, `--at/--dur` — `agate` closes on quiet parts) |
| pad in room tone / breath | `silence` (`--at`, comma list pads several points, `--dur` inserts quiet; `--detect` reports ranges; video holds: `freeze`) |
| one-click look | `grade --preset cinematic|vivid|vintage|soft|sepia|teal|noir|bleach|neon` (stacks under the sliders) |
| warm faces only | `grade --skin -1..1` (selectivecolor reds channel — warms skin, leaves the rest) |
| HALD image LUT | `grade --lut look.png` (PNG/JPG → haldclut; Darktable/RawTherapee exports) |
| karaoke / keep only the vocal | `vocal` (`--at`/`--dur` window, `--mode karaoke` drops the center, `isolate` keeps it — stereo only; `--amount` partial) |
| container swap, no re-encode | `remux` (mkv→mp4 etc., `-c copy` + faststart); `--audio` rips the track, `--video` video-only, `--aspect 16:9` fixes display AR; `--itsscale R` retimes the container without re-encoding (PAL↔film pull-down), `--offset SEC` sets the container start_time (repairs negative/odd capture starts); `--frag` fragmented MP4 (moof/mfra — playable while still being written, HLS/DASH pipelines); `--no-subs` drops subtitle/data streams in the repack (clean deliverable); `--from/--to` lossless trim (keyframe-accurate, no re-encode); `--lang jpn` keeps only audio tagged with that language (multi-language releases, dub extraction; with `--audio` rips just that track); `--default-audio N` makes track N the player default; `--cover pic` attaches feed art as an attached_pic stream (audio rips get cover art, videos get a thumbnail poster); `--chapters marks.txt` embeds container chapters from a YouTube-format list (the same file `chapter --yt` exports — Apple Podcasts/Books seek stops); `--title/--artist/--album/--genre/--comment/--date` writes library tags on the repack, `--lang` takes a comma list (`eng,jpn` keeps both dubs), `--strip-meta` wipes every inherited container tag (privacy — combine with tags to retag in one pass), `--no-cover` drops attached_pic cover-art streams, `--encrypt` CENC AES-CTR on the repack (mp4/mov only — ClearKey/Widevine DRM prep; `--key`/`--kid` 32-hex or auto-random, echoed in JSON), `--audio-delay SEC` shifts the audio track against the video — lip-sync repair without re-encoding (negative advances audio), `--video-delay SEC` shifts the video track instead (capture cards that lag the picture; mutually exclusive), `--tag hvc1` codec-tag retag for Apple/Safari HEVC playback, `--attach f` embeds attachment streams into mkv/webm (subtitle fonts ship inside the file), `--audio-order 1,0` keeps + reorders audio tracks (unlisted drop — put the program mix on track 0 for dumb players), `--sub-lang fra` keeps only subtitles tagged with that language (multi-subtitle releases — comma list keeps several), `--sub-order 1,0` keeps + reorders subtitle tracks by index (audience captions first on multi-sub releases — unlisted tracks drop)), `--keep 0,3` keeps ONLY the listed absolute stream indices (the escape hatch when the per-type orders can't express the pick — unlisted streams drop, `probe.streams` lists indices), `--decrypt HEX` reads CENC-encrypted sources (-decryption_key — 32-hex key, round-trips with `--encrypt` for key rotation), `--copy-ts` preserves input timestamps verbatim (-copyts — capture pipelines that need wall-clock pts kept, conflicts with the ts mutators), `--video-order 1,0` keeps + reorders video tracks by per-type index (multi-angle files — hero angle first, unlisted drop), `--forced-sub N` flags subtitle track N FORCED so players auto-show it for the audience's language (film-style forced captions — mkv/webm only, mp4 can't express it), `--sdh N`/`--forced-sub N` flag subtitle track N as SDH/forced captions, `--commentary N`/`--audio-desc N`/`--dub N`/`--original N` flag audio track N as commentary/audio-description/dub/original-language (all mkv/webm only — mp4 drops the flags silently, so ffkit refuses rather than no-ops), `--default-video N` picks the default video track on multi-angle/multi-cam files (`--default-audio`/`--default-sub` for the other kinds), `--no-video` drops the video streams but keeps audio/subs/cover/attachments (audio deliverable with its artwork intact — unlike --audio which rips only the track), --no-audio drops the audio streams but keeps picture/subs/cover (muted B-roll/screencast deliverable — unlike --video which rips only the track), --no-attachments drops embedded font/payload streams (--no-cover only removes attached_pic cover art), `--drop 1,3` drops ONLY the listed absolute stream indices (inverse of --keep — pull one commentary track / one language out, keep the rest), `--no-chapters` strips embedded container chapters in the repack (clean audio deliverable — players that show a broken TOC), `--genpts` regenerates missing/broken timestamps on the way in (-fflags +genpts — camera/truncated files that seek badly or probe at zero duration; conflicts with --copy-ts)) |
| top/bottom caption meme | `meme` (`--top`/`--bottom` text, `--color`, `--size`, `--outline`, `--at/--dur` window — `--at end` covers the tail), `--position` center/bottom, `--wrap` + `--align` multiline, `--fade` edge fades (needs --at/--dur), `--opacity` ghost text |
| fix my podcast voice | `voice` — one-shot chain: gate hiss → compress swings → loudnorm `--lufs` (default −16); `--at`/`--dur` windows it |
| slideshow that runs exactly N seconds | `slideshow` (`--dur` spreads the runtime across the stills, `--bg` letterbox color); `--fit` ends the montage exactly when the `--audio` bed ends; `--shuffle SEED` rerolls the photo order deterministically (same seed = same order, photo-dump montages), `--sort name|mtime` orders a camera dump by filename or shoot time, `--titles a,,c` burns a bottom caption strip on each still (comma slots in the final slide order — an empty entry skips that slide), `--audio-fade SEC` sets the music-bed tail fade (default 0.8 — longer lets the song ring out under the last still), `--audio-offset SEC` starts the bed T seconds in — skip the intro, use the chorus (--fit measures the remainder), `--audio-loop` repeats a short bed across the whole montage (short jingle under a long slideshow — refused with --fit), `--audio-fade-in SEC` eases the bed head in (pairs with --audio-fade's tail) |
| old interlaced footage | `deinterlace` (`--mode field` doubles the rate, `frame` same rate, `--parity` field order, `--engine` yadif/bwdif/estdif/kerndeint/w3fdif/mcdeint/fieldmatch/detelecine/`separate`/`pullup`/`phase` field reorder/`field` half-height quick preview) |
| fade to white | `fade --color white` (`--in`/`--out` seconds as usual) |
| blend two audio files | `crossfade` (`--second`, `--dur` overlap — acrossfade) |
| strip location/device tags | `strip` — drops all container metadata + chapters, stream copy |
| stills every N seconds | `frames` (`--every`, `--nth N` every-Nth-frame, `--number N` exact frame index (0-based — pinpoint a bad frame where --at's time math drifts on VFR; comma list `0,5,12` grabs several), `--width`) → `stem_001.png…` (`--untile 4x3` splits every frame into tiles — reverse a contact sheet) |
| 3-2-1 intro countdown | `countdown` (`--from` up to 600, `--each`, `--go`, `--at`, `--text`, `--position`, `--bg` plate, `--beep` + `--tone` Hz, `--format` mm:ss/h:mm:ss long counts, `--opacity` ghost, `--target HH:MM` counts to a wall-clock premiere time — 1s per count, up to 10 min ahead, rolls to tomorrow once passed) |
| invert / negative look | `invert` — `negate` the picture |
| split to fit a size cap | `split --size 9MB` — even grid aimed at Discord/WhatsApp caps |
| merge two audio sources at full level | `mix` `A B` (`--vol-a/--vol-b`, `--longest`, `--at/--dur`), `--duck` bed dips under voice (`--gate` hard-mutes it — talk-show bed), `--normalize` halves the sum |
| captions on top instead of bottom | `caption --position top` |
| lift/crush mid-tones | `grade --gamma` |
| drop the audio track entirely | `mute` (stream-copy video, no re-encode), `--at/--dur` window |
| elapsed-time corner counter | `timer`/`countdown` (`--at` takes `end`) (`--box-color` card, `--position`, `--at`, `--dur`, `--size`, `--color`, `--format ms` centiseconds), `--down` countdown, `--start` seed, `--opacity` ghost HUD; `timer --tc 01:00:00:00` burns a running HH:MM:SS:FF timecode (dailies/review copies); `timer --clock` burns the local wall clock HH:MM:SS (event/sports overlays); `timer --date` burns the local calendar YYYY-MM-DD (air-date/archive overlay — prefixes --clock when both set); `--utc` reads the clock/date in UTC instead of local (broadcast logs, cross-timezone crews); `countdown --utc` counts to a UTC wall-clock target (premieres on a shared schedule) |
| web-embed HLS package | `hls` (`--seg` seconds, `--single` one-file, `--copy` repack, `--poster` writes poster.jpg, `--poster-at T` picks the frame, `--encrypt`/`--key HEX`/`--key-uri URI` AES-128 segments + key.bin/key.info) → dir/`index.m3u8` + `seg_*.ts`; `--ladder 1080,720,480` → ABR variant playlists + `master.m3u8`; `--audio-only` podcast HLS; `--video-only` muted/preview renditions; `--fmp4` CMAF `.m4s` segments; `--live` sliding-window playlist (players join mid-write — newest `--live-window N` segments only, no endlist); `--start N` resumes segment numbering after a restart, `--epoch` seeds it from the epoch clock (24/7 channels), `--date` stamps EXT-X-PROGRAM-DATE-TIME on segments, `--discontinuity` marks a feed restart, `--time-names` names segments by wall-clock time (archive the recording by time), `--independent` tags EXT-X-INDEPENDENT-SEGMENTS + forces a keyframe per segment, `--iframes` tags EXT-X-I-FRAMES-ONLY (trick-play scrub previews), `--base-url URL` prefixes every playlist segment entry — serve segments from a CDN while the manifest stays local); `dash` DASH-pack → dir/manifest.mpd + init-/seg-*.m4s (`--seg`, `--copy` repack, `--single` one byte-range file per representation, `--webm` vp9+opus, `--window N` sliding live manifest, `--ladder 1080,720,480` → N video Representations at tiered bitrates in one AdaptationSet — adaptive DASH; `--streaming` writes a moof fragment per frame (low-latency DASH prep), `--frag SEC` writes a moof fragment per SEC inside each segment (trick-play prep — between per-frame and whole segments), `--sidx` writes a global SIDX box into the `--single` byte-range file (HTTP range seeking)) |
| go live / push a stream | `live` (`--to rtmp://…` / `rtmps://` / `tcp://` / `udp://` / `srt://` (SRT/UDP ride MPEG-TS), `--codec hevc` contribution-grade HEVC on TS transports, `--subs file.srt` burns live captions in, `--loop` forever, `--vbitrate`/`--abitrate`, `--volume 0..4` scales pushed-audio gain (quiet a loud BGM source without re-rendering it), `--crf 0-51` constant-quality instead of -b:v (conflicts with --vbitrate/--maxrate/--bufsize), `--scale WxH` downscale a big master to ingest size, `--fps N` cap output rate, `--record file.mp4` archive the stream locally while pushing (tee — encode once, mux twice), `--until SEC` auto-stop the stream (premiere windows), `--list` reads a concat manifest for 24/7 rotation (`--loop` = infinite), `--test` streams a generated test card + tone (verify the stream key before showtime), `--slate card.png --slate-dur SEC` holds a starting-soon card ahead of the feed (premieres), `--card art.png` gives an audio-only source a persistent still video (24/7 lofi-radio streams), `--overlay bug.png` pins a channel bug in a corner (`--overlay-position tl|tr|bl|br`, `--overlay-opacity`), `--restream url` pushes to a second ingest at once — multistream in one encode, `--gop N` keyframe interval for ingest specs (YouTube wants ≤2s), `--preset` x264 speed/quality, `--audio-only` drops the video path — audio podcast/radio push from any source (aac-only feed), `--no-audio` drops the audio path — silent feeds, `--vertical` letterboxes onto a 1080x1920 canvas — TikTok/Reels live, `--maxrate 4500k`/`--bufsize` CBR caps for ingest rate specs — bufsize defaults to 2x maxrate, `--start T` begins streaming T seconds into the source — skip a long event's dead head on replay, `--title "Show"` writes the show name into the FLV/TS metadata + --record archive (ingest dashboards display it), `--channels 1` mono push (speech/radio ingest specs — refuses --no-audio), `--audio-delay SEC` delays the pushed audio (adelay — capture cards whose sound leads the picture; refuses --no-audio), `--hold SEC` freezes the first frame + mutes the audio head (tpad clone + adelay — ingest warmup; refuses --slate/--card), `--loudnorm` dynamic loudness normalize on the pushed aac chain (broadcast −23 LUFS spec), `--rw-timeout SEC` aborts the push when the ingest stalls (socket stall watchdog — distinct from the global `--timeout` kill timer; single-destination pushes only), URL input relays a live source — `live rtmp://…/udp://…/http://… --to …` re-streams a pull feed to another ingest (file-only flags --list/--start/--loop/--slate refuse)); real-time `-re` pacing + x264/aac ingest encode); `deliver --to` streams the rendered platform pack to the same ingest URLs) |

| check encode quality loss | `qa` `ref.mp4 test.mp4` → psnr/ssim/msad/vif numbers (`--metric`) |
| normalize mixed footage for concat | `conform` (`--size WxH`, `--fps`, `--lufs`, `--pad` letterbox color + `--anchor`, `--blur` blurred fill, `--even` floors odd px dims — phone captures can't take yuv420p, `--ar HZ` resample rate (48k broadcast default; 44100 podcast/CD, 96000 masters), `--channels 1` mono masters) |
| light-leak / screen-blend overlay | `overlay --video leak.mp4 --mode screen` (31 blend modes — Photoshop-style burn/dodge/softlight/hardlight/vividlight/linearlight/pinlight/hardmix/exclusion/negation/subtract/divide/glow/phoenix/reflect plus grainmerge/grainextract film-grain composites, the 128-neutral variants multiply128/addition128/difference128, and/or/xor mask logic, average/extremity/freeze/heat…) |
| fix audio/video sync drift | `sync` (`--ms ±N` — pad or trim audio start) |
| sync a second take to the camera master | `align` (`ref target -o out` — auto-detects offset by audio cross-correlation; multi-cam, external recorder; `--window` bounds long takes; `--check` reports `offset_ms`/`direction` without rendering — sync QC) |
| rolling end credits | `scroll` (`--text`/`--file`, `--at` comma list replays the roll at several marks / `end` with `--dur`, `--align`, `--wrap`, `--speed` px/s, `--opacity` ghost credits — text rolls bottom→top) |
| splice a clip into the middle | `insert` (`--clip x.mp4 --at T`, comma list splices at several points, `end` appends — b-roll/ad read without manual split+concat; `--dur N` first N sec only), `--transition` xfade both joints, `--volume` clip audio, `--replace` overwrite the span under the clip (patch a flub — output keeps base duration), `--at chapterN` splices at embedded chapter N (patch a section in a chaptered file) |
| two-camera angle switching | `multicam` (`A B --at t1,t2,...` (`end` ok) — `--align` auto-syncs B to A by audio xcorr first (needs in-sync audio on both, broadband like speech/noise — pure tones don't correlate); `--keep-audio` stays on cam A, `--transition` soft cuts) |
| reframe 9:16 keeping faces | `crop` (`--aspect 9:16 --anchor top` keeps the face) |
| attach album cover art | `art` (`--image cover.png`) → mp3/m4a/mp4/mkv, `--extract` pull cover out |
| grab a cover/thumbnail frame | `thumb` / `extract` / `cover` (`--at`, comma `--at` = one still per time; `thumb --frame`, `--count N` even spreads, `--from end-N` tail window) → jpg/png — comma `extract --gif --at` = one GIF per beat |
| burn an .srt/.ass into pixels | `subs` (`--burn subs.srt` — libass), `--box` plate, `--shadow` depth, `--margin` px, `--rate` drift fix, `--from/--to` cue window, `--safe` social zone; `--convert` srt↔vtt (+ `.ass` both directions — Style/Format columns honoured) ; `--burn-si N` burns embedded track N; `--encoding gbk` legacy charset; `--sort` reorders out-of-order cues, `--fix-overlaps` clamps each cue's end to the next start (overlap repair — pairs with --sort when times are scrambled), `--dedupe` drops exact repeats — .srt cue-file hygiene for broken exports; `--min-dur SEC` extends flash-text cues up to SEC (caps at the next cue's start), `--min-gap SEC` trims the earlier cue's tail when the gap before the next cue is shorter (broadcast spec ~2 frames), `--join SEC` merges adjacent cues closer than SEC into one (auto-transcript over-fragmentation repair — Whisper-style fragments join into sentence-length cues); `--cps N` caption-speed gate (`over_limit`/`worst_cps` — Netflix-style readability spec), `--max-lines N` reports cues with more than N text lines (broadcast spec is 2); `--replace OLD,NEW` find/replaces cue text (rename a character across the file), `--strip-speakers` drops `[NAME]`/`<NAME>`/`ALL-CAPS:` speaker labels (auto-transcripts), `--wrap N` rewraps cue lines at N chars (portrait phones); `--append b.srt` joins a second subtitle file where the first file's cues end (post-`concat` transcripts), `--convert` to `.txt` exports a plain-text transcript, to `.ass` writes minimal styled ASS (Aegisub/anime-pipeline handoff), to `.lrc` writes synced lyrics (transcript → karaoke/music-player lyrics — the next line's timestamp acts as each line's end), to `.ttml`/`.dfxp` writes minimal TTML/DFXP (broadcast/Netflix subtitle exchange — `.ttml`/`.dfxp` input parses `<p begin end>` cues back too), `.sbv` reads/writes YouTube SubViewer captions (`H:MM:SS.mmm,H:MM:SS.mmm` headers — Studio-editable uploads round-trip); `--split 3,8` cuts an .srt at comma timestamps into `stem_0.srt`/`stem_1.srt`… parts — cues re-timed per part (splitting a transcript to match `split`/`cut` parts); `--resync O1,O2,N1,N2` remaps cue times linearly through two sync points (offset+drift in one pass — retiming subs exported for a different cut), `--find needle` keeps only cues containing the text (locate every 'um'/phrase spoken — cut or caption around them); `--strip-tags` drops inline markup from cue text — `{\…}` ASS override blocks (`<…>` tags are already dropped on load) for transcripts that burn literal markup; `--strip-sdh` drops `[SOUND]`/`(door creaks)`/`♪` annotations — dialogue-only transcript for burning; `--clip F,T` keeps cues overlapping a window, clamps the edges and re-times to 0 (grab the subtitle chunk for a cut segment — `end` ok for T), `--drop F,T` drops the window's cues and re-times the tail left by (T-F) — the `cut --drop` counterpart for transcripts, `--move N,T` re-seats cue N to start at T (nudge one mis-timed cue — N is the input file's numbering, duration kept) |
| split into exactly N parts | `split` (`--parts N` — equal-length grid) |
| title that fades in/out | `title` (`--fade` secs — soft entry/exit, `--box` card) |
| end-card title / tail-only effect | `<verb> --at end --dur N` — every `--at/--dur` window verb anchors the tail (title, speed, tempo, mix, music, mute, boomerang, zoom, blur, grade, volume, censor, meme, overlay, delogo, eq, reverb, fx, denoise, dehum, leveler, gate, vocal, voice, vdenoise, pitch, progress, waveform/spectrogram, bw/invert/sharpen/vignette). `thumb`/`cover`/`frames --at end` = last frame |
| dip-to-black at a cut / at every scene mark | `fade` (`--dip T --dur N` — half out, half back; comma list dips at several points; `--curve qsin/esin/hsin/log/qua/cub/exp` shapes the audio fade) |
| fix white balance / color cast | `grade` (`--hue` deg — rotates the hue) |
| quiet tail on a podcast | `silence` (`--end --dur` secs — appended) |
| logo/watermark that eases in | `overlay` (`--fade` secs — alpha in/out) |
| subtitle file is early/late (all or just a stretch) | `subs` (`--shift ±N` — retimes every cue; `--from`/`--to` bounds it) |
| full podcast/music tags | `meta` (`--album`/`--genre`/`--date`/`--track`/`--disc`/`--composer`/`--bpm`/`--lyrics file.lrc`/`--copyright` — LRC timestamps stripped into the lyrics tag; `--album-artist`/`--show`/`--season`/`--episode`/`--network` TV & podcast-feed set, `--creation-time`/`--location` archive/geo stamps + `--media-type` iTunes stik kind (music/tvshow/movie/audiobook) + `--gapless` continuous-album flag + `--description`/`--synopsis` episode notes & blurb + `--hd` iTunes HD badge + `--lang-audio eng,jpn`/`--lang-subs eng,fra` per-track language tags in track order — blank slots skip, players show the track language in the menu; `--title-audio "Program,Commentary"`/`--title-subs "English,Français"`/`--title-video "Main,Angle-2"` per-track display titles — players name the track instead of "Track N" (multi-cam files label each angle; stream titles need mkv — mp4 drops them)) |
| keep only the good parts | `cut` (`--ranges "10-20,40-50"` — joined) |
| solid color card / backplate | `solid` (`--color`/`--size`/`--dur`/`--fps`, optional silent track), `--noise` grain, `--text` card text (`--wrap`/`--align`), `--fade` card fades |
| boost without clipping | `volume` (`--limit` dBTP — brickwall after the gain) |
| rip out a middle section | `cut` (`--drop "30-45"` — keeps the rest joined; `end-N` = trim the tail) |
| keep only the tail / several parts | `cut` (`--ranges` — `T-end` through the tail, `end-N` last N secs) |
| text that survives busy frames | `title` (`--outline` — stroke around every glyph) |
| bars in brand color | `fit` (`--color` — pad fill instead of black) |
| burned subs, your style | `subs` (`--size`/`--color`/`--top`/`--outline`/`--font`) |
| audiogram in brand colors | `audiogram` (`--bg` backdrop) |
| subtle watermark | `overlay` (`--opacity` on `--image`) |
| split a podcast on pauses | `split` (`--silence=-35` — cuts at gap midpoints) |
| music bed that eases in/out | `music` (`--fade` on the bed) |
| one-word EQ curve | `eq` (`--preset voice/podcast/bright/bass/warm/air`), `--band` parametric, `--tilt`, `--deemph riaa/cd/fm50/fm75` undoes vinyl/FM/CD pre-emphasis, `--shelf low|high:FREQ:GAIN` shelves (rumble cut / air shelf), `--notch FREQ[:WIDTH]` kills a resonance, `--brickwall LO,HI` FFT bandpass (telephone / speech-band 300,3400), `--lowpass`/`--highpass`/`--bandpass FREQ[:W]` resonant Butterworth filters (+`--linear` = sinc+afir linear-phase ~60dB-stopband mastering cuts, no --at), `--subcut FREQ` mic-stand rumble, `--supercut FREQ` ultrasonic hiss on 96k masters, `--superpass FREQ[:Q]`/`--superstop FREQ` razor order-10 band isolate/kill, `--allpass FREQ:W` phase rotator (symmetrize lopsided vocals → free headroom) |
| soft b-roll cutaway edges | `broll` (`--fade`), `--position` pip (+`--border` ring), `--opacity` ghost insert |
| stills at exact moments | `frames` (`--at 12,45,90`) |
| audiogram on any canvas | `audiogram` (`--size` — 1080x1920, 1920x1080, 1080x1080) |
| swapped audio eases in/out | `replace` (`--fade`) |
| audiogram with a title | `audiogram` (`--text "EP 12"` near the top) |
| thumbnail at a given size | `thumb` (`--width`) |
| inverted flash/accent | `invert` (`--at`/`--dur`) |
| blur just a moment | `blur` (`--at`/`--dur`; `--engine gblur|directional|box|avg` — box = fast blocky kernel, avg = lightest area-average) |
| motion trails / ghost smears | `trail` (`--mode echo` tmix smear, `--frames`, `--at`/`--dur`; `--mode light` bright-pixel persistence via lagfun, `--decay`) |
| datamosh glitch | `glitch` (`--strength` channel-shift + noise; `--engine planes|swapuv|stutter|pixels|swaprect|random` variants) |
| partial invert | `solarize` (`--threshold`, `--at` window) |
| breathing zoom | `pulse` (`--rate`/`--depth`, `--at` window) |
| timelapse flicker fix | `deflicker` (`--size` frames) |
| relief emboss | `emboss` (`--amount` mix, `--at` window) |
| tilt-shift miniature | `tilt` (`--band` sharp strip, `--blur`) |
| handheld drift | `sway` (`--rate`/`--px`, `--at` window) |
| shaky walking / handheld shot | `stabilize --engine vidstab` (two-pass vid.stab — steadier than single-pass deshake; `--smoothing` frames) |
| rack-focus breathing | `rack` (`--rate`/`--blur`) |
| ink the edges | `outline` (`--strength`, `--at` window) |
| night-vision look | `night` (`--grain`, `--at` window) |
| falling snow | `snow` (`--density`/`--speed`, `--at` window) |
| impact punch (flash+shake) | `impact` (`--at`/`--amp`/`--flash`) |
| watery wave distortion | `wave` (`--amp`/`--speed`, `--at` window) |
| pendulum sway tilt | `spin` (`--deg`/`--rate`, `--at` window) |
| spotlight circle mask | `iris` (`--x`/`--y`/`--radius`, `--at` window) |
| radial zoom smear | `burst` (`--strength`) |
| sub-bass drop | `thump` (`--at`/`--freq`/`--gain`) |
| rising chirp to a hit | `riser` (`--at` lands, `--dur`/`--gain`) |
| airy whoosh swell | `whoosh` (`--at` lands, `--dur`/`--gain`) |
| voice sibilance tamer | `deesser` (`--amount`/`--freq`/window) |
| field-recorder mid/side stereo | `channel --mode ms` (decodes MS back to L/R) |
| loud+quiet speech in one file | `leveler --engine mcompand` (multiband lift+cap across rumble/body/air) |
| clipped/blown-out audio rescue | `declip` (`--engine clip` = `adeclip` peak interpolation; `--engine click` = `adeclick` vinyl pops/dropouts; `--window`/`--threshold`/window) |
| magnify subtle motion | `amplify` (`--amount`/`--radius`/`--threshold`, `--at` window) |
| sky/gradient banding fix | `deband` (`--strength`/`--radius`/window) |
| drop duplicate frames | `dedup` (`--frac`, VFR out) |
| audiogram CQT music spectrum | `audiogram --mode cqt` |
| audiogram scrolling spectrogram | `audiogram --mode spectro` |
| find black/frozen stretches (QC) | `scan` (JSON extras; no -o) — also `interlaced`/frames_* from idet, `has_cc`/`cc_lines` EIA-608 closed captions, `crop_hint`/`letterboxed` cropdetect letterbox QC, `vfr`/`vfr_ratio` variable-frame-rate QC, `--dupe REF` MPEG-7 duplicate/re-upload match, `--text` OCR burned text, `rg_gain_db`/`rg_peak` ReplayGain tags, `--motion` VMAF motion score (`motion_avg`/`motion_max` — bitrate-budget QC), `--timecode` VITC readout (`vitc`/`vitc_tc`/`vitc_frames` broadcast-master QC), `--bbox` content bounds (`content_detected`/`content_box`/`content_fill` — works on any uniform background, not just black), `--loud` EBU R128 loudness summary (`loud_i`/`loud_lra`/`loud_tp` — platform spec gate, audio-only files too), `hdr`/`wide_gamut`/`color_space`/`color_primaries`/`color_transfer` HDR & gamut QC from container colour tags, `--gop` keyframe-interval QC (`keyframes`/`keyframe_times`/`gop_max_sec`/`gop_avg_sec`/`gop_max_frames` — packet flags, no decode; "keyframe every ≤2s" ingest spec), `--hash` decoded-frame checksums to `<input>.framemd5` (archive-ingest integrity manifest) | `timecode` container TC (mov tmcd / mkv TIMECODE — slate-master QC, no decode), `--bitrate` packet-map video-rate curve (`bitrate_mean_mbps`/`bitrate_peak_mbps`/`bitrate_spike_at` — peak-rate ingest spec, no decode), `av_desync_ms` audio/video start_time offset (container lip-sync QC — no decode; repair via `remux --audio-delay`/`--video-delay`), `start_time` earliest stream start (negative/odd capture starts — repair via `remux --offset`) |
| dust specks / hot pixels | `dedust` (`--size` 1-4, `--dark` for dark specks; morphology, not blur; `--engine temporal` tlut2 kills one-frame sparkles/VHS dropouts) |
| inverse telecine | `deinterlace --engine fieldmatch` (film 29.97i → 23.976p) |
| denoise without melting detail | `vdenoise --engine edge` (nlmeans masked to flat areas) |
| fill border slivers | `extend` `--left|--right|--top|--bottom N --mode smear` (chroma-key rims, leftover letterbox) |
| HDR → SDR | `tonemap` (zscale linear + `--algo hable|reinhard|gamma|clip|linear` + bt709; PQ/HLG phone footage for SDR platforms) |
| 24p → NTSC interlace | `telecine` (`--pattern 23` 3:2 pulldown — broadcast/DVD-era delivery; inverse of fieldmatch) |
| straight ↔ premult alpha | `premult` (`--mode premultiply|unpremultiply`, alpha-safe prores4444 out — AE/Motion handoffs) |
| star trails / light painting | `stack --mode max` (maskedmax; `--mode min` = darkest composite, median default removes intruders) |
| pullup judder wobble | `dejudder` (`--cycle 4` for 3:2 telecine) |
| texture smooth, no blur | `smooth --engine deflate|inflate` (morphological — pores/texture, zero halo) |
| mono-collapse check | `scan` `phase_corr` extra: ~-1 = channels cancel on mono speakers |
| grain/noise budget check | `scan` `noise_floor`/`noisy` extras (bitplanenoise LSB occupancy — noisy sources devour bitrate) |
| keyed edge still green | `despill` (`--type green|blue`, `--mix`, `--expand` — recolour fringe, no keying) |
| clean 3:2 cadence | `deinterlace --engine detelecine` (deterministic inverse telecine; fieldmatch still picks its own) |
| butter-smooth 60fps | `interp` (`--fps 60`; `--slow 0.5` = smooth slow-mo from normal footage — motion-compensated in-betweens) |
| archive interlaced | `deinterlace --engine mcdeint` (motion-compensated — best quality, slowest) |
| white balance by Kelvin | `grade --kelvin 3000` (tungsten 2700 / daylight 5500 / cool 9000 — the camera dial, not a warm slider) |
| SD looks green in HD edit | `matrix` --to bt709 (colormatrix converts 601→709 — not just re-tagging; `--engine colorspace` also maps primaries+transfer for bt2020↔709) |
| film-style B&W | `bw --weights 1.5,0.3,0.1` (channel weights — red filter darkens skies like film) |
| blockbuster split-tone | `grade --split 0.8` (teal shadows + orange highlights; negative flips) |
| freeform tone curve | `grade --curve "0/0.08 0.5/0.55 1/1"` (matte fade / S-curve; curves master points) |
| broadcast QC clamp | `legalize` (limiter pins luma to 16-235; --min/--max custom) |
| lifted web rip | `levels` --in-min 0.06 --in-max 0.92 (Photoshop levels; --out-min = matte fade) |
| xerox / graphic B&W | `bw --cut 0.5` (hard luma threshold, not grayscale) |
| audio peak & mean | `scan` extras `audio_max_db`/`audio_mean_db` (volumedetect: clip + cheap loudness) |
| loudness spec gate | `scan --loud` extras `loud_i`/`loud_lra`/`loud_tp` (EBU R128: podcast ≈−16 I, broadcast ≈−23 I) |
| heat ripple / liquid warp | `displace` --map clip.mp4 (warp by another clip's luma; `--edge` wrap/mirror/smear/blank, `--at` window) |
| sharpen without halos | `sharpen --engine halo` (unsharp clamped to blurred base — strongest option, zero overshoot) |
| EQ applied + curve shown | `eqviz` --bands "f=200 w=100 g=10 t=h" (EQ'd audio + its response curve as video — mix QC card) |
| pixel-art / retro upscale | `upscale --engine xbr|two-xsai|hqx|epx` (integer-scale sprite edges, no ringing; factor snaps) |
| smarter saturation | `grade --vibrance -1..1` (boosts muted colors, protects saturated skin — safer than --saturation on faces) |
| crunchy edge outlines | `edge --engine sobel|kirsch|roberts|prewitt` (classic convolution kernels vs Canny-style edgedetect) |
| real noise-cancel | `denoise --ref roomtone.wav` (anlms adaptive cancel — a second mic's noise ref gets subtracted; podcast lav+room rigs) |
| cheap 60fps | `interp --engine framerate` (scene-aware frame blending ~10x faster than minterpolate; slight ghost on fast motion) |
| pick a good still | `thumb --best` (thumbnail filter scores each 100-frame batch — a clean typical frame from shaky footage) |
| meter-bridge audiogram | `audiogram --mode volume` (per-channel VU bars) |
| stereo-field audiogram | `audiogram --mode spatial` (showspatial image), `audiogram --mode bitscope` (bit-pattern scope) |
| blur QC | `scan` now reports `blur_frames`/`blur_mean`/`blur_min` (diff-entropy reads out-of-focus stretches; `--blur` tunes the cut) |
| connected line art | `edge --engine link` (hysteresis edge linking — Canny-style clean contours) |
| rescue a crushed rip | `smooth --engine uspp` (MPEG postproc deblock+dering, not a blur) |
| motion-only ghost | `trail --mode diff` (tblend difference — static background drops to black) |
| pipeline-health audiogram | `audiogram --mode monitor` (agraphmonitor stats viz) |
| mood colour wash | `grade --wash teal --wash-amount .4` (colorize veil that keeps luma) |
| match a reference look | `grade --match ref.mp4` (midequalizer pulls your histogram toward the reference — camera matching) |
| speed-line blur | `blur --engine directional --angle 30` (dblur streaks) |
| fast rip rescue | `smooth --engine pp7` (lighter postproc when uspp is too slow) |
| analog capture cleanup | `vdenoise --engine dotcrawl` (dedot: dot-crawl + rainbow edges on VHS / capture-card rips) |
| hex pixel readout | `scope --mode data --x 100 --y 90` (datascope grid of raw values at a point) |
| false-color glitch | `glitch --engine planes` (RGB channel rotation, clean acid look) |
| chroma borrow | `grade --color-from ref.mp4` (mergeplanes: your luma, their U/V color grade) |
| film-grain denoise | `vdenoise --engine fftdnoiz` (FFT-domain + temporal prev/next) |
| deblock twins | `smooth --engine spp|fspp` (light/fast postproc for blocky sources) |
| VHS stutter | `glitch --engine stutter` (shuffleframes frame-drop jitter) |
| 1D LUT support | `grade --lut curve.cube` (1D LUTs auto-route to lut1d — lut3d rejects them) |
| hard flicker kill | `deflicker --engine tmide` (temporal midway equalization for timelapse/strobe) |
| audio beds | `gen --pattern noise|tone|sweep --dur 30 -o bed.m4a` (pink-noise roomtone, sine reference tone, 20Hz→`--freq` speaker-test chirp) |
| chroma-swap look | `glitch --engine swapuv` (U/V flip — magenta↔green) |
| edit-point map | `scan --scenes` (scene_cuts timestamps via scdet — QC pass doubles as a cut list) |
| 3D format convert | `stereo` `--in sbsl --out arcd` (stereo3d: SBS→anaglyph preview, interleaved for 3D displays) |
| play an image as sound | `sonify` (spectrumsynth — paint/draw a picture → hear it; `--dur`/`--speed`/`--sample-rate`) |
| robot voice | `fx --kind ringmod` (fast-AM tremolo — Dalek/sci-fi comm channel) |
| edge-preserving smooth | `smooth --engine yaep` (yaepblur, bilateral-class) |
| archive deinterlace | `deinterlace --engine w3fdif` (Martin Weston 3-field, sharp diagonals) |
| precise logo mask | `delogo --image mask.png` (white pixels in your drawn bitmap = inpaint — non-rectangular logos) |
| compression QC | `scope --mode mvs` (codecview motion-vector arrows; coherent arrows = clean pans, jitter = noisy blocks) |
| cheap-lens fringe | `aberrate` --amount 5 (rgbashift: red left/blue right — VHS/glitch edge) |
| mono-compat visual | `audiogram --mode phase` (aphasemeter scope — thin line = mono, cloud = decorrelated) |
| beauty/skin smoothing | `smooth` (`--engine` smartblur/bilateral/sab — bilateral & sab keep edges sharper; `--strength`, `--at`/`--dur` window) |
| reframe 360/equirect footage | `v360` (`--yaw`/`--pitch`/`--fov`, `--in` projection, `--size`) |
| noisy clip, pick denoiser | `vdenoise --engine nlmeans\|hqdn3d\|atadenoise\|vaguedenoise\|bm3d` (bm3d/dctdnoiz/owdenoise strongest, no --at; `median` salt&pepper, `chroma` color speckle), `denoise --engine auto\|wavel\|fftdn` |
| old footage is too low-res | `upscale` (zscale spline36 + unsharp, `--factor` 2 doubles dims) |
| filmed a screen/whiteboard at an angle | `perspective` `--points x0,y0,...,x3,y3` deskews the quad onto the frame (TL,TR,BL,BR) |
| broadcast range tag | `transcode --range limited` |
| real temporal interlace | `transcode --interlaced --interlace-mode weave` (60p→30i: consecutive frames woven into fields — feed double-rate progressive; default `il` = same-frame interleave) |
| halo-free sharpening | `sharpen --engine cas` (`--amount`) |
| auto-contrast flat footage | `equalize` (`--strength`/window) |
| dominant colors | `pick` (mean + 6-zone swatch, `--at`) |
| visual diff | `diff` (`--side` reference beside diff; `--mode mask --threshold` bare change-mask QC) |
| glitched/dropped frames | `repair` (`--ref` another take, `--at`/`--dur` the bad stretch, `--ref-at` the clean frame to paste in — freezeframes) |
| magnify subtle motion | `amplify` (`--amount` factor, `--radius` frames, `--threshold` diff cap, `--at` window) |
| keep one color | `selective` (`--color C`/`--similarity`/`--blend` edge feather, `--engine chroma` chromahold for saturated hues, `--at` window) |
| test card | `bars` (`--size`/`--dur`/`--hd`/`--tone` 1kHz, `--kind sd|pal100|pal75|rgb|yuv|allrgb|allyuv|mptest|testsrc` other patterns — allrgb/allyuv = full color-cube QC sweeps, mptest = encoder-torture cycle, testsrc = all-in-one animated calibration card) |
| QC scope overlay | `scope` (`--mode vector|wave|hist|mvs|data|qp|pix|osc|drift|loud|cie|palette|graph|safe` — drift = luma-ramp curve, loud = loudness-over-time curve, cie = CIE-1931 gamut horseshoe, palette = color-swatch grid (GIF/8-bit QC), graph = live filtergraph stats card (encode-pipeline debug), safe = full-frame broadcast safe-area guides (90% action / 80% title + center cross), `--position` corner, `--at` window) |
| anamorphic restore | `desqueeze` (`--factor` lens ratio, `--axis`) |
| comic look | `cartoon` (`--levels` posterize, `--at` window) |
| thermal luma map | `heat` (`--preset` pseudocolor, `--at` window) |
| mandala mirror | `kaleido` (`--at` window) |
| mirror symmetry (dance) | `mirror` (`--axis x` left→right / `--axis y` top→bottom, `--at`/`--dur` window) |
| retro pixelation | `pix` (`--strength` 2-64 block divisor, `--at`/`--dur` window) |
| unmirror selfie / flip art | `flip` (`--axis x` horizontal / `--axis y`, `--at`/`--dur` window) |
| pop-art posterize | `poster` (`--levels` 2-64 palette colors, `--at`/`--dur` window) |
| two-color grade | `duotone` (`--shadow`/`--highlight` colors, `--at`/`--dur` window) |
| dreamy bloom | `glow` (`--strength` blur radius, `--at`/`--dur` window) |
| retro tape look | `vhs` (`--strength` 0-3 noise + chroma shift + scanlines, `--at`/`--dur` window) |
| shutter smear | `motionblur` (`--frames` 2-8 temporal blend, `--at`/`--dur` window) |
| flash cut | `strobe` (`--rate`/`--duty`/`--color`, `--at` window) |
| neon outline | `edge` (`--mode wires|colormix`, `--at` window) |
| fisheye / defish | `lens` (`--k1`/`--k2`, `--at` window) |
| text in a corner | `title` (`--position top-right` …) |
| B&W only for a moment | `bw` (`--at`/`--dur`) |
| sharpen only the key shot | `sharpen` (`--at`/`--dur`) |
| wipe metadata before posting | `meta` (`--clear`) |
| dark-edge only for a beat | `vignette` (`--at`/`--dur`) |
| grade only the dream sequence | `grade` (`--at`/`--dur`) |
| hear the b-roll under me | `broll` (`--audio`) |
| captions in my brand font | `subs` (`--burn --font`) |
| progress bar only in the back half | `progress` (`--at`/`--dur`, `--opacity` ghost bar) |
| waveform band at the top | `audiogram` (`--position`) |
| pull OUT of a shot (reveal) | `zoom` (`--out`, `--center X,Y` punch target) |
| title with a soft shadow | `title` (`--shadow`) |
| still at an exact width | `extract` (`--gif` clip, `--width`, `--at end` last frame, `--chapter N` pulls embedded chapter N as its own file), `--loop` gif repeats, `--alpha` pulls the alpha channel out as a grayscale PNG (matte QC/export — needs an alpha-capable input), `--gif --transparent` keeps alpha in the GIF (Discord/Telegram stickers — needs prores 4444/qtrle source), `--webp` animated WebP clip (smaller than GIF, alpha kept natively — `--lossless`, `--bounce` ok), `--audio` rips the audio track losslessly (stream copy — `-o` extension picks the container; `--track N` picks which, `--all` dumps every track to `stem_aN.<ext>`), `--subs` pulls an embedded subtitle track to .srt/.ass/.vtt (`--track N` picks the language, `--all` dumps every track to `stem_sN.<ext>` — captions out of a finished export for re-timing or re-burning), `--lang jpn` picks one track by its language tag on either rip (use --all for every track), `--from/--to` rips just a window of the audio track (clip a song cue without re-encoding), `--attachment N` rips attachment stream N back out to a file (fonts/files a `remux --attach` put in — mkv/webm), `--cover` pulls embedded cover art back out as an image (the `remux --cover` round-trip — verify feed art or grab it for redesign), `--keyframes` dumps every I-frame as an image (GOP-boundary stills — keyframe-interval QC, timelapse source, fast scene scouting; bare -o gets `stem_%03d`) |
| countdown with tick beeps | `countdown` (`--beep`, `--text` label during the count) |
| one-word compressor curve | `leveler` (`--preset`, `--engine compand` single-band transfer curve — quieter than acompressor's knee) |
| spectrogram in brand colors | `spectrogram` (`--color`) |
| music kicks in after the intro / sting at both ends | `music` (`--at`/`--dur`, comma `--at 0,end` = intro+outro stings) |
| fix an out-of-phase mic | `channel` (`--mode invert --side`) |
| rescue dark or blown footage | `grade` (`--exposure -3..3` — real EV stops, not a brightness slide) |
| contact-sheet breathing room | `sheet` (`--pad`/`--margin`) |
| diagonal watermark | `overlay` (`--angle`) |
| short overlay clip repeats | `overlay` (`--loop` — covers the base) |
| waveform showing quiet detail | `waveform` (`--scale log`) |
| split on longer pauses | `split` (`--silence --min-silence`) |
| wobble/sci-fi/echo/lofi/telephone voice | `fx` (`--kind` 22 effects, incl. `saturate` warmth, `excite` air, `crush` bitcrusher, `ringmod` true AM robot, `fshift` metallic alien, `contrast` dynamics tilt, `wah` auto-wah swept resonant peak) |
| effect only in the drop | `fx` (`--at`/`--dur`) |
| boomerang that loops 3x | `boomerang` (`--times`), `--at/--dur` window |
| H.265 for Apple / smaller archive | `transcode` (`--preset hevc`) |
| edit proxies for the NLE | `transcode` (`--preset proxy` — ≤960x540 veryfast x264 + aac 96k; smooth scrubbing on long takes / multicam dailies, never a delivery format) |
| gate tuned for speech vs studio | `gate` (`--preset voice|podcast|studio`) |
| echo/reverb only on the hook | `reverb` (`--at`/`--dur`) |
| bass boost only on the drop | `eq` (`--at`/`--dur`) |
| repeat just the funny bit | `loop` (`--from`/`--to`/`--times`, `--fade` seamless joints; `--to end` ok) |
| selectable soft subs in mp4 | `subs` (`--mux file.srt --lang spa`) |
| stroked TikTok captions | `caption` (`--outline RRGGBB`) |
| branded audiogram title font | `audiogram` (`--font`) |
| name each tile in a grid | `grid` (`--labels "a,b"`), `--fill` crop-fill cells |
| gentle logo cleanup | `delogo` (`--soft`; `--regions x:y:w:h,...` covers several spots; `--shape circle` elliptical mask) |
| animated gradient card | `solid` (`--gradient ff0000:0000ff`) |
| reframe / crop out an edge | `crop` (`--region x:y:w:h` or `--aspect 1:1`/`9:16` centered) |
| motion / loop | `speed`, `reverse`, `loop`, `stabilize` (`--edge` fill), `fade` |
| picture | `grade` (+ `--lut` .cube/.png), `bw`, `vignette`, `sharpen`, `blur` |
| logo / PiP | `overlay` |
| green screen | `key` (`--bg`, `--color`/`--similarity`/`--blend`, `--despill` for fringe, `--mode luma` keys a brightness band (`--threshold` pivot) instead of a color, `--mode chroma` is the YUV-domain keyer (broadcast chromakey — better on wrinkled/uneven screens), `--at`/`--dur` window — comma list ok) |
| hand-drawn matte → alpha | `key --mode matte --mask` (mask luma becomes alpha → prores 4444) |
| reaction / multi-cam grid | `grid` (`--layout 2x2`, `--size`, `--gap`/`--bg` gutters, `--time` stamps every tile, `--focus` hero layout — first input big left ~2/3, rest stack right) |
| watch-time progress bar | `progress` (`--color`, `--height`, `--edge`, `--bg` track, `--reverse` depletes the bar) |
| freeze a beat / outro hold | `freeze` (`--ease`/`--reverse` swoop, `--zoom` push-in, `--at T --dur D` — comma `--at` freezes at several points, or `--end D`) |
| blur a face / logo | `censor` (`--region x:y:w:h` — comma list covers several spots, `--mode pixel|blur|solid` (solid = black-bar redact), `--strength`, `--shape circle` ellipse mask; `--at`/`--dur` limits the window) |
| slow-mo punch-in | `speed` (`--factor`/`--ramp`, `--at`/`--dur` for just one window); `speed --fit SEC` retimes the whole clip to an exact length (auto factor — a 90s take --fit 15 becomes 6x) |
| boomerang replay | `boomerang` (forward then reversed, one loop) |
| YouTube/player chapters | `chapter` (`--at T|TITLE` repeatable, `--auto` silence gaps, `--remove` strips; lossless; `--yt` export/`--import` YouTube `H:MM:SS Title` lines, `--cue` CUE sheet, `--podcast` Podcasting 2.0 JSON chapters, `--lrc` synced-lyrics cues for music players, `--vtt` WebVTT chapter file for web `<track kind="chapters">` nav, `--spread N` even-grid marks (uniform TOC — `--titles a,b,c` names them); `--csv` exports H:MM:SS.mmm,Title lines for Resolve/Premiere markers and spreadsheet round-trips, `--edl` exports a CMX-style EDL at the clip's frame rate (NLE timeline markers), `--fcpxml` exports Final Cut Pro XML — FCP/Resolve import each mark as a timeline marker (editor-native TOC exchange), `--srt` exports the marks as a soft-sub file — each chapter title becomes a cue spanning to the next mark (burn to preview where seek points land); `--import` auto-detects .json/.cue/.lrc/.vtt/.csv/.srt/.fcpxml (transcript → TOC: every cue becomes a mark titled by its first line); `--scenes` scdet scene-cut detection — marks every cut like `split --scenes` finds (auto-TOC for unmarked masters), `--rate R` rescales every mark ×R (marks authored for a retimed cut — scales before --shift) |
| punch-zoom a moment | `zoom` (`--factor`, `--at`/`--dur`) |
| strip letterbox/pillarbox | `autocrop` (cropdetect scan → crop, `--buffer N` keeps N px edge) |
| contact sheet / preview grid | `sheet` (`--cols`/`--rows`/`--tile` → PNG, `--time` stamps, `--from`/`--to` window) |
| player seek-preview thumbnails | `sprite` (`--every` secs → `<stem>-N.jpg` sheets + `.vtt` with `#xywh` cues) |
| title card mid-clip | `title` (`--text` or `--file notes.txt`, `--at` S for lower-third timing) |
| voice-over on video's own audio | `replace --audio V --mix G --duck` (sidechain) |
| pitch-shift voice/music | `pitch` (`--at`/`--dur` window, `--semitones N`, duration preserved; `--formant` natural timbre via librubberband) |
| film grain | `grade --grain N` |
| auto cut on scene changes | `split --scenes 0.3` |
| strip dead air head+tail (audio) | `cutsil` (`--thresh -45`) |
| grid with one input's audio | `grid --audio N` |
| draft/tiled watermark | `overlay --tile N` (diagonal watermark pass) |
| shift subtitle timing | `caption --shift SEC` |
| gif tuning | `transcode --preset gif --fps --width`, `extract --gif --bounce` (palindrome loop), `extract --colors` palette size |
| headphone fatigue on long audio | `fx --kind crossfeed` (`--strength` 0..1 ear bleed); `fx --kind sub` adds a synthesized low octave; `fx --kind autopan` sweeps L-R |
| draw a freehand EQ curve | `eq --curve "80,0;3000,-6;8000,4"` (freq,gain dB points, interpolated) or `eq --graphic` 18-band classic EQ |
| animated backdrop for a music/text card | `gen` `--pattern mandelbrot\|gradients\|life\|sierpinski\|hald` (no input file; `--size`/`--dur`/`--colors`/`--seed`) — `hald` writes an identity HALD LUT PNG (`--level`, default 8) to grade in an editor then feed `grade --lut`; audio patterns `noise` (`--color white/pink/brown/blue/violet/velvet`), `tone --freq`, `sweep`, `silence` (anullsrc digital-black bed for padding) |
| italic-style slant / dynamic tilt | `shear` `--x`/`--y` (-2..2; `--fill` edge color, `--interp`) — `--at`/`--dur` windows |
| fix a color cast / white balance | `wb` (auto per-channel normalization; `--strength`, `--independence 0` keeps grade, `--smooth` frames, `--engine greyedge` gentler cast fix) |
| QC a clip for strobes before posting | `scan` — also reports `flash_frames`/`flash_max_badness` (photosensitive-epilepsy check) |
| stereo too wide / phase issues | `channel` `--mode base --pan -1..1` (-1 folds to mono, +1 widens) |
| one-ear voice fix / pan the mix | `channel` (`--mode dualmono`/`mono`/`swap`/`mix51` surround→stereo, `pan --pan -1..1` to one ear, `bal --pan -1..1` rebalance lopsided stereo, `split` → `_L/_R.wav` host/guest stems, `merge --with B` two tracks → one multichannel file (mono+mono → stereo host-L/guest-R podcast), `mid`/`side` M/S extract, `haas` stereo widening, `surround` stereo→5.1 upmix, `bands --freqs 300,3000` → `<stem>_bandN.wav` frequency-band stems for remixes, `sync --side right --cm 34` delay the closer mic by its distance to fix two-mic comb-filtering, `earwax` headphone widening) |
| loop to a length | `loop --until SEC` |
| text draft watermark | `title --tile N` |
| audio EQ polish | `eq` (`--bass`/`--treble`/`--presence` dB) |
| animated push-in | `zoom --motion kenburns` |
| still-image cutaway | `broll --insert img.png --still` |
| wrong-orientation phone clip | `rotate` (`--deg`/`--flip`) |
| burned-in logo/watermark | `delogo` (`--x --y --w --h` or `--regions x:y:w:h,...` for several; `--at`/`--dur` only some of the time; `--find logo.png` auto-locates it in the first 15s — no coordinates needed; `--find + --track` follows a MOVING mark every frame via cover_rect) |
| smooth slow-mo | `speed --factor 0.5 --interp` |
| styled title text | `title --size 2 --color ff0000` |
| lower-third placement | `title --position bottom` (or `top`/`center`) |
| container metadata tags | `meta` (`--title`/`--artist`/`--comment`, `--copy` pulls tags+chapters from another file) |
| fix display rotation flag | `meta --rotate 90` (lossless; clears with `--rotate 0`) |
| room tone on a voice | `reverb` (`--size room|hall|cave`, `--wet 0..0.9`, `--ir file.wav` convolution reverb from IR packs, `--tail`) |
| wobble/sci-fi/echo/lofi/telephone/saturate/excite/crush audio | `fx` (`--kind`, `--strength`, `--at`/`--dur`; `contrast` expands/compresses dynamics) |
| Ken Burns on a photo cutaway | `broll --insert img.png --still --motion kenburns` |
| styled captions | `caption --color ff0000 --size 1.5` |
| logo only for part of the clip | `overlay --at 2 --dur 5` |
| rip embedded subtitles | `subs` (`--stream N`, `--all` every stream) |
| bleep out a word | `bleep` (`--at`/`--dur`, comma list for several spots; `--freq`/`--level`) |
| warm/cool white balance | `grade --warm -1..1` |
| B-roll cutaway | `broll` (`--insert --at --duration`, comma `--at` flashes it at several points; A-roll audio stays; `--loop` replays short inserts) |
| extract | `extract` |
| many files | `batch` |
| no verb | `graph` — [graph.md](references/graph.md) |
| still uncovered | `ffmpeg` `--because "REASON"` `-- -i IN … OUT` |

`pipeline` runs the scheme. `install-skill` and `version` are setup, not plan steps.

## Report

User language for the prose; keep these labels in English. Numbers from `--json`. Quote the plan you ran.

```
Plan: 剪掉停顿 → 片头字 → 烧字幕 → 导出 Reel
Done: reel.mp4 — 58.2s 1080x1920 30fps h264 aac
Steps: probe -> jumpcut -> title -> caption burn -> deliver reels -> look --at 1
Check: matches “做成 Reel 并能静音看完” (verified: true)
Look: reel_frame.png (hook readable, captions clear of the bottom UI)
Notes: …
```

`Failed:` quote `error.message`. A pipeline failure names the step. `Look: not needed` when the picture did not change.

## References

- [pipeline.md](references/pipeline.md) — the plan file `ffkit pipeline` runs (`$src`, `$in`, `expect`)
- [recipes.md](references/recipes.md) — schemes to adapt after the user agrees
- [gotchas.md](references/gotchas.md) — VFR, even sizes, concat copy, GIF palette
- [graph.md](references/graph.md) — filter graph when verbs cannot express a step
- [platforms.md](references/platforms.md) — Reels / Shorts / YouTube / GIF

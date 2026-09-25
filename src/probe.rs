use std::path::Path;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::error::Error;
use crate::spawn::{self, Argv};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Probe {
    pub duration: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fps: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vcodec: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acodec: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channels: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sample_rate: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pix_fmt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_space: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_primaries: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_transfer: Option<String>,
    pub has_video: bool,
    pub has_audio: bool,
    /// A `data` codec_type stream is muxed in (GoPro telemetry, timed
    /// metadata) — strip candidates before delivery
    pub has_data: bool,
    /// An `attachment` codec_type stream is muxed in (subtitle fonts,
    /// embedded payloads) — gate for `extract --attachment` / `remux
    /// --no-attachments`
    pub has_attachment: bool,
    /// A subtitle stream is muxed in (srt/ass/mov_text — gate extract
    /// --subs / remux --no-subs / deliver --subs)
    pub has_subs: bool,
    pub encrypted: bool,
    /// Container-wide bitrate (format.bit_rate) — overall-budget QC for
    /// platform ingest caps
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bit_rate: Option<u64>,
    /// Video carries an alpha channel (yuva*/rgba family) — QC before
    /// shipping sticker/overlay assets where transparency matters.
    pub has_alpha: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    pub variable_frame_rate_suspected: bool,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub subtitle_streams: u32,
    /// Absolute stream indices carrying the attached_pic disposition
    /// (album/feed art muxed as a video stream)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attached_pic_indices: Vec<u32>,
    /// |audio start - video start| in ms — lip-sync QC from the container
    /// (capture cards / multicam dailies drift). Missing on files whose
    /// streams don't carry start_time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub av_desync_ms: Option<f64>,
    /// Earliest stream start_time — some captures start negative or at a
    /// nonzero offset (players that can't seek those misbehave;
    /// `remux --offset` shifts it).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<f64>,
    /// Container timecode (mov tmcd → video-stream `timecode` tag, mkv
    /// `TIMECODE` format tag): QC for masters expected to carry a
    /// slate-matching TC; zero decode. None when absent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timecode: Option<String>,
    /// Every metadata tag grouped by source ("format", "stream:0", …) —
    /// a metadata audit: what did the last export actually stamp.
    /// Skipped when the file carries none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<serde_json::Map<String, serde_json::Value>>,
    /// Every elementary stream at its absolute index — multi-track QC:
    /// which stream carries which codec/language before a `remux --lang`
    /// or `extract --track` (dub/subtitle audits on deliverables).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub streams: Vec<ProbeStream>,
    /// Display-matrix rotation in degrees on the first video stream
    /// (phone-shot portrait footage; -90 == the rotate=90 tag). QC before
    /// `meta --rotate` — None when the container carries no rotation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rotation: Option<f64>,
    /// Number of chapters embedded in the container (TOC QC on deliverables —
    /// `chapter --list` shows the marks themselves)
    #[serde(default)]
    pub chapter_count: u32,
    /// Programs multiplexed into the container (format.nb_programs —
    /// multi-service mpegts/spts deliverables: >1 means a mux carries
    /// several programs, pick before repack or you get all of them)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub program_count: Option<u32>,
    /// Container detection confidence 0-100 (format.probe_score — a
    /// <100 score means the probe was unsure: mis-detected or damaged
    /// container; catch it before a silent bad repack)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub probe_score: Option<u32>,
    /// Multiplexed services in the container (mpegts/spts — which channel
    /// each program is and which streams carry it; `remux --program N`
    /// picks one by `num`)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub programs: Vec<ProbeProgram>,
}

/// One line of the stream table — index matches `remux`/`extract`
/// stream selection and `probe.tags`' `stream:N` keys.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProbeStream {
    pub index: u32,
    /// video | audio | subtitle | data | attachment | …
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub codec: Option<String>,
    /// Codec tag string (avc1/hvc1/hev1/mp4a/stpp…) — QC that
    /// `remux --tag` landed; untagged streams read `[0][0][0][0]` and
    /// are filtered out.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub codec_tag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// Per-track display title (meta --title-audio/--title-subs/
    /// --title-video writes these; mkv/webm only — mp4 drops them)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channels: Option<u32>,
    /// Audio sample rate Hz (audio streams — QC after `conform --ar` /
    /// `transcode --ar` on multi-rate files)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sample_rate: Option<u32>,
    /// Codec profile string (h264 High/Baseline, aac_lc, …) — platform
    /// ingest specs reject some profiles (High 10 / 4:2:2 on social)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    /// Average frame rate (video — QC `conform --fps`/`transcode --fps`
    /// landed on every track of a mixed-rate file)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fps: Option<f64>,
    /// Real base frame rate (r_frame_rate) — differs from `fps` on VFR
    /// footage, so `r_fps != fps` flags the per-track variable rate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r_fps: Option<f64>,
    /// Stream duration in seconds (truncated-track QC — an audio track
    /// shorter than the video tail leaves dead air)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<f64>,
    /// Per-stream bitrate in bps (per-track rate QC — ingest specs gate
    /// video bitrate separately from audio)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bit_rate: Option<u64>,
    /// Pixel format (video — 4:2:2/10-bit masters get rejected by social
    /// platforms needing yuv420p; QC every track of mixed-depth files)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pix_fmt: Option<String>,
    /// Sample aspect ratio (video — anamorphic masters carry SAR ≠ 1:1;
    /// QC before re-encoding drops the tag and squeezes the picture)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sar: Option<String>,
    /// Display aspect ratio (video — the shape a player renders;
    /// "16:9" anamorphic vs stored width/height)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dar: Option<String>,
    /// Field order (video — "progressive" / "tt"/"tb"/"tff"/"bff";
    /// interlaced masters trip deinterlace + broadcast deliverable QC)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field_order: Option<String>,
    /// Channel layout (audio — "stereo"/"mono"/"5.1"; a 5.1 master
    /// hiding among stereo deliverables trips platform spec QC)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_layout: Option<String>,
    /// Color space tag (video — a bt2020 track hiding among bt709s in a
    /// multi-angle file; HDR/SDR deliverable QC per track)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_space: Option<String>,
    /// Color primaries (video — bt2020/bt709; the HDR/SDR pair with
    /// color_transfer a platform's HDR spec requires)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_primaries: Option<String>,
    /// Transfer characteristic (video — smpte2084=PQ / arib-std-b67=HLG;
    /// an SDR track in an HDR package slips spec QC without it)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_transfer: Option<String>,
    /// Color range (video — "tv" limited / "pc" full JPEG-range; a
    /// full-range file through a limited-range pipeline crushes blacks)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_range: Option<String>,
    /// Sample format (audio — "s16"/"fltp" planar vs packed; delivery
    /// spec QC on the actual sample encoding, not just the codec name)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sample_fmt: Option<String>,
    /// Per-track start_time (audio/video — a track starting late is a
    /// baked-in lip-sync/crossfade offset, the raw signal behind
    /// `av_desync_ms`)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<f64>,
    /// Total frame count (video — exact frame budget for split math /
    /// verifying -frames:N caps landed; absent on raw/nut streams)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nb_frames: Option<u64>,
    /// Codec level (video — e.g. 40 = H.264 Level 4.0; device spec QC —
    /// pairs with `profile` for "High@L4.0")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<u32>,
    /// Player-default track (disposition.default) — QC which track a
    /// player picks before `remux --default-audio`/`--default-sub`.
    #[serde(default, skip_serializing_if = "is_false")]
    pub default: bool,
    /// FORCED-flagged subtitle track — QC that `remux --forced-sub` landed
    /// (film-style captions players auto-show for the audience's language)
    #[serde(default, skip_serializing_if = "is_false")]
    pub forced: bool,
    /// Hearing-impaired/SDH track (accessibility — QC that `remux --sdh`
    /// landed; players label the track "SDH")
    #[serde(default, skip_serializing_if = "is_false")]
    pub hearing_impaired: bool,
    /// Commentary track (director's commentary — QC `remux --commentary`)
    #[serde(default, skip_serializing_if = "is_false")]
    pub comment: bool,
    /// Visual-impaired/audio-description track — QC `remux --audio-desc`
    #[serde(default, skip_serializing_if = "is_false")]
    pub visual_impaired: bool,
    /// Dub track (dubbed-language track — QC `remux --dub`)
    #[serde(default, skip_serializing_if = "is_false")]
    pub dub: bool,
    /// Original-language track — QC `remux --original`
    #[serde(default, skip_serializing_if = "is_false")]
    pub original: bool,
    /// Attached-picture track (muxed cover art — QC that `remux --cover`
    /// landed and which stream index carries it)
    #[serde(default, skip_serializing_if = "is_false")]
    pub attached_pic: bool,
    /// Coded frame size vs display size (video — macroblock-padded encodes
    /// store e.g. 1920x1088 for 1080p; padding QC on masters)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coded_width: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coded_height: Option<u32>,
    /// Container stream id (video/audio — "0x100"-style PID on mpegts,
    /// track id elsewhere; multi-program transport-stream QC)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_id: Option<String>,
    /// avcc length-prefixed h264 (video — mp4/mov "true", mpegts/raw
    /// annex-b "false"; QC before muxing into HLS/fMP4 which need avcc)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_avc: Option<bool>,
    /// avcc length prefix bytes (video — 4 on normal mp4s; pairs with
    /// is_avc for the annex-b → avcc conversion decision)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nal_length_size: Option<u32>,
    /// 4:2:0 chroma siting (video — "left"/"center"/"topleft"; broadcast
    /// spec QC on chroma phase alignment)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chroma_location: Option<String>,
    /// Source bit depth per sample (video/audio — the container's own
    /// 8/10/12-bit declaration; catches a 10-bit master even when the
    /// pix_fmt name is ambiguous)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bits_per_raw_sample: Option<u32>,
    /// B-frames in use (video — 0 means baseline/realtime-safe encode;
    /// >0 means decoder lookahead delay, QC for low-latency/mobile specs)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_b_frames: Option<u32>,
}

fn is_false(v: &bool) -> bool {
    !*v
}

fn is_zero(v: &u32) -> bool {
    *v == 0
}

impl Probe {
    pub fn summary(&self) -> String {
        let mut parts = Vec::new();
        parts.push(format!("{:.2}s", self.duration));
        if let (Some(w), Some(h)) = (self.width, self.height) {
            parts.push(format!("{w}x{h}"));
        }
        if let Some(fps) = self.fps {
            parts.push(format!("{fps:.2}fps"));
        }
        if let Some(v) = &self.vcodec {
            parts.push(v.clone());
        }
        if let Some(a) = &self.acodec {
            parts.push(a.clone());
        }
        if let Some(c) = self.channels {
            parts.push(match c {
                1 => "mono".into(),
                2 => "stereo".into(),
                n => format!("{n}ch"),
            });
        }
        parts.join(" ")
    }

    /// Member stream indices of multiplexed program `num` split by kind —
    /// `(video, audio, other)`. `hls`/`dash --program` map the members
    /// they need (a `0:p:N` program map can't compose with a media-type
    /// specifier, so ladders select by absolute index). `None` when
    /// `programs[]` carries no such `num`.
    pub fn program_members(&self, num: u32) -> Option<(Vec<u32>, Vec<u32>, Vec<u32>)> {
        let p = self.programs.iter().find(|p| p.num == num)?;
        let mut video = Vec::new();
        let mut audio = Vec::new();
        let mut other = Vec::new();
        for idx in &p.streams {
            match self
                .streams
                .iter()
                .find(|s| s.index == *idx)
                .map(|s| s.kind.as_str())
            {
                Some("video") => video.push(*idx),
                Some("audio") => audio.push(*idx),
                _ => other.push(*idx),
            }
        }
        Some((video, audio, other))
    }
}

#[derive(Deserialize)]
struct FfprobeOut {
    #[serde(default)]
    streams: Vec<FfprobeStream>,
    #[serde(default)]
    format: Option<FfprobeFormat>,
    #[serde(default)]
    chapters: Vec<ChapterTime>,
    #[serde(default)]
    programs: Vec<FfprobeProgram>,
}

#[derive(Deserialize, Default)]
struct FfprobeProgram {
    #[serde(default)]
    program_num: Option<u32>,
    #[serde(default)]
    tags: Option<std::collections::HashMap<String, String>>,
    #[serde(default)]
    streams: Vec<FfprobeProgramStream>,
}

#[derive(Deserialize, Default)]
struct FfprobeProgramStream {
    index: Option<u32>,
}

/// One multiplexed service in the container (mpegts/spts —
/// `remux --program N` picks it by `num`)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProbeProgram {
    /// Program number — the `remux --program N` selector
    pub num: u32,
    /// Broadcaster service name (which channel this program is)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_name: Option<String>,
    /// Member stream indices (positions in `streams[]`)
    pub streams: Vec<u32>,
}

#[derive(Deserialize, Default)]
struct FfprobeStream {
    #[serde(default)]
    codec_type: String,
    index: Option<u32>,
    #[serde(default)]
    disposition: Option<std::collections::HashMap<String, i64>>,
    #[serde(default)]
    codec_name: Option<String>,
    #[serde(default)]
    codec_tag_string: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    #[serde(default)]
    coded_width: Option<u32>,
    #[serde(default)]
    coded_height: Option<u32>,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    is_avc: Option<String>,
    #[serde(default)]
    nal_length_size: Option<String>,
    #[serde(default)]
    chroma_location: Option<String>,
    #[serde(default)]
    bits_per_raw_sample: Option<String>,
    #[serde(default)]
    has_b_frames: Option<u32>,
    #[serde(default)]
    r_frame_rate: Option<String>,
    #[serde(default)]
    avg_frame_rate: Option<String>,
    #[serde(default)]
    pix_fmt: Option<String>,
    #[serde(default)]
    sample_aspect_ratio: Option<String>,
    #[serde(default)]
    display_aspect_ratio: Option<String>,
    #[serde(default)]
    field_order: Option<String>,
    #[serde(default)]
    color_space: Option<String>,
    #[serde(default)]
    color_primaries: Option<String>,
    #[serde(default)]
    color_transfer: Option<String>,
    #[serde(default)]
    color_range: Option<String>,
    channels: Option<u32>,
    #[serde(default)]
    channel_layout: Option<String>,
    #[serde(default)]
    sample_fmt: Option<String>,
    #[serde(default)]
    sample_rate: Option<String>,
    #[serde(default)]
    nb_frames: Option<String>,
    #[serde(default)]
    level: Option<i64>,
    #[serde(default)]
    profile: Option<String>,
    #[serde(default)]
    duration: Option<String>,
    #[serde(default)]
    bit_rate: Option<String>,
    #[serde(default)]
    start_time: Option<String>,
    #[serde(default)]
    tags: Option<std::collections::HashMap<String, String>>,
    #[serde(default)]
    side_data_list: Option<Vec<FfprobeSideData>>,
}

#[derive(Deserialize, Default)]
struct FfprobeSideData {
    #[serde(default)]
    side_data_type: Option<String>,
    #[serde(default)]
    rotation: Option<f64>,
}

#[derive(Deserialize, Default)]
struct FfprobeFormat {
    #[serde(default)]
    duration: Option<String>,
    #[serde(default)]
    size: Option<String>,
    #[serde(default)]
    format_name: Option<String>,
    #[serde(default)]
    bit_rate: Option<String>,
    #[serde(default)]
    nb_programs: Option<u32>,
    #[serde(default)]
    probe_score: Option<u32>,
    #[serde(default)]
    tags: Option<std::collections::HashMap<String, String>>,
}

pub fn probe(path: &Path, timeout: Duration) -> Result<Probe, Error> {
    // URL inputs (rtmp/srt/udp/http/tcp) aren't files — ffprobe opens them
    if !path.to_string_lossy().contains("://") {
        crate::paths::ensure_input(path)?;
    }
    let mut argv = Argv::ffprobe();
    argv.extend([
        "-print_format",
        "json",
        "-show_format",
        "-show_streams",
        "-show_chapters",
        "-show_programs",
        "-v",
        "error",
    ]);
    argv.push(path);
    let spawned = spawn::run(&argv, timeout, false)?;
    let spawned = spawn::require_ok(&argv, spawned)?;
    let raw = spawn::stdout_str(&spawned)?;
    let mut p = parse_ffprobe(raw)?;
    if !path.to_string_lossy().contains("://") {
        p.encrypted = file_has_encryption_atoms(path);
    }
    Ok(p)
}

#[derive(Deserialize)]
struct ChaptersOut {
    #[serde(default)]
    chapters: Vec<ChapterTime>,
}
#[derive(Deserialize)]
struct ChapterTime {
    start_time: Option<String>,
    #[serde(default)]
    tags: std::collections::HashMap<String, String>,
}

/// One embedded chapter mark.
pub struct ChapterMark {
    pub start: f64,
    pub title: Option<String>,
}

/// Chapter marks (time + title) embedded in the container — lectures, courses.
pub fn chapter_marks(path: &Path, timeout: Duration) -> Result<Vec<ChapterMark>, Error> {
    let mut argv = Argv::ffprobe();
    argv.extend(["-print_format", "json", "-show_chapters", "-v", "error"]);
    argv.push(path);
    let spawned = spawn::require_ok(&argv, spawn::run(&argv, timeout, false)?)?;
    let raw = spawn::stdout_str(&spawned)?;
    let out: ChaptersOut = serde_json::from_str(raw)
        .map_err(|e| Error::ffmpeg(format!("ffprobe chapters json: {e}")))?;
    Ok(out
        .chapters
        .iter()
        .filter_map(|c| {
            Some(ChapterMark {
                start: c.start_time.as_deref()?.parse::<f64>().ok()?,
                title: c.tags.get("title").cloned(),
            })
        })
        .collect())
}

/// Chapter boundaries (seconds) embedded in the container — lectures, courses.
pub fn chapters(path: &Path, timeout: Duration) -> Result<Vec<f64>, Error> {
    Ok(chapter_marks(path, timeout)?
        .into_iter()
        .map(|m| m.start)
        .collect())
}

pub fn parse_ffprobe(raw: &str) -> Result<Probe, Error> {
    let parsed: FfprobeOut =
        serde_json::from_str(raw).map_err(|e| Error::ffmpeg(format!("ffprobe json: {e}")))?;

    let video = parsed
        .streams
        .iter()
        .find(|s| s.codec_type == "video" && s.width.is_some())
        .or_else(|| parsed.streams.iter().find(|s| s.codec_type == "video"));
    let audio = parsed.streams.iter().find(|s| s.codec_type == "audio");

    let r_fps = video.and_then(|v| parse_rate(v.r_frame_rate.as_deref()));
    let avg_fps = video.and_then(|v| parse_rate(v.avg_frame_rate.as_deref()));
    let vfr = match (r_fps, avg_fps) {
        (Some(a), Some(b)) => (a - b).abs() > 0.05,
        _ => false,
    };

    let duration = parsed
        .format
        .as_ref()
        .and_then(|f| f.duration.as_deref())
        .and_then(parse_f64)
        .or_else(|| video.and_then(|v| v.duration.as_deref().and_then(parse_f64)))
        .or_else(|| audio.and_then(|v| v.duration.as_deref().and_then(parse_f64)))
        .unwrap_or(0.0);

    // program-less containers (mp4/mov/mkv) report nb_programs 0 —
    // only meaningful on multiplexed ts/spts, so 0 reads as absent
    let program_count = parsed
        .format
        .as_ref()
        .and_then(|f| f.nb_programs)
        .filter(|n| *n > 0);
    let probe_score = parsed.format.as_ref().and_then(|f| f.probe_score);

    let timecode = video
        .and_then(|v| v.tags.as_ref().and_then(|t| t.get("timecode").cloned()))
        .or_else(|| {
            parsed.format.as_ref().and_then(|f| {
                f.tags
                    .as_ref()
                    .and_then(|t| t.get("TIMECODE").or_else(|| t.get("timecode")).cloned())
            })
        });

    let tags = {
        let mut m = serde_json::Map::new();
        if let Some(t) = parsed.format.as_ref().and_then(|f| f.tags.as_ref()) {
            if !t.is_empty() {
                m.insert("format".to_string(), serde_json::json!(t));
            }
        }
        for (i, s) in parsed.streams.iter().enumerate() {
            if let Some(t) = s.tags.as_ref() {
                if !t.is_empty() {
                    let idx = s.index.unwrap_or(i as u32);
                    m.insert(format!("stream:{idx}"), serde_json::json!(t));
                }
            }
        }
        if m.is_empty() {
            None
        } else {
            Some(m)
        }
    };

    Ok(Probe {
        duration,
        width: video.and_then(|v| v.width),
        height: video.and_then(|v| v.height),
        fps: avg_fps.or(r_fps),
        vcodec: video.and_then(|v| v.codec_name.clone()),
        acodec: audio.and_then(|v| v.codec_name.clone()),
        channels: audio.and_then(|v| v.channels),
        sample_rate: audio
            .and_then(|v| v.sample_rate.as_deref())
            .and_then(parse_f64)
            .map(|n| n as u32),
        pix_fmt: video.and_then(|v| v.pix_fmt.clone()),
        has_alpha: video
            .and_then(|v| v.pix_fmt.as_deref())
            .map(pix_fmt_has_alpha)
            .unwrap_or(false),
        color_space: video.and_then(|v| v.color_space.clone()),
        color_primaries: video.and_then(|v| v.color_primaries.clone()),
        color_transfer: video.and_then(|v| v.color_transfer.clone()),
        has_video: video.is_some(),
        has_audio: audio.is_some(),
        has_data: parsed.streams.iter().any(|s| s.codec_type == "data"),
        has_subs: parsed.streams.iter().any(|s| s.codec_type == "subtitle"),
        has_attachment: parsed.streams.iter().any(|s| s.codec_type == "attachment"),
        encrypted: false,
        bit_rate: parsed
            .format
            .as_ref()
            .and_then(|f| f.bit_rate.as_deref())
            .and_then(|v| v.parse().ok()),
        size_bytes: parsed
            .format
            .as_ref()
            .and_then(|f| f.size.as_deref())
            .and_then(|s| s.parse().ok()),
        format: parsed.format.and_then(|f| f.format_name),
        variable_frame_rate_suspected: vfr,
        subtitle_streams: parsed
            .streams
            .iter()
            .filter(|s| s.codec_type == "subtitle")
            .count() as u32,
        attached_pic_indices: parsed
            .streams
            .iter()
            .filter(|s| {
                s.disposition
                    .as_ref()
                    .and_then(|d| d.get("attached_pic").copied())
                    .unwrap_or(0)
                    == 1
            })
            .filter_map(|s| s.index)
            .collect(),
        av_desync_ms: match (
            video
                .and_then(|v| v.start_time.as_deref())
                .and_then(parse_f64),
            audio
                .and_then(|a| a.start_time.as_deref())
                .and_then(parse_f64),
        ) {
            (Some(v), Some(a)) => Some(((v - a).abs() * 1000.0).round()),
            _ => None,
        },
        start_time: parsed
            .streams
            .iter()
            .filter_map(|s| s.start_time.as_deref().and_then(parse_f64))
            .reduce(f64::min),
        timecode,
        tags,
        streams: parsed
            .streams
            .iter()
            .enumerate()
            .map(|(i, s)| ProbeStream {
                index: s.index.unwrap_or(i as u32),
                kind: s.codec_type.clone(),
                codec: s.codec_name.clone(),
                codec_tag: s
                    .codec_tag_string
                    .clone()
                    .filter(|t| t.as_str() != "[0][0][0][0]"),
                language: s.tags.as_ref().and_then(|t| t.get("language").cloned()),
                title: s.tags.as_ref().and_then(|t| t.get("title").cloned()),
                width: s.width,
                height: s.height,
                channels: s.channels,
                channel_layout: s.channel_layout.clone(),
                sample_rate: s.sample_rate.as_deref().and_then(|v| v.parse().ok()),
                profile: s.profile.clone(),
                fps: parse_rate(s.avg_frame_rate.as_deref())
                    .or_else(|| parse_rate(s.r_frame_rate.as_deref())),
                r_fps: parse_rate(s.r_frame_rate.as_deref()),
                duration: s.duration.as_deref().and_then(parse_f64),
                bit_rate: s.bit_rate.as_deref().and_then(|v| v.parse().ok()),
                pix_fmt: s.pix_fmt.clone(),
                sar: s
                    .sample_aspect_ratio
                    .clone()
                    .filter(|r| r.as_str() != "0:1"),
                dar: s
                    .display_aspect_ratio
                    .clone()
                    .filter(|r| r.as_str() != "0:1"),
                field_order: s.field_order.clone().filter(|f| f.as_str() != "unknown"),
                color_space: s.color_space.clone(),
                color_primaries: s.color_primaries.clone(),
                color_transfer: s.color_transfer.clone(),
                color_range: s.color_range.clone().filter(|r| r.as_str() != "unknown"),
                sample_fmt: s.sample_fmt.clone(),
                start_time: s.start_time.as_deref().and_then(parse_f64),
                default: s
                    .disposition
                    .as_ref()
                    .and_then(|d| d.get("default").copied())
                    .unwrap_or(0)
                    == 1,
                nb_frames: s.nb_frames.as_deref().and_then(|v| v.parse().ok()),
                level: s.level.and_then(|l| u32::try_from(l).ok()),
                coded_width: s.coded_width,
                coded_height: s.coded_height,
                stream_id: s.id.clone(),
                is_avc: s.is_avc.as_deref().map(|v| v == "true"),
                nal_length_size: s.nal_length_size.as_deref().and_then(|v| v.parse().ok()),
                chroma_location: s.chroma_location.clone(),
                bits_per_raw_sample: s
                    .bits_per_raw_sample
                    .as_deref()
                    .and_then(|v| v.parse().ok()),
                has_b_frames: s.has_b_frames,
                forced: s
                    .disposition
                    .as_ref()
                    .and_then(|d| d.get("forced").copied())
                    .unwrap_or(0)
                    == 1,
                hearing_impaired: s
                    .disposition
                    .as_ref()
                    .and_then(|d| d.get("hearing_impaired").copied())
                    .unwrap_or(0)
                    == 1,
                comment: s
                    .disposition
                    .as_ref()
                    .and_then(|d| d.get("comment").copied())
                    .unwrap_or(0)
                    == 1,
                visual_impaired: s
                    .disposition
                    .as_ref()
                    .and_then(|d| d.get("visual_impaired").copied())
                    .unwrap_or(0)
                    == 1,
                dub: s
                    .disposition
                    .as_ref()
                    .and_then(|d| d.get("dub").copied())
                    .unwrap_or(0)
                    == 1,
                original: s
                    .disposition
                    .as_ref()
                    .and_then(|d| d.get("original").copied())
                    .unwrap_or(0)
                    == 1,
                attached_pic: s
                    .disposition
                    .as_ref()
                    .and_then(|d| d.get("attached_pic").copied())
                    .unwrap_or(0)
                    == 1,
            })
            .collect(),
        rotation: video.and_then(|v| {
            v.side_data_list.as_ref().and_then(|l| {
                l.iter()
                    .find(|sd| {
                        sd.side_data_type
                            .as_deref()
                            .map(|t| t.contains("Display Matrix"))
                            .unwrap_or(false)
                    })
                    .and_then(|sd| sd.rotation)
            })
        }),
        chapter_count: parsed.chapters.len() as u32,
        program_count,
        probe_score,
        programs: parsed
            .programs
            .iter()
            .filter_map(|p| {
                p.program_num.map(|num| ProbeProgram {
                    num,
                    service_name: p.tags.as_ref().and_then(|t| t.get("service_name").cloned()),
                    streams: p.streams.iter().filter_map(|s| s.index).collect(),
                })
            })
            .collect(),
    })
}

/// Alpha-carrying pixel formats, with the bit-depth/endianness suffix
/// stripped ("rgba64le" -> "rgba", "yuva420p10le" -> "yuva420p").
fn pix_fmt_has_alpha(pf: &str) -> bool {
    let base = pf.trim_end_matches(|c: char| c.is_ascii_digit() || matches!(c, 'l' | 'e' | 'b'));
    base.starts_with("yuva")
        || base.starts_with("gbrap")
        || matches!(
            base,
            "rgba" | "bgra" | "argb" | "abgr" | "ya" | "ayuv" | "vuya" | "vuyx"
        )
}

fn parse_rate(s: Option<&str>) -> Option<f64> {
    let s = s?;
    if s == "0/0" || s.is_empty() {
        return None;
    }
    if let Some((a, b)) = s.split_once('/') {
        let a: f64 = a.parse().ok()?;
        let b: f64 = b.parse().ok()?;
        if b == 0.0 {
            return None;
        }
        Some(a / b)
    } else {
        s.parse().ok()
    }
}

fn parse_f64(s: &str) -> Option<f64> {
    s.parse().ok()
}

/// Keyframe packet map of the first video stream: (total packet count,
/// [(packet index, dts_time)]). No decode — packet flags only, cheap even
/// on long masters. Empty on audio-only inputs or when ffprobe fails.
pub fn keyframe_packets(path: &Path, timeout: Duration) -> (usize, Vec<(usize, f64)>) {
    let mut argv = crate::spawn::Argv::ffprobe();
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
    argv.push(path);
    let mut packet_count = 0usize;
    let mut keys: Vec<(usize, f64)> = Vec::new();
    if let Ok(sp) = crate::spawn::run(&argv, timeout, false) {
        for (i, line) in String::from_utf8_lossy(&sp.stdout).lines().enumerate() {
            packet_count += 1;
            let mut f = line.trim().split(',');
            let dts: f64 = f.next().and_then(|v| v.parse().ok()).unwrap_or(0.0);
            if f.next().is_some_and(|fl| fl.contains('K')) {
                keys.push((i, dts));
            }
        }
    }
    (packet_count, keys)
}

/// CENC detection: ffprobe doesn't surface encryption, so scan the moov
/// (head or tail of file) for the protection boxes the muxers write —
/// `sinf` protection-info or the encrypted sample entries `encv`/`enca`/`schi`.
fn file_has_encryption_atoms(path: &Path) -> bool {
    use std::io::{Read, Seek, SeekFrom};
    const SLAB: usize = 1_048_576;
    let mut f = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return false,
    };
    let mut buf = vec![0u8; SLAB];
    let n = f.read(&mut buf).unwrap_or(0);
    buf.truncate(n);
    if let Ok(m) = f.metadata() {
        if m.len() > SLAB as u64 {
            let mut tail = vec![0u8; SLAB];
            if f.seek(SeekFrom::End(-(SLAB as i64))).is_ok() {
                let n = f.read(&mut tail).unwrap_or(0);
                tail.truncate(n);
                buf.extend_from_slice(&tail);
            }
        }
    }
    [b"sinf", b"encv", b"enca", b"schi"]
        .iter()
        .any(|t| buf.windows(4).any(|w| w == t.as_slice()))
}

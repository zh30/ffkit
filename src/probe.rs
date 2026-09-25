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
    /// Attached-picture track (muxed cover art — QC that `remux --cover`
    /// landed and which stream index carries it)
    #[serde(default, skip_serializing_if = "is_false")]
    pub attached_pic: bool,
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
}

#[derive(Deserialize)]
struct FfprobeOut {
    #[serde(default)]
    streams: Vec<FfprobeStream>,
    #[serde(default)]
    format: Option<FfprobeFormat>,
    #[serde(default)]
    chapters: Vec<ChapterTime>,
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
    width: Option<u32>,
    height: Option<u32>,
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
        "-v",
        "error",
    ]);
    argv.push(path);
    let spawned = spawn::run(&argv, timeout, false)?;
    let spawned = spawn::require_ok(&argv, spawned)?;
    let raw = spawn::stdout_str(&spawned)?;
    parse_ffprobe(raw)
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
                forced: s
                    .disposition
                    .as_ref()
                    .and_then(|d| d.get("forced").copied())
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

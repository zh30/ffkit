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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channels: Option<u32>,
    /// Player-default track (disposition.default) — QC which track a
    /// player picks before `remux --default-audio`/`--default-sub`.
    #[serde(default, skip_serializing_if = "is_false")]
    pub default: bool,
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
    color_space: Option<String>,
    #[serde(default)]
    color_primaries: Option<String>,
    #[serde(default)]
    color_transfer: Option<String>,
    channels: Option<u32>,
    #[serde(default)]
    sample_rate: Option<String>,
    #[serde(default)]
    duration: Option<String>,
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
                width: s.width,
                height: s.height,
                channels: s.channels,
                default: s
                    .disposition
                    .as_ref()
                    .and_then(|d| d.get("default").copied())
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

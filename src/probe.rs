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
    pub has_video: bool,
    pub has_audio: bool,
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
    channels: Option<u32>,
    #[serde(default)]
    sample_rate: Option<String>,
    #[serde(default)]
    duration: Option<String>,
}

#[derive(Deserialize, Default)]
struct FfprobeFormat {
    #[serde(default)]
    duration: Option<String>,
    #[serde(default)]
    size: Option<String>,
    #[serde(default)]
    format_name: Option<String>,
}

pub fn probe(path: &Path, timeout: Duration) -> Result<Probe, Error> {
    crate::paths::ensure_input(path)?;
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
    })
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

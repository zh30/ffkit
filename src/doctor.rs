use std::collections::BTreeSet;
use std::time::Duration;

use serde::Serialize;
use serde_json::json;

use crate::cli::Globals;
use crate::contract::Contract;
use crate::error::Error;
use crate::spawn::{self, Argv};

const KEY_ENCODERS: &[&str] = &[
    "libx264",
    "libx265",
    "aac",
    "libmp3lame",
    "libvpx-vp9",
    "libaom-av1",
    "gif",
    "png",
    "prores",
    "flac",
    "libopus",
];
const KEY_FILTERS: &[&str] = &[
    "scale",
    "overlay",
    "subtitles",
    "loudnorm",
    "concat",
    "tile",
    "palettegen",
    "paletteuse",
    "xfade",
    "lut3d",
    "acrossfade",
    "crop",
    "pad",
    "fps",
    "format",
    "setsar",
    "hflip",
    "vflip",
    "transpose",
    "drawtext",
    "thumbnail",
    "afwtdn",
    "hqdn3d",
    "gblur",
    "showwaves",
    "colorkey",
];

#[derive(Serialize, Clone)]
struct ToolInfo {
    path: Option<String>,
    version: Option<String>,
    usable: bool,
}

pub fn run(_g: &Globals) -> Result<Contract, Error> {
    let ffmpeg = inspect("ffmpeg");
    let ffprobe = inspect("ffprobe");

    let encoders = if ffmpeg.usable {
        present_tokens("ffmpeg", &["-encoders"], KEY_ENCODERS)
    } else {
        BTreeSet::new()
    };
    let filters = if ffmpeg.usable {
        present_tokens("ffmpeg", &["-filters"], KEY_FILTERS)
    } else {
        BTreeSet::new()
    };

    let versions = crate::version::report();
    let extra = json!({
        "ffmpeg": ffmpeg.clone(),
        "ffprobe": ffprobe.clone(),
        "encoders": encoders,
        "filters": filters,
        "ffkit": crate::version::doctor_ffkit_json(&versions),
        "installed_skills": versions.installed,
    });

    if !ffmpeg.usable || !ffprobe.usable {
        let err = Error::missing_tool(
            "ffmpeg/ffprobe not found on PATH; install ffmpeg (macOS: brew install ffmpeg)",
        );
        return Ok(Contract::failed("doctor", &err).with_extra(extra));
    }

    let mut c = Contract::ok("doctor", None, None).with_extra(extra);
    let mut summary = format!(
        "ffmpeg {}  {}",
        ffmpeg.version.as_deref().unwrap_or("?"),
        crate::version::summary(&versions)
    );
    if versions.stale > 0 {
        summary.push_str("; run ffkit install-skill");
    }
    c.summary = Some(summary);
    Ok(c)
}

fn inspect(program: &str) -> ToolInfo {
    let argv = Argv {
        program: program.into(),
        args: vec!["-version".into()],
    };
    match spawn::run(&argv, Duration::from_secs(10), false) {
        Ok(out) if out.status_ok => {
            let text = String::from_utf8_lossy(&out.stdout);
            let version = text
                .lines()
                .next()
                .and_then(|l| l.split_whitespace().nth(2))
                .map(str::to_string);
            ToolInfo {
                path: which(program),
                version,
                usable: true,
            }
        }
        Ok(_) | Err(_) => ToolInfo {
            path: which(program),
            version: None,
            usable: false,
        },
    }
}

fn which(program: &str) -> Option<String> {
    let Ok(path) = std::env::var("PATH") else {
        return None;
    };
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(program);
        if candidate.is_file() {
            return Some(candidate.display().to_string());
        }
    }
    None
}

fn present_tokens(program: &str, args: &[&str], keys: &[&str]) -> BTreeSet<String> {
    let argv = Argv {
        program: program.into(),
        args: args.iter().map(|s| (*s).into()).collect(),
    };
    let Ok(out) = spawn::run(&argv, Duration::from_secs(15), false) else {
        return BTreeSet::new();
    };
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    keys.iter()
        .filter(|k| text.split_whitespace().any(|t| t == **k))
        .map(|s| (*s).to_string())
        .collect()
}

pub fn list_filters() -> Result<BTreeSet<String>, Error> {
    let mut argv = Argv::ffmpeg();
    argv.push("-filters");
    let spawned = spawn::run(&argv, Duration::from_secs(15), false)?;
    let spawned = spawn::require_ok(&argv, spawned)?;
    let text = spawn::stdout_str(&spawned)?;
    let mut names = BTreeSet::new();
    for line in text.lines() {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() >= 3 && cols[2].contains("->") {
            names.insert(cols[1].to_string());
        }
    }
    Ok(names)
}

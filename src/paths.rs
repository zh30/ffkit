use std::path::{Path, PathBuf};

use crate::error::Error;

pub fn ensure_input(path: &Path) -> Result<(), Error> {
    if !path.exists() {
        return Err(Error::input(format!("input not found: {}", path.display())));
    }
    if !path.is_file() {
        return Err(Error::input(format!(
            "input is not a file: {}",
            path.display()
        )));
    }
    Ok(())
}

pub fn abs(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .unwrap_or_else(|_| path.to_path_buf())
    }
}

pub fn same_file(a: &Path, b: &Path) -> bool {
    let a = abs(a);
    let b = abs(b);
    if let (Ok(ca), Ok(cb)) = (a.canonicalize(), b.canonicalize()) {
        return ca == cb;
    }
    a == b
}

pub fn ensure_output_allowed(
    output: &Path,
    inputs: &[&Path],
    overwrite: bool,
) -> Result<(), Error> {
    for input in inputs {
        if same_file(output, input) {
            return Err(Error::input(format!(
                "refusing to overwrite source: {}",
                input.display()
            )));
        }
    }
    if output.exists() && !overwrite {
        return Err(Error::input(format!(
            "output exists (pass --overwrite to replace): {}",
            output.display()
        )));
    }
    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            std::fs::create_dir_all(parent)
                .map_err(|e| Error::output(format!("cannot create {}: {e}", parent.display())))?;
        }
    }
    Ok(())
}

pub fn display(path: &Path) -> String {
    path.display().to_string()
}

pub const MEDIA_EXTS: &[&str] = &[
    "mp4", "mov", "mkv", "webm", "avi", "m4v", "wav", "m4a", "mp3", "aac", "flac", "ogg", "opus",
    "gif", "png", "jpg", "jpeg", "srt", "ass", "vtt",
];

pub fn is_media(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| MEDIA_EXTS.iter().any(|x| x.eq_ignore_ascii_case(e)))
        .unwrap_or(false)
}

pub fn even(n: u32) -> u32 {
    n & !1
}

use std::path::{Path, PathBuf};

use crate::error::Error;

const CANDIDATES: &[&str] = &[
    "/System/Library/Fonts/Supplemental/Arial.ttf",
    "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
    "/usr/share/fonts/TTF/DejaVuSans.ttf",
];

pub fn resolve(explicit: Option<&Path>) -> Result<PathBuf, Error> {
    if let Some(p) = explicit {
        if p.is_file() {
            return Ok(p.to_path_buf());
        }
        return Err(Error::input(format!("font not found: {}", p.display())));
    }
    for c in CANDIDATES {
        let p = Path::new(c);
        if p.is_file() {
            return Ok(p.to_path_buf());
        }
    }
    Err(Error::missing_tool(
        "no TTF font found for caption burn; pass --font PATH (Arial/DejaVu/Liberation)",
    ))
}

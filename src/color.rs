use crate::error::Error;

/// Common color names creators actually type (subset of ffmpeg/CSS names).
const NAMES: &[(&str, &str)] = &[
    ("black", "000000"),
    ("white", "ffffff"),
    ("red", "ff0000"),
    ("green", "00ff00"),
    ("blue", "0000ff"),
    ("yellow", "ffff00"),
    ("cyan", "00ffff"),
    ("magenta", "ff00ff"),
    ("gray", "808080"),
    ("grey", "808080"),
    ("silver", "c0c0c0"),
    ("maroon", "800000"),
    ("olive", "808000"),
    ("lime", "00ff00"),
    ("aqua", "00ffff"),
    ("teal", "008080"),
    ("navy", "000080"),
    ("purple", "800080"),
    ("fuchsia", "ff00ff"),
    ("orange", "ffa500"),
    ("pink", "ffc0cb"),
    ("brown", "a52a2a"),
    ("gold", "ffd700"),
];

fn is_hex(s: &str) -> bool {
    matches!(s.len(), 3 | 6 | 8) && s.chars().all(|c| c.is_ascii_hexdigit())
}

/// Normalize a user color string for lavfi filter args (`color=c=`, `pad=`,
/// `colors=`). Hex becomes `0xRRGGBB`; names pass through as ffmpeg colors.
pub fn lavfi(input: &str) -> String {
    let s = input
        .trim()
        .trim_start_matches("0x")
        .trim_start_matches('#');
    if is_hex(s) {
        format!("0x{s}")
    } else {
        // ffmpeg named colors are alnum (+ optional @alpha); anything else is
        // passed through verbatim so `lavfi("red")` == `lavfi("Red")`.
        s.to_string()
    }
}

/// Resolve a color to RGB for the raster text pipeline. Accepts hex
/// (3/6/8 digits, optional 0x/#) and the names in `NAMES`.
pub fn rgb(input: &str) -> Result<[u8; 3], Error> {
    let s = input.trim();
    let lower = s.to_lowercase();
    if let Some((_, hex)) = NAMES.iter().find(|(n, _)| *n == lower) {
        return rgb(hex);
    }
    let h = s.trim_start_matches("0x").trim_start_matches('#');
    let h = if h.len() == 3 {
        h.chars().map(|c| format!("{c}{c}")).collect::<String>()
    } else {
        h.to_string()
    };
    if h.len() < 6 || !h.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(Error::input(format!(
            "color '{input}' must be RRGGBB hex or a name like 'red'"
        )));
    }
    let b = |i: usize| {
        u8::from_str_radix(&h[i..i + 2], 16)
            .map_err(|_| Error::input(format!("color '{input}' must be RRGGBB hex")))
    };
    Ok([b(0)?, b(2)?, b(4)?])
}

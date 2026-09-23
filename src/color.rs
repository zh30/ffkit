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

/// (hue degrees, saturation 0..1) of a user color — for filters that take
/// HSL parameters (colorize).
pub fn hsl(input: &str) -> Result<(f64, f64), Error> {
    let [r, g, b] = rgb(input)?;
    let (r, g, b) = (r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0);
    let (max, min) = (r.max(g).max(b), r.min(g).min(b));
    let d = max - min;
    let sat = if max <= 0.0 { 0.0 } else { d / max };
    if d <= 0.0 {
        return Ok((0.0, sat));
    }
    let mut h = if max == r {
        ((g - b) / d) % 6.0
    } else if max == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    h *= 60.0;
    if h < 0.0 {
        h += 360.0;
    }
    Ok((h, sat))
}

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

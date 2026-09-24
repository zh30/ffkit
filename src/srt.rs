use crate::error::Error;
use crate::time::parse_time;

#[derive(Debug, Clone)]
pub struct Cue {
    pub start: f64,
    pub end: f64,
    pub text: String,
}

pub fn parse_srt(raw: &str) -> Result<Vec<Cue>, Error> {
    let mut cues = Vec::new();
    let blocks = raw.replace("\r\n", "\n").replace('\r', "\n");
    for block in blocks.split("\n\n") {
        let block = block.trim();
        if block.is_empty() {
            continue;
        }
        let mut lines = block.lines();
        let first = lines.next().unwrap_or("");
        let timing = if first.contains("-->") {
            first
        } else {
            lines.next().unwrap_or("")
        };
        let Some((start_s, end_s)) = timing.split_once("-->") else {
            continue;
        };
        let start = parse_srt_ts(start_s.trim())?;
        let end = parse_srt_ts(end_s.trim())?;
        if end <= start {
            return Err(Error::input(format!(
                "srt cue end must be after start ({start} >= {end})"
            )));
        }
        let text = lines
            .map(strip_tags)
            .filter(|l| !l.is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        if text.is_empty() {
            continue;
        }
        cues.push(Cue { start, end, text });
    }
    if cues.is_empty() {
        return Err(Error::input("srt file has no cues"));
    }
    Ok(cues)
}

/// Remove inline markup a cue may carry: HTML-ish `<i>`/`<b>`/
/// `<font …>` spans and ASS `{\…}` override blocks.
pub fn strip_markup(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '<' => {
                for c in chars.by_ref() {
                    if c == '>' {
                        break;
                    }
                }
            }
            '{' => {
                for c in chars.by_ref() {
                    if c == '}' {
                        break;
                    }
                }
            }
            _ => out.push(ch),
        }
    }
    out.split('\n')
        .map(|l| l.trim_end().to_string())
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

pub fn to_srt(cues: &[Cue]) -> String {
    let mut out = String::new();
    for (i, c) in cues.iter().enumerate() {
        out.push_str(&format!(
            "{}\n{} --> {}\n{}\n\n",
            i + 1,
            fmt_srt_ts(c.start),
            fmt_srt_ts(c.end),
            c.text
        ));
    }
    out
}

fn fmt_srt_ts(secs: f64) -> String {
    let ms = (secs.max(0.0) * 1000.0).round() as u64;
    let h = ms / 3_600_000;
    let m = (ms / 60_000) % 60;
    let s = (ms / 1000) % 60;
    let msec = ms % 1000;
    format!("{h:02}:{m:02}:{s:02},{msec:03}")
}

fn parse_srt_ts(s: &str) -> Result<f64, Error> {
    parse_time(&s.replace(',', "."))
}

fn strip_tags(s: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_cue() {
        let cues = parse_srt("1\n00:00:00,000 --> 00:00:01,000\nHELLO\n").unwrap();
        assert_eq!(cues.len(), 1);
        assert_eq!(cues[0].text, "HELLO");
        assert!((cues[0].end - 1.0).abs() < 0.001);
    }
}

use crate::error::Error;

/// Parse a timestamp: seconds (`12.5`), `mm:ss`, or `hh:mm:ss` with optional fractional part.
pub fn parse_time(s: &str) -> Result<f64, Error> {
    let s = s.trim();
    if s.is_empty() {
        return Err(Error::input("empty timestamp"));
    }
    if !s.contains(':') {
        let v: f64 = s
            .parse()
            .map_err(|_| Error::input(format!("invalid timestamp: {s}")))?;
        if !v.is_finite() || v < 0.0 {
            return Err(Error::input(format!("invalid timestamp: {s}")));
        }
        return Ok(v);
    }
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() < 2 || parts.len() > 3 {
        return Err(Error::input(format!("invalid timestamp: {s}")));
    }
    let mut vals = Vec::with_capacity(parts.len());
    for p in &parts {
        let v: f64 = p
            .parse()
            .map_err(|_| Error::input(format!("invalid timestamp: {s}")))?;
        if !v.is_finite() || v < 0.0 {
            return Err(Error::input(format!("invalid timestamp: {s}")));
        }
        vals.push(v);
    }
    let seconds = if vals.len() == 2 {
        vals[0] * 60.0 + vals[1]
    } else {
        vals[0] * 3600.0 + vals[1] * 60.0 + vals[2]
    };
    Ok(seconds)
}

pub fn fmt_time(seconds: f64) -> String {
    format!("{seconds:.3}")
}

/// Resolve a `--at` value: a timestamp, or `end` meaning the tail of the
/// media — `duration - dur` (`--dur` is required when passing `end`).
pub fn resolve_at(s: &str, dur: Option<f64>, duration: f64) -> Result<f64, Error> {
    if s.trim().eq_ignore_ascii_case("end") {
        let d = dur.ok_or_else(|| Error::input("--at end needs --dur"))?;
        if d <= 0.0 || d > duration {
            return Err(Error::input("--dur is outside the input"));
        }
        return Ok(duration - d);
    }
    parse_time(s)
}

/// Resolve a (possibly comma-separated) `--at` list into `(start, end)` windows.
/// A comma list requires `--dur`; each entry may use `end`. Ends clamp to the
/// media duration.
pub fn enable_windows(at: &str, dur: Option<f64>, duration: f64) -> Result<Vec<(f64, f64)>, Error> {
    if at.contains(',') && dur.is_none() {
        return Err(Error::input("a comma list of --at times needs --dur"));
    }
    let mut out = Vec::new();
    for part in at.split(',') {
        let s = resolve_at(part.trim(), dur, duration)?;
        if !(0.0..duration).contains(&s) {
            return Err(Error::input("--at is outside the input"));
        }
        let e = match dur {
            Some(d) => (s + d).min(duration),
            None => duration,
        };
        out.push((s, e));
    }
    Ok(out)
}

/// Sorted, merged `(start, end)` windows for trim/concat graphs (`speed`,
/// `tempo`, `zoom`): a comma `--at` list resolves per entry, then overlapping
/// or touching windows merge into one (back-to-back FX is one segment).
pub fn window_list(at: &str, dur: Option<f64>, duration: f64) -> Result<Vec<(f64, f64)>, Error> {
    let mut w = enable_windows(at, dur, duration)?;
    w.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    let mut merged: Vec<(f64, f64)> = Vec::new();
    for (s, e) in w {
        match merged.last_mut() {
            Some(prev) if s <= prev.1 + 1e-6 => prev.1 = prev.1.max(e),
            _ => merged.push((s, e)),
        }
    }
    Ok(merged)
}

/// `enable='...'` predicate for `--at`/`--dur`: `gte(t,s)` when the window runs
/// to the tail, otherwise OR'd `between(t,s,e)` terms (comma list = several).
pub fn enable_expr(at: &str, dur: Option<f64>, duration: f64) -> Result<String, Error> {
    let w = enable_windows(at, dur, duration)?;
    if w.len() == 1 && (dur.is_none() || w[0].0 + dur.unwrap() >= duration) {
        return Ok(format!("gte(t,{:.3})", w[0].0));
    }
    Ok(w.iter()
        .map(|(s, e)| format!("between(t,{s:.3},{e:.3})"))
        .collect::<Vec<_>>()
        .join("+"))
}

/// Resolve a `--at` for a still-frame grab: a timestamp, or `end` meaning
/// the last frame — a small epsilon before the media's tail.
pub fn resolve_frame_at(s: &str, duration: f64) -> Result<f64, Error> {
    if s.trim().eq_ignore_ascii_case("end") {
        return Ok((duration - 0.05).max(0.0));
    }
    parse_time(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seconds() {
        assert_eq!(parse_time("1.5").unwrap(), 1.5);
        assert_eq!(parse_time("0").unwrap(), 0.0);
    }

    #[test]
    fn mmss() {
        assert_eq!(parse_time("1:20").unwrap(), 80.0);
        assert_eq!(parse_time("1:20.5").unwrap(), 80.5);
    }

    #[test]
    fn hhmmss() {
        assert_eq!(parse_time("1:02:03").unwrap(), 3723.0);
    }

    #[test]
    fn rejects_negative() {
        assert!(parse_time("-1").is_err());
    }
}

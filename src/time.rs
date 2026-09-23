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

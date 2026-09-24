//! `align` — auto-sync a second recording to a reference by audio
//! cross-correlation (multi-cam shoots, external recorders, phone + camera).
//! Shifts the target's audio onto the reference timeline and reports the
//! detected offset in ms; use `sync --ms` to nudge by hand afterwards.

use std::path::Path;
use std::process::{Command, Stdio};

use serde_json::json;

use crate::cli::{AlignArgs, Globals};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;

pub(crate) const SAMPLE_RATE: u32 = 16_000;
const ANALYZE_SECS: f64 = 45.0;

/// Decode `path` to mono s16 PCM at SAMPLE_RATE, truncated to ANALYZE_SECS.
pub(crate) fn pcm(path: &Path, window: Option<f64>) -> Result<Vec<f32>, Error> {
    let mut a = vec![
        "-hide_banner".to_string(),
        "-nostdin".to_string(),
        "-v".to_string(),
        "error".to_string(),
        "-i".to_string(),
        path.display().to_string(),
    ];
    if let Some(w) = window {
        a.extend(["-t".to_string(), format!("{w:.3}")]);
    }
    a.extend([
        "-vn".to_string(),
        "-ac".to_string(),
        "1".to_string(),
        "-ar".to_string(),
        "16000".to_string(),
        "-f".to_string(),
        "s16le".to_string(),
        "-".to_string(),
    ]);
    let out = Command::new("ffmpeg")
        .args(&a)
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .output()
        .map_err(|e| Error::input(format!("probe pcm decode: {e}")))?;
    if !out.status.success() {
        return Err(Error::input(format!(
            "couldn't decode audio from {}: {}",
            path.display(),
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    let cap = (ANALYZE_SECS * SAMPLE_RATE as f64) as usize;
    let mut v = Vec::with_capacity(out.stdout.len() / 2);
    for ch in out.stdout.chunks_exact(2) {
        v.push(i16::from_le_bytes([ch[0], ch[1]]) as f32 / 32768.0);
    }
    v.truncate(cap);
    Ok(v)
}

#[derive(Clone, Copy, Default)]
struct Cx {
    re: f64,
    im: f64,
}

/// Iterative radix-2 FFT, in place. `n` must be a power of two.
fn fft(a: &mut [Cx], inverse: bool) {
    let n = a.len();
    let mut j = 0usize;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;
        if i < j {
            a.swap(i, j);
        }
    }
    let mut len = 2;
    while len <= n {
        let ang = -2.0 * std::f64::consts::PI / len as f64 * if inverse { -1.0 } else { 1.0 };
        let (wre, wim) = (ang.cos(), ang.sin());
        let mut i = 0;
        while i < n {
            let mut cur = Cx { re: 1.0, im: 0.0 };
            for k in 0..len / 2 {
                let u = a[i + k];
                let v = a[i + k + len / 2];
                let t = Cx {
                    re: v.re * cur.re - v.im * cur.im,
                    im: v.re * cur.im + v.im * cur.re,
                };
                a[i + k] = Cx {
                    re: u.re + t.re,
                    im: u.im + t.im,
                };
                a[i + k + len / 2] = Cx {
                    re: u.re - t.re,
                    im: u.im - t.im,
                };
                cur = Cx {
                    re: cur.re * wre - cur.im * wim,
                    im: cur.re * wim + cur.im * wre,
                };
            }
            i += len;
        }
        len <<= 1;
    }
    if inverse {
        for x in a.iter_mut() {
            x.re /= n as f64;
            x.im /= n as f64;
        }
    }
}

/// Lag of `b` relative to `a` in samples: positive = b's content starts later.
pub(crate) fn detect_lag(a: &[f32], b: &[f32], max_lag: usize) -> Option<i64> {
    if a.len() < 4_000 || b.len() < 4_000 {
        return None;
    }
    let mut n = 1usize;
    while n < a.len() + b.len() {
        n <<= 1;
    }
    let mut fa: Vec<Cx> = a
        .iter()
        .map(|&x| Cx {
            re: x as f64,
            im: 0.0,
        })
        .collect();
    fa.resize(n, Cx::default());
    let mut fb: Vec<Cx> = b
        .iter()
        .map(|&x| Cx {
            re: x as f64,
            im: 0.0,
        })
        .collect();
    fb.resize(n, Cx::default());
    fft(&mut fa, false);
    fft(&mut fb, false);
    // r[k] = sum_i a[i+k] * b[i]  (positive k = a delayed relative to b)
    for i in 0..n {
        let (u, v) = (fa[i], fb[i]);
        fa[i] = Cx {
            re: u.re * v.re + u.im * v.im,
            im: u.im * v.re - u.re * v.im,
        };
    }
    fft(&mut fa, true);
    let k_max = max_lag.min(n / 2 - 1);
    let mut best: Option<(i64, f64)> = None;
    for k in -(k_max as i64)..=(k_max as i64) {
        let idx = if k >= 0 {
            k as usize
        } else {
            n - (-k) as usize
        };
        let v = fa[idx].re;
        // r[k]>0 peak at k = -d when b lags a by d → d = -k
        if best.map(|(_, bv)| v > bv).unwrap_or(true) {
            best = Some((-k, v));
        }
    }
    best.map(|(d, _)| d)
}

pub fn run(args: AlignArgs, g: &Globals) -> Result<Contract, Error> {
    let pa = engine::probe_or_err(&args.reference, g)?;
    let pb = engine::probe_or_err(&args.target, g)?;
    if !pa.has_audio || !pb.has_audio {
        return Err(Error::input("align needs an audio track in both inputs"));
    }
    if !(1.0..=60.0).contains(&args.max_lag) {
        return Err(Error::input("--max-lag must be 1..=60 seconds"));
    }

    let window = args.window.filter(|w| *w > 0.0);
    let (a, b) = (pcm(&args.reference, window)?, pcm(&args.target, window)?);
    let lag = detect_lag(&a, &b, (args.max_lag * SAMPLE_RATE as f64) as usize)
        .ok_or_else(|| Error::input("no usable audio to align (too short)"))?;
    let ms = lag as f64 * 1000.0 / SAMPLE_RATE as f64;

    // --check: QC only — report the offset, skip the render.
    if args.check {
        let mut c = Contract::dry_run("align", None, Some(pb.clone()));
        c = c.with_extra(json!({
            "offset_ms": (ms * 10.0).round() / 10.0,
            "direction": if ms.abs() < 1.0 { "in_sync" } else if ms > 0.0 { "target_late" } else { "target_early" },
            "window": window,
        }));
        return Ok(c);
    }
    let output = args
        .output
        .as_deref()
        .ok_or_else(|| Error::input("align needs -o (or --check to probe only)"))?;

    // lag > 0: target starts later → trim its head. lag < 0: pad its start.
    let af = if ms > 0.0 {
        format!("atrim=start={:.3},asetpts=PTS-STARTPTS", ms / 1000.0)
    } else {
        format!("adelay={:.0}:all=1", -ms)
    };

    let mut argv = ffmpeg_base(g.progress);
    argv.extend(["-i".to_string(), args.target.display().to_string()]);
    if pb.has_video {
        argv.extend([
            "-filter_complex".to_string(),
            format!("[0:a]{af}[aout]"),
            "-map".to_string(),
            "0:v".to_string(),
            "-map".to_string(),
            "[aout]".to_string(),
            "-c:v".to_string(),
            "copy".to_string(),
        ]);
    } else {
        argv.extend(["-af".to_string(), af]);
    }
    argv.extend(["-c:a".to_string(), "aac".to_string()]);
    argv.push(output.display().to_string());

    let inputs: Vec<&Path> = vec![&args.reference, &args.target];
    let mut c = engine::write_job("align", &inputs, output, vec![argv], g)?;
    c = c.with_extra(json!({ "offset_ms": (ms * 10.0).round() / 10.0, "window": window }));
    Ok(c)
}

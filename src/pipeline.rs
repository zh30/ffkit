use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;
use serde_json::json;

use crate::cli::Globals;
use crate::contract::{Contract, Status};
use crate::error::Error;
use crate::paths;
use crate::probe::Probe;

const BLOCKED: &[&str] = &["pipeline", "batch", "install-skill", "version"];

#[derive(Deserialize)]
struct Plan {
    goal: String,
    #[serde(default)]
    input: Option<String>,
    #[serde(default)]
    expect: Option<Expect>,
    steps: Vec<Step>,
}

#[derive(Deserialize)]
struct Step {
    tool: String,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    argv: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct Expect {
    #[serde(default)]
    aspect: Option<String>,
    #[serde(default)]
    width: Option<u32>,
    #[serde(default)]
    height: Option<u32>,
    #[serde(default)]
    duration_gt: Option<f64>,
    #[serde(default)]
    duration_lt: Option<f64>,
    #[serde(default)]
    has_audio: Option<bool>,
    #[serde(default)]
    has_video: Option<bool>,
    #[serde(default)]
    ext: Option<String>,
    #[serde(default)]
    fps: Option<f64>,
    #[serde(default)]
    vcodec: Option<String>,
    #[serde(default)]
    acodec: Option<String>,
    #[serde(default)]
    pix_fmt: Option<String>,
    #[serde(default)]
    max_mb: Option<f64>,
    #[serde(default)]
    av_drift_max: Option<f64>,
    // measured: decoded by ffmpeg analyzers after the last step
    #[serde(default)]
    lufs: Option<f64>,
    #[serde(default)]
    lufs_tol: Option<f64>,
    #[serde(default)]
    true_peak_max: Option<f64>,
    #[serde(default)]
    max_black: Option<f64>,
    #[serde(default)]
    max_freeze: Option<f64>,
    #[serde(default)]
    ssim_min: Option<f64>,
}

pub fn run(plan_path: PathBuf, g: &Globals) -> Result<Contract, Error> {
    paths::ensure_input(&plan_path)?;
    let raw = std::fs::read_to_string(&plan_path)?;
    let plan: Plan =
        serde_json::from_str(&raw).map_err(|e| Error::input(format!("pipeline: {e}")))?;
    if plan.goal.trim().is_empty() {
        return Err(Error::input("pipeline plan needs a non-empty goal"));
    }
    if plan.steps.is_empty() {
        return Err(Error::input("pipeline plan needs at least one step"));
    }
    if plan.steps.len() > 24 {
        return Err(Error::input("pipeline supports at most 24 steps"));
    }

    let allowed: Vec<String> = crate::verb_names();
    let exe = std::env::current_exe().map_err(|e| Error::output(e.to_string()))?;
    let src = plan.input.as_deref();
    let mut results = Vec::new();
    let mut last_ok: Option<Contract> = None;
    let mut prev_out: Option<String> = None;

    for (i, step) in plan.steps.iter().enumerate() {
        let tool = step.tool.trim();
        if BLOCKED.contains(&tool) {
            return Err(Error::input(format!("pipeline cannot run {tool}")));
        }
        if !allowed.iter().any(|n| n == tool) {
            return Err(Error::input(format!("unknown pipeline tool '{tool}'")));
        }
        let argv = subst_argv(&step.argv, src, prev_out.as_deref(), i)?;
        if tool == "ffmpeg" && !argv.iter().any(|a| a == "--because") {
            return Err(Error::input("pipeline ffmpeg step needs --because in argv"));
        }

        let mut cmd = Command::new(&exe);
        cmd.arg(tool);
        cmd.args(&argv);
        cmd.arg("--json");
        cmd.arg("--timeout").arg(g.timeout.as_secs().to_string());
        if g.overwrite {
            cmd.arg("--overwrite");
        }
        if g.dry_run {
            cmd.arg("--dry-run");
        }
        let out = cmd.output().map_err(|e| Error::output(e.to_string()))?;
        let stdout = String::from_utf8_lossy(&out.stdout);
        let parsed: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap_or_else(|_| {
            json!({
                "status": if out.status.success() { "ok" } else { "failed" },
                "tool": tool,
                "error": { "kind": "ffmpeg", "message": String::from_utf8_lossy(&out.stderr) },
            })
        });
        let status = parsed
            .get("status")
            .and_then(|s| s.as_str())
            .unwrap_or("failed");
        let mut row = json!({
            "index": i,
            "tool": tool,
            "result": parsed.clone(),
        });
        if let Some(label) = step.label.as_deref().filter(|s| !s.is_empty()) {
            row["label"] = json!(label);
        }
        results.push(row);
        if status != "ok" && status != "dry_run" {
            let err = Error::ffmpeg(format!(
                "pipeline step {i} ({tool}) failed: {}",
                parsed
                    .pointer("/error/message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("see extra.steps")
            ));
            return Ok(Contract::failed("pipeline", &err).with_extra(json!({
                "goal": plan.goal,
                "failed_step": i,
                "steps": results,
            })));
        }
        if let Some(p) = parsed.get("output").and_then(|o| o.as_str()) {
            if !p.is_empty() {
                prev_out = Some(p.to_string());
            }
        }
        if let Ok(c) = serde_json::from_value::<WireContract>(parsed) {
            last_ok = Some(c.into_contract());
        }
    }

    let mut c = last_ok.unwrap_or_else(|| Contract::ok("pipeline", None, None));
    c.tool = "pipeline".into();
    c.summary = Some(plan.goal.clone());

    let mut extra = json!({
        "goal": plan.goal,
        "steps": results,
    });
    if let Some(src) = src {
        extra["input"] = json!(src);
    }

    if let Some(expect) = &plan.expect {
        if c.status == Status::Ok {
            // re-probe the file on disk rather than trusting the step's own report
            let fresh = c
                .output
                .as_deref()
                .and_then(|o| crate::probe::probe(Path::new(o), g.timeout).ok());
            let probe = fresh.as_ref().or(c.probe.as_ref());
            let mut checks = check_expect(expect, probe, c.output.as_deref());
            checks.extend(check_signal(expect, probe, c.output.as_deref(), src, g));
            let ok = checks.iter().all(|ch| ch["ok"] == true);
            extra["expect"] = json!({ "ok": ok, "checks": checks });
            c.verified = Some(ok);
        }
    }

    c = c.with_extra(extra);
    Ok(c)
}

fn subst_argv(
    argv: &[String],
    src: Option<&str>,
    prev: Option<&str>,
    step: usize,
) -> Result<Vec<String>, Error> {
    let mut out = Vec::with_capacity(argv.len());
    for a in argv {
        match a.as_str() {
            "$src" => {
                let s = src.ok_or_else(|| Error::input("pipeline $src needs a top-level input"))?;
                out.push(s.to_string());
            }
            "$in" => {
                let p = prev.ok_or_else(|| {
                    Error::input(format!("pipeline step {step}: $in has no previous output"))
                })?;
                out.push(p.to_string());
            }
            _ => out.push(a.clone()),
        }
    }
    Ok(out)
}

fn check_expect(
    expect: &Expect,
    probe: Option<&Probe>,
    output: Option<&str>,
) -> Vec<serde_json::Value> {
    let mut checks = Vec::new();
    let some_probe = probe.is_some();

    if let Some(spec) = expect.aspect.as_deref() {
        let (ok, got) = match probe.and_then(|p| p.width.zip(p.height)) {
            Some((w, h)) => (aspect_matches(w, h, spec), format!("{w}x{h}")),
            None => (
                false,
                if some_probe { "no frame" } else { "no probe" }.into(),
            ),
        };
        checks.push(json!({"key": "aspect", "want": spec, "got": got, "ok": ok}));
    }
    if let Some(w) = expect.width {
        let (ok, got) = match probe.and_then(|p| p.width) {
            Some(gw) => (gw == w, gw.to_string()),
            None => (false, "none".into()),
        };
        checks.push(json!({"key": "width", "want": w, "got": got, "ok": ok}));
    }
    if let Some(h) = expect.height {
        let (ok, got) = match probe.and_then(|p| p.height) {
            Some(gh) => (gh == h, gh.to_string()),
            None => (false, "none".into()),
        };
        checks.push(json!({"key": "height", "want": h, "got": got, "ok": ok}));
    }
    if let Some(min) = expect.duration_gt {
        let (ok, got) = match probe {
            Some(p) => (p.duration > min, format!("{:.3}", p.duration)),
            None => (false, "no probe".into()),
        };
        checks.push(json!({"key": "duration_gt", "want": min, "got": got, "ok": ok}));
    }
    if let Some(max) = expect.duration_lt {
        let (ok, got) = match probe {
            Some(p) => (p.duration < max, format!("{:.3}", p.duration)),
            None => (false, "no probe".into()),
        };
        checks.push(json!({"key": "duration_lt", "want": max, "got": got, "ok": ok}));
    }
    if let Some(want) = expect.has_audio {
        let (ok, got) = match probe {
            Some(p) => (p.has_audio == want, p.has_audio.to_string()),
            None => (false, "no probe".into()),
        };
        checks.push(json!({"key": "has_audio", "want": want, "got": got, "ok": ok}));
    }
    if let Some(want) = expect.has_video {
        let (ok, got) = match probe {
            Some(p) => (p.has_video == want, p.has_video.to_string()),
            None => (false, "no probe".into()),
        };
        checks.push(json!({"key": "has_video", "want": want, "got": got, "ok": ok}));
    }
    if let Some(ext) = expect.ext.as_deref() {
        let want = ext.trim_start_matches('.').to_ascii_lowercase();
        let got = output
            .and_then(|p| std::path::Path::new(p).extension())
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let ok = !got.is_empty() && got == want;
        checks.push(json!({"key": "ext", "want": want, "got": got, "ok": ok}));
    }
    check_static(expect, probe, &mut checks);
    checks
}

fn check_static(expect: &Expect, probe: Option<&Probe>, checks: &mut Vec<serde_json::Value>) {
    let str_check = |key: &str, want: &Option<String>, got: Option<&str>| {
        want.as_deref().map(|w| {
            let got = got.unwrap_or("none");
            json!({"key": key, "want": w, "got": got, "ok": got.eq_ignore_ascii_case(w)})
        })
    };
    if let Some(want) = expect.fps {
        let got = probe.and_then(|p| p.fps);
        // 29.97 vs 30000/1001 — two decimals is what a spec sheet means
        let ok = got.is_some_and(|f| (f - want).abs() < 0.02);
        checks.push(json!({"key": "fps", "want": want, "got": got, "ok": ok}));
    }
    checks.extend(str_check(
        "vcodec",
        &expect.vcodec,
        probe.and_then(|p| p.vcodec.as_deref()),
    ));
    checks.extend(str_check(
        "acodec",
        &expect.acodec,
        probe.and_then(|p| p.acodec.as_deref()),
    ));
    checks.extend(str_check(
        "pix_fmt",
        &expect.pix_fmt,
        probe.and_then(|p| p.pix_fmt.as_deref()),
    ));
    if let Some(max) = expect.max_mb {
        let got = probe.and_then(|p| p.size_bytes).map(|b| b as f64 / 1e6);
        let ok = got.is_some_and(|mb| mb <= max);
        checks.push(json!({"key": "max_mb", "want": max, "got": got.map(round3), "ok": ok}));
    }
    if let Some(max) = expect.av_drift_max {
        let got = probe.and_then(av_drift);
        let ok = got.is_some_and(|d| d <= max);
        let got = got.map_or(json!("no audio+video"), |d| json!(round3(d)));
        checks.push(json!({"key": "av_drift_max", "want": max, "got": got, "ok": ok}));
    }
}

/// Worst gap between the first video and first audio track, at either end.
fn av_drift(p: &Probe) -> Option<f64> {
    let span = |kind: &str| {
        let s = p
            .streams
            .iter()
            .find(|s| s.kind == kind && !p.attached_pic_indices.contains(&s.index))?;
        let start = s.start_time.unwrap_or(0.0);
        Some((start, start + s.duration?))
    };
    let (vs, ve) = span("video")?;
    let (a_s, ae) = span("audio")?;
    Some((vs - a_s).abs().max((ve - ae).abs()))
}

fn round3(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0
}

/// Decoded checks. Each needs a full read of the output, so only the
/// ones `expect` names are run, in one pass (plus one for ssim).
fn check_signal(
    expect: &Expect,
    probe: Option<&Probe>,
    output: Option<&str>,
    src: Option<&str>,
    g: &Globals,
) -> Vec<serde_json::Value> {
    let mut checks = Vec::new();
    let loudness = expect.lufs.is_some() || expect.true_peak_max.is_some();
    let ask = crate::measure::Ask {
        loudness,
        black_min: expect.max_black,
        freeze_min: expect.max_freeze,
    };
    let any = loudness || ask.black_min.is_some() || ask.freeze_min.is_some();
    if any {
        let has_a = probe.is_some_and(|p| p.has_audio);
        let has_v = probe.is_some_and(|p| p.has_video);
        let usable = output.is_some()
            && (!loudness || has_a)
            && (ask.black_min.is_none() && ask.freeze_min.is_none() || has_v);
        let measured = match output {
            Some(o) if usable => {
                crate::measure::signal(Path::new(o), &ask, g.timeout).map_err(|e| e.to_string())
            }
            _ => Err("output lacks the stream this check reads".to_string()),
        };
        match measured {
            Ok(m) => {
                if let Some(want) = expect.lufs {
                    let tol = expect.lufs_tol.unwrap_or(1.0);
                    let ok = m.lufs.is_some_and(|i| (i - want).abs() <= tol);
                    checks.push(
                        json!({"key": "lufs", "want": want, "tol": tol, "got": m.lufs, "ok": ok}),
                    );
                }
                if let Some(max) = expect.true_peak_max {
                    let ok = m.true_peak.is_some_and(|tp| tp <= max);
                    checks.push(
                        json!({"key": "true_peak_max", "want": max, "got": m.true_peak, "ok": ok}),
                    );
                }
                if let Some(min) = expect.max_black {
                    checks.push(json!({"key": "max_black", "want": min, "got": ranges(&m.black), "ok": m.black.is_empty()}));
                }
                if let Some(min) = expect.max_freeze {
                    checks.push(json!({"key": "max_freeze", "want": min, "got": ranges(&m.freeze), "ok": m.freeze.is_empty()}));
                }
            }
            Err(msg) => {
                for (key, on) in [
                    ("lufs", expect.lufs.is_some()),
                    ("true_peak_max", expect.true_peak_max.is_some()),
                    ("max_black", expect.max_black.is_some()),
                    ("max_freeze", expect.max_freeze.is_some()),
                ] {
                    if on {
                        checks.push(json!({"key": key, "got": msg, "ok": false}));
                    }
                }
            }
        }
    }
    if let Some(min) = expect.ssim_min {
        checks.push(check_ssim(min, probe, output, src, g));
    }
    checks
}

fn check_ssim(
    min: f64,
    probe: Option<&Probe>,
    output: Option<&str>,
    src: Option<&str>,
    g: &Globals,
) -> serde_json::Value {
    let fail = |got: &str| json!({"key": "ssim_min", "want": min, "got": got, "ok": false});
    let (Some(out), Some(src)) = (output, src) else {
        return fail("needs a top-level input and an output");
    };
    let Ok(sp) = crate::probe::probe(Path::new(src), g.timeout) else {
        return fail("input does not probe");
    };
    // frame-by-frame comparison is only meaningful on the same timeline
    let out_dur = probe.map_or(0.0, |p| p.duration);
    if (out_dur - sp.duration).abs() > 0.5 {
        return fail(&format!(
            "timeline changed ({:.3}s vs input {:.3}s), not comparable",
            out_dur, sp.duration
        ));
    }
    match crate::measure::ssim(Path::new(out), Path::new(src), g.timeout) {
        Ok(v) => json!({"key": "ssim_min", "want": min, "got": v, "ok": v >= min}),
        Err(e) => fail(&e.to_string()),
    }
}

fn ranges(r: &[(f64, f64)]) -> serde_json::Value {
    json!(r
        .iter()
        .map(|(a, b)| if b.is_finite() {
            format!("{a:.2}-{b:.2}")
        } else {
            format!("{a:.2}-end")
        })
        .collect::<Vec<_>>())
}

fn aspect_matches(w: u32, h: u32, spec: &str) -> bool {
    let Some((aw, ah)) = parse_aspect(spec) else {
        return false;
    };
    if aw == 0 || ah == 0 || h == 0 {
        return false;
    }
    let expect_w = (h as u64 * aw as u64) / ah as u64;
    w.abs_diff(expect_w as u32) <= 2
}

fn parse_aspect(s: &str) -> Option<(u32, u32)> {
    let (a, b) = s.split_once(':')?;
    let aw = a.parse().ok()?;
    let ah = b.parse().ok()?;
    Some((aw, ah))
}

#[derive(serde::Deserialize)]
struct WireContract {
    status: String,
    #[serde(default)]
    tool: String,
    #[serde(default)]
    output: Option<String>,
    #[serde(default)]
    probe: Option<Probe>,
    #[serde(default)]
    summary: Option<String>,
    #[serde(default)]
    verified: Option<bool>,
}

impl WireContract {
    fn into_contract(self) -> Contract {
        let status = match self.status.as_str() {
            "dry_run" => Status::DryRun,
            "failed" => Status::Failed,
            _ => Status::Ok,
        };
        Contract {
            status,
            tool: self.tool,
            output: self.output,
            probe: self.probe,
            commands: Vec::new(),
            error: None,
            summary: self.summary,
            verified: self.verified,
            extra: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subst_src_and_in() {
        let got = subst_argv(
            &["$src".into(), "x".into(), "$in".into()],
            Some("/a.mp4"),
            Some("/b.mp4"),
            1,
        )
        .unwrap();
        assert_eq!(got, vec!["/a.mp4", "x", "/b.mp4"]);
    }

    #[test]
    fn subst_in_needs_prev() {
        let err = subst_argv(&["$in".into()], Some("/a.mp4"), None, 0).unwrap_err();
        assert!(err.message().contains("$in"));
    }

    #[test]
    fn subst_src_needs_input() {
        let err = subst_argv(&["$src".into()], None, None, 0).unwrap_err();
        assert!(err.message().contains("$src"));
    }

    #[test]
    fn aspect_9_16_on_1080x1920() {
        assert!(aspect_matches(1080, 1920, "9:16"));
        assert!(!aspect_matches(1920, 1080, "9:16"));
        assert!(aspect_matches(1920, 1080, "16:9"));
    }
}

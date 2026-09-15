use std::path::PathBuf;
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
            let checks = check_expect(expect, c.probe.as_ref(), c.output.as_deref());
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
    checks
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

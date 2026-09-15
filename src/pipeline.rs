use std::path::PathBuf;
use std::process::Command;

use serde::Deserialize;
use serde_json::json;

use crate::cli::Globals;
use crate::contract::{Contract, Status};
use crate::error::Error;
use crate::paths;

const BLOCKED: &[&str] = &["pipeline", "batch", "install-skill", "version"];

#[derive(Deserialize)]
struct Plan {
    goal: String,
    steps: Vec<Step>,
}

#[derive(Deserialize)]
struct Step {
    tool: String,
    #[serde(default)]
    argv: Vec<String>,
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
    let mut results = Vec::new();
    let mut last_ok: Option<Contract> = None;

    for (i, step) in plan.steps.iter().enumerate() {
        let tool = step.tool.trim();
        if BLOCKED.contains(&tool) {
            return Err(Error::input(format!("pipeline cannot run {tool}")));
        }
        if !allowed.iter().any(|n| n == tool) {
            return Err(Error::input(format!("unknown pipeline tool '{tool}'")));
        }
        if tool == "ffmpeg" && !step.argv.iter().any(|a| a == "--because") {
            return Err(Error::input("pipeline ffmpeg step needs --because in argv"));
        }

        let mut cmd = Command::new(&exe);
        cmd.arg(tool);
        cmd.args(&step.argv);
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
        results.push(json!({
            "index": i,
            "tool": tool,
            "result": parsed.clone(),
        }));
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
        if let Ok(c) = serde_json::from_value::<WireContract>(parsed.clone()) {
            last_ok = Some(c.into_contract());
        }
    }

    let mut c = last_ok.unwrap_or_else(|| Contract::ok("pipeline", None, None));
    c.tool = "pipeline".into();
    c.summary = Some(plan.goal.clone());
    c = c.with_extra(json!({
        "goal": plan.goal,
        "steps": results,
    }));
    Ok(c)
}

#[derive(serde::Deserialize)]
struct WireContract {
    status: String,
    #[serde(default)]
    tool: String,
    #[serde(default)]
    output: Option<String>,
    #[serde(default)]
    probe: Option<crate::probe::Probe>,
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

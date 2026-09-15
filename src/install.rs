use std::path::PathBuf;

use serde_json::json;

use crate::contract::Contract;
use crate::embed::{self, GOTCHAS, GRAPH, PIPELINE, PLATFORMS, SKILL_MD};
use crate::error::Error;

pub fn run(dry_run: bool) -> Result<Contract, Error> {
    let targets = candidate_dirs()?;
    let version = embed::skill_version();

    let mut written = Vec::new();
    for dir in &targets {
        if dry_run {
            written.push(json!({
                "path": dir.display().to_string(),
                "wrote": false,
                "version": version,
            }));
            continue;
        }
        std::fs::create_dir_all(dir.join("references"))?;
        std::fs::write(dir.join("SKILL.md"), SKILL_MD)?;
        std::fs::write(dir.join("VERSION"), format!("{version}\n"))?;
        std::fs::write(dir.join("references/gotchas.md"), GOTCHAS)?;
        std::fs::write(dir.join("references/graph.md"), GRAPH)?;
        std::fs::write(dir.join("references/platforms.md"), PLATFORMS)?;
        std::fs::write(dir.join("references/pipeline.md"), PIPELINE)?;
        written.push(json!({
            "path": dir.display().to_string(),
            "wrote": true,
            "version": version,
        }));
    }

    let mut c = Contract::ok("install-skill", None, None).with_extra(json!({
        "targets": written,
        "dry_run": dry_run,
        "version": version,
    }));
    c.summary = Some(format!("{} skill dir(s)  v{version}", targets.len()));
    if dry_run {
        c.status = crate::contract::Status::DryRun;
        c.verified = None;
    }
    Ok(c)
}

pub fn candidate_dirs() -> Result<Vec<PathBuf>, Error> {
    let home = dirs_home()?;
    let candidates = [
        home.join(".grok/skills/ffkit"),
        home.join(".claude/skills/ffkit"),
        home.join(".codex/skills/ffkit"),
        home.join(".cursor/skills/ffkit"),
    ];
    let mut targets = Vec::new();
    for path in candidates {
        let marker = path
            .parent()
            .and_then(|p| p.parent())
            .map(|agent_home| agent_home.exists())
            .unwrap_or(false);
        if marker {
            targets.push(path);
        }
    }
    if targets.is_empty() {
        targets.push(home.join(".grok/skills/ffkit"));
    }
    Ok(targets)
}

fn dirs_home() -> Result<PathBuf, Error> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .ok_or_else(|| Error::output("HOME is not set"))
}

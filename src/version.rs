use serde::Serialize;
use serde_json::json;

use crate::contract::Contract;
use crate::embed;
use crate::error::Error;
use crate::install;

#[derive(Debug, Serialize)]
pub struct VersionReport {
    pub ffkit: String,
    pub skill: String,
    pub skill_match: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git: Option<String>,
    pub stale: usize,
    pub installed: Vec<InstalledCopy>,
}

#[derive(Debug, Serialize)]
pub struct InstalledCopy {
    pub path: String,
    pub version: Option<String>,
    #[serde(rename = "match")]
    pub matches: bool,
}

pub fn report() -> VersionReport {
    let ffkit = embed::binary_version().to_string();
    let skill = embed::skill_version().to_string();
    let installed = installed_copies(&ffkit);
    let stale = installed.iter().filter(|c| !c.matches).count();
    VersionReport {
        skill_match: ffkit == skill,
        ffkit,
        skill,
        git: embed::git_sha().map(str::to_string),
        stale,
        installed,
    }
}

pub fn run(check: bool) -> Result<Contract, Error> {
    let report = report();
    let extra = serde_json::to_value(&report)?;
    if check && !report.skill_match {
        let err = Error::verification(format!(
            "embedded skill {} != ffkit {}; rebuild the binary",
            report.skill, report.ffkit
        ));
        return Ok(Contract::failed("version", &err).with_extra(extra));
    }
    if check && report.stale > 0 {
        let err = Error::verification(format!(
            "{} installed skill copy(ies) != ffkit {}; run ffkit install-skill",
            report.stale, report.ffkit
        ));
        return Ok(Contract::failed("version", &err).with_extra(extra));
    }

    let mut c = Contract::ok("version", None, None).with_extra(extra);
    c.summary = Some(summary(&report));
    Ok(c)
}

pub fn summary(report: &VersionReport) -> String {
    let mut s = format!("ffkit {}  skill {}", report.ffkit, report.skill);
    if let Some(git) = &report.git {
        s.push_str(&format!("  git {git}"));
    }
    if report.stale > 0 {
        s.push_str(&format!("  {} stale", report.stale));
    }
    s
}

fn installed_copies(bin_ver: &str) -> Vec<InstalledCopy> {
    let Ok(dirs) = install::candidate_dirs() else {
        return Vec::new();
    };
    dirs.into_iter()
        .map(|dir| {
            let skill_path = dir.join("SKILL.md");
            let version = read_installed_version(&dir);
            let matches = version.as_deref() == Some(bin_ver);
            InstalledCopy {
                path: skill_path.display().to_string(),
                version,
                matches,
            }
        })
        .collect()
}

fn read_installed_version(dir: &std::path::Path) -> Option<String> {
    let stamp = dir.join("VERSION");
    if let Ok(s) = std::fs::read_to_string(&stamp) {
        let v = s.trim();
        if !v.is_empty() {
            return Some(v.to_string());
        }
    }
    let md = std::fs::read_to_string(dir.join("SKILL.md")).ok()?;
    embed::parse_skill_version(&md).map(str::to_string)
}

/// JSON object for doctor.extra.ffkit
pub fn doctor_ffkit_json(report: &VersionReport) -> serde_json::Value {
    json!({
        "version": report.ffkit,
        "skill_version": report.skill,
        "skill_match": report.skill_match,
        "git": report.git,
        "stale": report.stale,
    })
}

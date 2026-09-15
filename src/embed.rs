pub const SKILL_MD: &str = include_str!("../SKILL.md");
pub const GOTCHAS: &str = include_str!("../references/gotchas.md");
pub const GRAPH: &str = include_str!("../references/graph.md");
pub const PLATFORMS: &str = include_str!("../references/platforms.md");
pub const PIPELINE: &str = include_str!("../references/pipeline.md");
pub const RECIPES: &str = include_str!("../references/recipes.md");

/// Cargo.toml version. Skill + CLI ship as one SemVer.
pub fn binary_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

pub fn skill_version() -> &'static str {
    parse_skill_version(SKILL_MD).unwrap_or(binary_version())
}

pub fn git_sha() -> Option<&'static str> {
    option_env!("FFKIT_GIT_SHA")
}

/// Read `version:` from YAML frontmatter only (first `---` block).
pub fn parse_skill_version(md: &str) -> Option<&str> {
    let rest = md.strip_prefix("---")?;
    let fm = rest.split("---").next()?;
    for line in fm.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("version:") {
            let v = rest.trim().trim_matches('"').trim_matches('\'');
            if !v.is_empty() {
                return Some(v);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_version_matches_crate() {
        assert_eq!(skill_version(), binary_version());
    }

    #[test]
    fn ignores_version_outside_frontmatter() {
        let md = "---\nname: x\nversion: 1.2.3\n---\n\nversion: 9.9.9\n";
        assert_eq!(parse_skill_version(md), Some("1.2.3"));
    }
}

use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=SKILL.md");
    println!("cargo:rerun-if-changed=.git/HEAD");

    let pkg = std::env::var("CARGO_PKG_VERSION").expect("CARGO_PKG_VERSION");
    let skill = std::fs::read_to_string("SKILL.md").expect("SKILL.md");
    let skill_ver = parse_frontmatter_version(&skill).unwrap_or_default();
    if skill_ver != pkg {
        panic!("SKILL.md version `{skill_ver}` != Cargo.toml `{pkg}`");
    }

    if let Some(sha) = git_sha() {
        println!("cargo:rustc-env=FFKIT_GIT_SHA={sha}");
    }
}

fn parse_frontmatter_version(md: &str) -> Option<String> {
    let rest = md.strip_prefix("---")?;
    let fm = rest.split("---").next()?;
    for line in fm.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("version:") {
            let v = rest.trim().trim_matches('"').trim_matches('\'');
            if !v.is_empty() {
                return Some(v.to_string());
            }
        }
    }
    None
}

fn git_sha() -> Option<String> {
    let out = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let sha = String::from_utf8(out.stdout).ok()?;
    let sha = sha.trim();
    if sha.is_empty() {
        None
    } else {
        Some(sha.to_string())
    }
}

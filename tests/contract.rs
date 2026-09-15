use ffkit::verb_names;

#[test]
fn skill_mentions_every_verb() {
    let skill = include_str!("../SKILL.md");
    let verbs = verb_names();
    assert!(!verbs.is_empty());
    for name in verbs {
        let mentioned = skill.contains(&format!("`{name}`"))
            || skill.contains(&format!("`ffkit {name}"))
            || skill.contains(&format!("ffkit {name} "));
        assert!(
            mentioned,
            "SKILL.md Hands inventory must mention verb `{name}` so clap cannot drift"
        );
    }
}

#[test]
fn description_has_triggers() {
    let skill = include_str!("../SKILL.md");
    let desc = skill.split("---").nth(1).expect("frontmatter");
    assert!(desc.contains("Use when"));
    assert!(desc.to_lowercase().contains("ffmpeg"));
    assert!(desc.contains("libass") || desc.contains("mux"));
    assert!(
        desc.len() < 1200,
        "description must stay inside the 1024-char budget plus yaml keys"
    );
}

#[test]
fn skill_version_matches_crate() {
    let skill = include_str!("../SKILL.md");
    let version = ffkit::embed::parse_skill_version(skill).expect("frontmatter version:");
    assert_eq!(version, env!("CARGO_PKG_VERSION"));
}

const README_LANGS: &[(&str, &str)] = &[
    ("README.md", include_str!("../README.md")),
    ("README.zh.md", include_str!("../README.zh.md")),
];

#[test]
fn readme_languages_stay_in_sync() {
    let verbs = verb_names();
    let facts = [
        "/releases",
        "install.sh",
        "ffkit pipeline",
        "README.md",
        "README.zh.md",
    ];
    let mut heading_counts = Vec::new();
    for (name, body) in README_LANGS {
        for fact in facts {
            assert!(
                body.contains(fact),
                "{name} must contain `{fact}` (keep README languages in sync)"
            );
        }
        for verb in &verbs {
            let mentioned = body.contains(&format!("`{verb}`"))
                || body.contains(&format!("`ffkit {verb}"))
                || body.contains(&format!("ffkit {verb} "));
            assert!(
                mentioned,
                "{name} must mention verb `{verb}` like the other README language"
            );
        }
        let headings = body.lines().filter(|l| l.starts_with("##")).count();
        heading_counts.push((*name, headings));
    }
    let first = heading_counts[0].1;
    for (name, n) in &heading_counts {
        assert_eq!(
            *n, first,
            "{name} has {n} ## headings, README.md has {first}; keep the same sections"
        );
    }
}

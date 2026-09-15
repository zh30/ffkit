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

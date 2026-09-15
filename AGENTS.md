# ffkit — for agents working on this repository

This repo is a **skill**. `SKILL.md` is the product the calling agent loads. The Rust binary `ffkit` is the skill's hands. System `ffmpeg` / `ffprobe` are the engine.

This is not a Cloudflare, pnpm, or TanStack project. Ignore those global defaults here.

## Commands

```
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo run -- doctor --json
cargo run -- probe FILE --json
```

## Rules

- **SKILL.md is the agent prompt.** Keep it short. Workflow + routing table + report shape. Flags live in `ffkit <verb> --help`, not restated in the skill.
- **One JSON contract** (`src/contract.rs`) for every verb, graph, and raw ffmpeg. `error.message` is quoted, never rewritten by the skill.
- **Spawn ffmpeg with `Command` args.** No `sh -c`. No string-concatenated shell.
- **Do not add `ffmpeg-next` / `ffmpeg-sys`.** Encoding always shells out to the user's ffmpeg.
- Adding a verb: clap subcommand + `src/verbs/<name>.rs` + one row in the SKILL.md routing table. `tests/contract.rs` fails if they drift.
- **One SemVer** for skill + CLI. Canonical: `Cargo.toml` `version`. `SKILL.md` frontmatter `version:` must match (`build.rs` fails the compile otherwise). Bump with `scripts/bump-version.sh patch|minor|major`. Then `cargo test && cargo install --path . --force && ffkit install-skill`. Record the change under `CHANGELOG.md` `[Unreleased]` before bumping (the script opens a new dated heading).
- `ffkit version --check` must stay green after install. Do not hardcode the version in README.
- Picture-changing verbs must stay on the `look` path in the skill workflow.
- Source files are never valid `-o` targets.

## Layout

- `SKILL.md` — loaded by Grok / Claude Code / Codex / Cursor
- `references/` — gotchas, graph schema, platforms; one hop from SKILL.md
- `src/` — CLI + verbs
- `tests/` — fixture round-trips against a local ffmpeg

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

- **SKILL.md is the agent prompt.** The user chats; the agent proposes a scheme; `ffkit` executes it. Restated outcome → short plan → `ffkit pipeline` (or one verb). Hands table is ingredients. Do not add a verb for every look/effect; compose existing hands or `graph`/`ffmpeg`.
- Flags live in `ffkit <verb> --help`, not restated in the skill.
- **One JSON contract** (`src/contract.rs`) for every verb, graph, and raw ffmpeg. `error.message` is quoted, never rewritten by the skill.
- **Spawn ffmpeg with `Command` args.** No `sh -c`. No string-concatenated shell.
- **Do not add `ffmpeg-next` / `ffmpeg-sys`.** Encoding always shells out to the user's ffmpeg.
- Adding a verb only when a plan step is repeatedly error-prone as raw ffmpeg. clap subcommand + `src/verbs/<name>.rs` + one Hands row. `tests/contract.rs` fails if they drift.
- **One SemVer** for skill + CLI. Canonical: `Cargo.toml` `version`. `SKILL.md` frontmatter `version:` must match (`build.rs` fails the compile otherwise). Bump with `scripts/bump-version.sh patch|minor|major`. Then `cargo test && cargo install --path . --force && ffkit install-skill`. Record the change under `CHANGELOG.md` `[Unreleased]` before bumping (the script opens a new dated heading).
- `ffkit version --check` must stay green after install. Do not hardcode the version in README.
- Picture-changing verbs must stay on the `look` path in the skill workflow.
- Source files are never valid `-o` targets.

## Layout

- `SKILL.md` — loaded by Grok / Claude Code / Codex / Cursor
- `references/` — gotchas, graph schema, platforms; one hop from SKILL.md
- `src/` — CLI + verbs
- `tests/` — fixture round-trips against a local ffmpeg

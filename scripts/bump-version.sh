#!/usr/bin/env bash
# Bump the single ffkit SemVer in Cargo.toml, SKILL.md, and CHANGELOG.md.
# Usage: scripts/bump-version.sh patch|minor|major|x.y.z
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

if [[ $# -ne 1 ]]; then
  echo "usage: scripts/bump-version.sh patch|minor|major|x.y.z" >&2
  exit 2
fi

current="$(python3 - <<'PY'
import re, pathlib
text = pathlib.Path("Cargo.toml").read_text()
m = re.search(r'^version = "([0-9]+\.[0-9]+\.[0-9]+)"', text, re.M)
if not m:
    raise SystemExit("Cargo.toml: no version")
print(m.group(1))
PY
)"

spec="$1"
next="$(python3 - "$current" "$spec" <<'PY'
import sys
cur, spec = sys.argv[1], sys.argv[2]
major, minor, patch = (int(x) for x in cur.split("."))
if spec == "major":
    nxt = f"{major + 1}.0.0"
elif spec == "minor":
    nxt = f"{major}.{minor + 1}.0"
elif spec == "patch":
    nxt = f"{major}.{minor}.{patch + 1}"
else:
    parts = spec.split(".")
    if len(parts) != 3 or not all(p.isdigit() for p in parts):
        raise SystemExit(f"not a SemVer x.y.z: {spec}")
    nxt = spec
print(nxt)
PY
)"

python3 - "$current" "$next" <<'PY'
import datetime, pathlib, re, sys
old, new = sys.argv[1], sys.argv[2]
today = datetime.date.today().isoformat()

cargo = pathlib.Path("Cargo.toml")
text = cargo.read_text()
text, n = re.subn(r'^version = "' + re.escape(old) + r'"', f'version = "{new}"', text, count=1, flags=re.M)
if n != 1:
    raise SystemExit("Cargo.toml version replace failed")
cargo.write_text(text)

skill = pathlib.Path("SKILL.md")
st = skill.read_text()
st, n = re.subn(r'^version:\s*.+$', f'version: {new}', st, count=1, flags=re.M)
if n != 1:
    raise SystemExit("SKILL.md version replace failed")
skill.write_text(st)

cl = pathlib.Path("CHANGELOG.md")
ct = cl.read_text()
needle = "## [Unreleased]\n"
insert = f"## [Unreleased]\n\n## [{new}] — {today}\n"
if needle not in ct:
    raise SystemExit("CHANGELOG.md missing ## [Unreleased]")
if f"## [{new}]" in ct:
    raise SystemExit(f"CHANGELOG.md already has {new}")
cl.write_text(ct.replace(needle, insert, 1))
print(f"{old} -> {new}")
PY

echo "Next: cargo test, merge the PR. Push to main publishes GitHub Release v${next}."
echo "Local pack: ./scripts/pack-release.sh && ./scripts/publish-release.sh"

#!/usr/bin/env bash
# Build a runnable Release zip: binary + skill + install.sh.
# Env: BIN (path to ffkit), TARGET (rustc triple), DIST (output dir).
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

version="$(python3 - <<'PY'
import re, pathlib
text = pathlib.Path("Cargo.toml").read_text()
m = re.search(r'^version = "([0-9]+\.[0-9]+\.[0-9]+)"', text, re.M)
if not m:
    raise SystemExit("Cargo.toml: no version")
print(m.group(1))
PY
)"

host="$(rustc -vV | awk '/^host:/{print $2}')"
TARGET="${TARGET:-$host}"
DIST="${DIST:-"$root/dist"}"
mkdir -p "$DIST"

if [[ -z "${BIN:-}" ]]; then
    if [[ "$TARGET" == "$host" ]]; then
        cargo build --release --locked
        BIN="$root/target/release/ffkit"
    else
        rustup target add "$TARGET"
        cargo build --release --locked --target "$TARGET"
        BIN="$root/target/$TARGET/release/ffkit"
    fi
fi

if [[ ! -f "$BIN" ]]; then
    echo "missing binary: $BIN" >&2
    exit 1
fi

name="ffkit-${version}-${TARGET}"
stage="$DIST/$name"
rm -rf "$stage"
mkdir -p "$stage/references"
cp "$BIN" "$stage/ffkit"
chmod +x "$stage/ffkit"
cp "$root/SKILL.md" "$root/README.md" "$stage/"
cp "$root/scripts/INSTALL.txt" "$stage/INSTALL.txt"
cp "$root/scripts/install-from-zip.sh" "$stage/install.sh"
chmod +x "$stage/install.sh"
cp "$root/references/"*.md "$stage/references/"

zip_path="$DIST/${name}.zip"
rm -f "$zip_path"
(
    cd "$DIST"
    zip -qry "${name}.zip" "$name"
)
echo "$zip_path"

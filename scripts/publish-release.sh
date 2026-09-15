#!/usr/bin/env bash
# Create or update the GitHub Release for Cargo.toml's version from dist/*.zip.
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

DIST="${DIST:-"$root/dist"}"
shopt -s nullglob
zips=("$DIST"/*.zip)
if [[ ${#zips[@]} -eq 0 ]]; then
    echo "no zip files in $DIST" >&2
    exit 1
fi

notes="$(mktemp)"
python3 "$root/scripts/changelog-notes.py" "$version" >"$notes"
tag="v${version}"

if gh release view "$tag" >/dev/null 2>&1; then
    gh release upload "$tag" "${zips[@]}" --clobber
    echo "updated $tag assets"
else
    gh release create "$tag" "${zips[@]}" \
        --title "ffkit ${version}" \
        --notes-file "$notes"
    echo "created $tag"
fi
rm -f "$notes"

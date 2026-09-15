#!/usr/bin/env python3
"""Print the CHANGELOG.md section for one SemVer (stdout)."""
import pathlib
import re
import sys

if len(sys.argv) != 2:
    raise SystemExit("usage: changelog-notes.py X.Y.Z")

ver = sys.argv[1]
text = pathlib.Path("CHANGELOG.md").read_text()
pat = rf"## \[{re.escape(ver)}\][^\n]*\n(.*?)(?=\n## \[|\Z)"
m = re.search(pat, text, re.S)
body = m.group(1).strip() if m else ""
print(f"## ffkit {ver}\n")
if body:
    print(body)
    print()
print("Download the **platform zip** (binary + skill + `install.sh`).")
print("Do not use the Source code zip as the install bundle.")
print()
print(f"- macOS Apple Silicon: `ffkit-{ver}-aarch64-apple-darwin.zip`")
print(f"- macOS Intel: `ffkit-{ver}-x86_64-apple-darwin.zip`")
print(f"- Linux x86_64: `ffkit-{ver}-x86_64-unknown-linux-gnu.zip`")
print()
print("Unzip, then `./ffkit doctor --json` or `./install.sh`. Needs ffmpeg/ffprobe.")

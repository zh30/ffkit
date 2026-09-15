#!/usr/bin/env python3
"""Fail if Cargo.toml version is not greater than the latest GitHub Release."""
import os
import re
import subprocess
import sys
from pathlib import Path
from typing import Optional, Tuple


def parse(v: str) -> Tuple[int, int, int]:
    v = v.strip().lstrip("v")
    parts = v.split(".")
    if len(parts) != 3 or not all(p.isdigit() for p in parts):
        raise SystemExit(f"not SemVer: {v}")
    return int(parts[0]), int(parts[1]), int(parts[2])


def cargo_version() -> str:
    text = Path("Cargo.toml").read_text()
    m = re.search(r'^version = "([0-9]+\.[0-9]+\.[0-9]+)"', text, re.M)
    if not m:
        raise SystemExit("Cargo.toml: no version")
    return m.group(1)


def latest_release() -> Optional[str]:
    env = os.environ.copy()
    try:
        out = subprocess.run(
            ["gh", "release", "list", "--limit", "1", "--json", "tagName", "--jq", ".[0].tagName"],
            check=False,
            capture_output=True,
            text=True,
            env=env,
        )
    except FileNotFoundError:
        print("gh not installed; skip release comparison", file=sys.stderr)
        return None
    if out.returncode != 0:
        err = (out.stderr or "").strip()
        if "Release not found" in err or "no releases" in err.lower() or not err:
            return None
        # Empty repo with zero releases often exits 0 with empty stdout.
        print(err, file=sys.stderr)
        return None
    tag = (out.stdout or "").strip()
    return tag or None


def main() -> None:
    current = cargo_version()
    latest = latest_release()
    if latest is None:
        print(f"no GitHub Release yet; {current} is the first")
        return
    if parse(current) > parse(latest):
        print(f"{current} > {latest}")
        return
    raise SystemExit(
        f"Cargo.toml {current} must be greater than latest Release {latest}. "
        "Run ./scripts/bump-version.sh patch|minor|major and update CHANGELOG.md / README / SKILL.md."
    )


if __name__ == "__main__":
    main()

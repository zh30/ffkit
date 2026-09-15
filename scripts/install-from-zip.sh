#!/bin/sh
# Copy the unzipped ffkit binary onto PATH and write the skill.
# Run from the unzipped Release folder (next to the `ffkit` binary).
set -e
DIR=$(CDPATH= cd -- "$(dirname "$0")" && pwd)
BIN="$DIR/ffkit"
if [ ! -f "$BIN" ]; then
    echo "ffkit binary missing in $DIR" >&2
    exit 1
fi
chmod +x "$BIN"

if [ -n "${FFKIT_BINDIR:-}" ]; then
    DEST=$FFKIT_BINDIR
elif [ -d "$HOME/.cargo/bin" ]; then
    DEST=$HOME/.cargo/bin
elif [ -w /usr/local/bin ]; then
    DEST=/usr/local/bin
else
    DEST=$HOME/.local/bin
fi
mkdir -p "$DEST"
cp "$BIN" "$DEST/ffkit"
chmod +x "$DEST/ffkit"

if command -v xattr >/dev/null 2>&1; then
    xattr -d com.apple.quarantine "$DEST/ffkit" 2>/dev/null || true
fi

"$DEST/ffkit" install-skill
echo "ffkit -> $DEST/ffkit"
echo "Need ffmpeg and ffprobe on PATH. Then: ffkit doctor --json"
if ! command -v ffkit >/dev/null 2>&1; then
    echo "Add $DEST to PATH to run ffkit by name."
fi

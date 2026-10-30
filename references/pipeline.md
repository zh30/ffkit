# Pipeline

Write this **after** you have proposed the scheme to the user. `goal` is the original task, not the last verb. `expect` is that task as numbers from probe. `ffkit pipeline plan.json --json` runs the steps in order, stops on the first failure, and returns one contract whose `probe` is the last successful writing step.

## Shape

```json
{
  "goal": "9:16 clip of the first half second",
  "input": "talk.mp4",
  "expect": {
    "aspect": "9:16",
    "duration_gt": 0.35,
    "duration_lt": 0.7,
    "has_video": true
  },
  "steps": [
    {
      "tool": "cut",
      "label": "剪前半秒",
      "argv": ["$src", "--start", "0", "--end", "0.5", "-o", "01-cut.mp4"]
    },
    {
      "tool": "fit",
      "label": "竖屏",
      "argv": ["$in", "--aspect", "9:16", "--fit", "pad", "-o", "reel.mp4"]
    }
  ]
}
```

- `goal` is the outcome you restated to the user (required).
- `input` is the user's source. `$src` in any `argv` expands to it.
- `$in` in any `argv` expands to the previous step's `output`. First step cannot use `$in`. `look` last, or the next `$in` becomes a PNG.
- `label` is the user-language step (for the report). Optional.
- `tool` is an `ffkit` subcommand. Not `pipeline`, `batch`, `install-skill`, or `version`.
- `argv` is the verb's arguments as you would type them after `ffkit <tool>`. Include `-o` on writing steps. Do not wrap in a shell. Tokens are a whole argv element (`"$in"`), not a substring.
- `ffmpeg` steps must include `--because` in `argv`.
- `--overwrite` / `--dry-run` / `--timeout` on `ffkit pipeline` apply to every step. Re-runs need `--overwrite`.

## expect

Checked against the **last** output, re-probed from disk (not the step's own report). The file is kept either way. `verified` is true only when every check passes. Unknown keys are an error, so a typo cannot pass silently.

Shape (ffprobe, instant):

| key | meaning |
|-----|---------|
| `aspect` | `9:16` / `16:9` / `1:1` (2px tolerance) |
| `width` / `height` | exact pixels |
| `duration_gt` / `duration_lt` | seconds, exclusive |
| `has_audio` / `has_video` | bool |
| `ext` | `mp4`, `wav`, `png`, … (no dot needed) |
| `fps` | frames per second (±0.02, so `29.97` matches 30000/1001) |
| `vcodec` / `acodec` / `pix_fmt` | `h264`, `aac`, `yuv420p`, … |
| `max_mb` | file size ceiling, MB (10⁶ bytes) |
| `av_drift_max` | seconds; worst start or end gap between first video and audio track |

Signal (decodes the output once with ffmpeg's analyzers; costs about one playback at decode speed):

| key | meaning |
|-----|---------|
| `lufs` (+ `lufs_tol`, default 1) | integrated loudness, EBU R128 (`ebur128`) |
| `true_peak_max` | dBTP ceiling |
| `max_black` | fail on any black stretch at least this many seconds (`blackdetect`) |
| `max_freeze` | fail on any frozen-picture stretch at least this long (`freezedetect`) |
| `ssim_min` | mean SSIM of output vs `input` (`ssim`). Only for re-encodes of the same timeline (`compress`, `transcode`); a changed duration fails as not comparable |

Each row in `extra.expect.checks` is `{key, want, got, ok}`; `got` for black/freeze lists the offending ranges. Targets come from the user or [platforms.md](platforms.md), not from the verb's flags (a `loudnorm -I -16` step checked against `"lufs": -16` proves the verb, not the platform).

Example for a Reel: `{"aspect": "9:16", "fps": 30, "vcodec": "h264", "lufs": -14, "true_peak_max": -1, "max_black": 0.5, "av_drift_max": 0.05}`.

No `expect` → `verified` follows the last step. Failed expect is not a crash; the report's `Check:` line must say the original task is not yet true and quote the failing rows.

Then `ffkit look` on the final picture. Report against `goal`, not against the last verb. More schemes: [recipes.md](recipes.md).

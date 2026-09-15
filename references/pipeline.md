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

Checked against the **last** probe (and output path for `ext`). The file is kept either way. `verified` is true only when every check passes.

| key | meaning |
|-----|---------|
| `aspect` | `9:16` / `16:9` / `1:1` (2px tolerance) |
| `width` / `height` | exact pixels |
| `duration_gt` / `duration_lt` | seconds, exclusive |
| `has_audio` / `has_video` | bool |
| `ext` | `mp4`, `wav`, `png`, … (no dot needed) |

No `expect` → `verified` follows the last step. Failed expect is not a crash; the report's `Check:` line must say the original task is not yet true.

Then `ffkit look` on the final picture. Report against `goal`, not against the last verb. More schemes: [recipes.md](recipes.md).

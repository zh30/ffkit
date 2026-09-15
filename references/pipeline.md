# Pipeline

Write this **after** you have proposed the scheme to the user. `goal` is the original task, not the last verb. `ffkit pipeline plan.json --json` runs the steps in order, stops on the first failure, and returns one contract whose `probe` is the last successful writing step.

## Shape

```json
{
  "goal": "9:16 clip of the first half second",
  "steps": [
    {
      "tool": "cut",
      "argv": ["talk.mp4", "--start", "0", "--end", "0.5", "-o", "01-cut.mp4"]
    },
    {
      "tool": "fit",
      "argv": ["01-cut.mp4", "--aspect", "9:16", "--fit", "pad", "-o", "reel.mp4"]
    }
  ]
}
```

- `goal` is the outcome you restated to the user (required).
- `tool` is an `ffkit` subcommand. Not `pipeline`, `batch`, `install-skill`, or `version`.
- `argv` is the verb's arguments as you would type them after `ffkit <tool>`. Include `-o` on writing steps. Do not wrap in a shell.
- `ffmpeg` steps must include `--because` in `argv`.
- `--overwrite` / `--dry-run` / `--timeout` on `ffkit pipeline` apply to every step.

## Example: mute-friendly Reel

```json
{
  "goal": "做成 Reel，静音也能看，开头有 hook",
  "steps": [
    {"tool": "jumpcut", "argv": ["talk.mp4", "-o", "01.mp4"]},
    {"tool": "title", "argv": ["01.mp4", "--text", "WAIT", "--duration", "0.8", "-o", "02.mp4"]},
    {"tool": "caption", "argv": ["02.mp4", "--srt", "talk.srt", "--mode", "burn", "-o", "03.mp4"]},
    {"tool": "deliver", "argv": ["03.mp4", "--platform", "reels", "-o", "reel.mp4"]}
  ]
}
```

Then `ffkit look reel.mp4 --at 1` (and `--at` a caption time). Report against `goal`, not against `deliver`.

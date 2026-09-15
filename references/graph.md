# Graph

Use `ffkit graph plan.json` when no verb expresses the filter chain. Prefer a verb when one fits.

## Plan shape

```json
{
  "inputs": ["talk.mp4", "logo.png"],
  "filter_complex": [
    {
      "filter": "scale",
      "in": ["0:v"],
      "opts": { "w": 1080, "h": -2 },
      "out": ["v1"]
    },
    {
      "filter": "overlay",
      "in": ["v1", "1:v"],
      "opts": { "x": "W-w-20", "y": "20" },
      "out": ["vout"]
    }
  ],
  "map": ["vout", "0:a"],
  "output": "out.mp4",
  "args": { "c:v": "libx264", "c:a": "copy", "movflags": "+faststart" }
}
```

- `in` / `out` labels become `[label]`. Stream specifiers like `0:v` are valid labels.
- Filter names must exist in this ffmpeg (`ffkit doctor --json` → `filters`, or the graph command checks `ffmpeg -filters`).
- `args` keys are ffmpeg output flags; a leading `-` is optional.
- `--dry-run --json` prints the argv that would run. Source overwrite is still refused.

## When to use graph vs ffmpeg

Graph: several labeled filters, you want validation and JSON.

`ffkit ffmpeg --because REASON -- …`: a flag or bitstream filter no graph field covers (hwaccel, `-f concat` tricks, `-map 0:s:1`). `REASON` is the missing verb or graph field in one line; the flag is required. Last argument is the output path. Still wrapped: timeout, `--overwrite`, output probe.

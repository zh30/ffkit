# Creator multimedia needs → remaining ffkit gaps

Researched 2026-09-22 against **ffkit 0.78.0** (`main` + round-34 branch). Same-day research base as the 0.26.0/0.27.0 rounds below — refreshed priorities only, no new sources needed. This note is not a restatement of landed work — the tail lists what not to redo.

## What 2026 creators still trip on

**Platform size limits are the recurring hard wall.** Agents can edit and export, but "fit under N" is still hand-math:

- Discord free tier is **10 MB** now (nitro 500 MB); WhatsApp video caps ~**16 MB**; email attachments ~**20–25 MB**. Creators keep hitting "file too large" *after* finishing an edit.
- ffmpeg has no `--size` flag: the fix is bitrate math — `video_kbps = (target_bits × 0.98 − audio_bits) / duration` — then **two-pass** to actually land the size ([ffmpeg-cookbook, Apr 2026](https://ffmpeg-cookbook.com/en/articles/two-pass-encoding/); [bitrate math walkthrough, Jul 2026](https://dev.to/alexpua/how-i-make-ffmpeg-hit-an-exact-file-size-the-bitrate-math-nobody-explains-3o7o)). Agents reinvent this per job and overshoot.

**Podcast→clips is the #1 scaled workflow.** [Loopdesk, Jul 2026](https://loopdesk.ai/blog/video-workflows-for-creators): the four workflows that cover almost every creator are podcast-to-clips, weekly YouTube, archive mining, multi-platform publishing. Audio-only episodes need a **picture** before they can be a Reel/Short — the standard artifact is a cover still with an animated waveform (**audiogram**). ffmpeg `showwaves` renders it without any extra dependency.

**Audio cleanup comes before loudness.** [MSY Editor, Mar 2026](https://msyeditor.com/ai-video-editing-workflow-2026/): in every AI edit pipeline, noise reduction is applied *first*, before levels and B-roll — room rumble, laptop fan, hiss. Homebrew/apt ffmpeg ships `afwtdn`, `highpass`, `afftdn`; no model files needed.

**Multi-source audio is the other half of "my audio is bad".** Creators record on the camera AND a lav mic; "swap in the clean track" is a daily ask. `music` mixes a bed *under* the original; nothing *replaces* it. Raw `-map 0:v -map 1:a` is simple but error-prone (sync offset, length mismatch, codec copy of an incompatible container).

**Photo montages are the second-largest audio-only adjacent case.** "Turn these 12 photos into a 30-second Reel with music" is the classic slideshow ask: normalize stills onto a canvas, crossfade, lay a bed that ends cleanly. Doable in raw ffmpeg (`-loop 1 -t` + xfade chain) but the offset math (`k*(per-fade)`) is exactly the kind of thing agents get wrong.

**Landscape→vertical repurpose wants a blurred fill, not black bars.** The repurpose playbooks (CuteDyno Jun 2026, loopdesk multi-platform) assume a blurred pillarbox when the source is 16:9 — black `pad` bars read as unedited. `split + scale=increase,crop + gblur + overlay=centered fg` is a known-graph but error-prone raw.

**HDR iPhone footage washed out in SDR feeds** needs `zscale`+`tonemap`; Homebrew ffmpeg here has `tonemap` but **no `zscale`** (needs `--with-libzimg`). Gate behind `doctor` before promising it.

## Gaps vs ffkit 0.39.0

|| Creator request | Today | Gap |
||-----------------|-------|-----|
|| Swap camera audio for lav mic / new voice / new music | `music` mixes a bed under; nothing replaces | Raw `ffmpeg -map` only — landed this run as `replace` |
|| Photo dump → montage video ("把照片做成视频") | nothing builds video from stills | Landed this run as `slideshow` |
|| "套我的 LUT / film look" | `grade` sliders only | Landed this run as `grade --lut` (`lut3d`) |
|| Word-group "karaoke" captions (chunked, no ASR) | whole-cue raster burn | Landed this run as `caption --chunk N` (approximation — true word highlight still needs a timing source) |
|| Transition variety on montage | `slideshow`/`concat` fade only | Landed this run as `slideshow --transition` + `--motion kenburns` |
|| "Keep room tone under the lav" | `replace` drops the original | Landed this run as `replace --mix G` |
|| Multi-clip montage with transitions | `concat --transition` 2-clip fade only | Landed this run: N-clip xfade/acrossfade chain |
|| "Split into 30s chunks for Status/Stories" | `cut` one range at a time | Landed this run as `split --every` |
|| "把我 P 到绿幕背景上" | `overlay` needs a mask; nothing keys | Landed this run as `key` (`colorkey`) |
|| Chapter/explicit-point splits | `split --every` grid only | Landed this run as `split --at` |
|| HDR→SDR for iPhone clips | — | needs libzimg (`zscale` absent on Homebrew/apt) |

## Ordered directions (this run)
- [x] `mix --gain G` — covered by `--vol-a/--vol-b` (round 66+); dropped.
- [x] `caption --lang` — dropped: burned captions carry no lang tag; `subs --mux --lang` covers selectable subs.
- [x] `rough --by-scene` — shipped round 97.
- [x] `spectrogram --scale` — shipped round 92.
- [x] `timer --bg` — covered by `timer --box-color` (raster plate, shipped earlier).
- [x] `subs --burn --from/--to` — shipped round 100.
- [ ] `insert --transition` already; `--transition` on `multicam` shipped r87 — `concat --level` shipped r75.

1. **`grid --labels`** — "cada tile con su nombre" → raster PNG labels overlaid per cell (no drawtext needed — local ffmpeg 9 lacks it).
2. **`delogo --soft`** — "quitar el logo suave" → generated PNG mask + removelogo interpolation.
3. **`solid --gradient`** — "fondo degradado" → lavfi gradients source for animated cards.
1. **`subs --mux/--lang`** — "subtítulos seleccionables" → mov_text (mp4) / srt (mkv) stream + language tag.
2. **`caption --outline`** — stroked glyphs for busy frames (`render_caption_outlined`).
3. **`audiogram --font`** — custom font for `--text` title.
1. **`reverb --at/--dur`, `eq --at/--dur`** — "echo solo en el hook" → shared `engine::audio_window` (duck/mix) extracted from `fx`.
2. **`loop --from/--to`** — "repetir solo el chiste" → three concat arms, section loops via `loop`/`aloop`.
3. **`loop=size=0` no-op fixed** — the filter silently emits once without a frame buffer; sizes now computed from probe fps/rate (boomerang --times was silently broken since r49).
1. **`fx` echo/lofi/radio** — "voz de walkie-talkie" → radio (bandpass telephone), lofi (acrusher), echo (aecho).
2. **`fx --at` reworked** — none of the FX filters accept timeline `enable` on ffmpeg 4.4 → split/duck/mix graph instead.
3. **`transcode --preset hevc`** — "mismo archivo más chico" → libx265 + `hvc1` tag (QuickTime).
4. **`gate --preset`** — voice/podcast/studio tuned curves.
1. **`fx`** — "efecto de voz raro" → tremolo/vibrato/flanger/phaser/chorus rack.
2. **`fx --at/--dur`** — "el efecto solo en el drop" → enable window (chorus refuses: no timeline).
3. **`boomerang --times`** — "boomerang que se repite" → loop/aloop the cycle.
1. **`overlay --angle`** — "marca de agua en diagonal" → rotate filter pre-chain.
2. **`waveform --scale`** — "onda con detalle bajo" → showwavespic log/sqrt scale.
3. **`split --min-silence`** — "corta en pausas más largas" → silence-gap knob.
1. **`music --at/--dur`** — "la música entra después de la intro" → adelay+atrim bed window.
2. **`channel --mode invert --side`** — "micrófono fuera de fase" → aeval polarity flip.
3. **`sheet --pad/--margin`** — "más aire en la hoja de contacto" → tile spacing.
1. **`countdown --beep`** — "cuenta atrás con pitido" → aevalsrc gated 880Hz.
2. **`leveler --preset`** — "comprime mi voz sin saber números" → voice/podcast/master curves.
3. **`spectrogram --color`** — "espectrograma en mis colores" → showspectrumpic color scheme.
1. **`zoom --out`** — "revela saliendo" → zoompan decreasing z.
2. **`title --shadow`** — "título con sombra" → blurred ghost card under the title.
3. **`extract --width`** — "fotograma a 1080px" → still scaling.
1. **`subs --font`** — "subtítulos en mi fuente" → force_style FontName.
2. **`progress --at/--dur`** — "barra solo al final" → overlay enable window.
3. **`audiogram --position`** — "onda arriba" → wave band y factor (top/center/bottom).
1. **`vignette --at/--dur`** — "viñeta solo ese momento" → timed dark-edge.
2. **`grade --at/--dur`** — "color solo en el sueño" → `:enable=` on every chain filter.
3. **`broll --audio`** — "que se oiga el b-roll" → insert's audio atrim+adelay+amix into the window.
1. **`bw --at/--dur`** — "blanco y negro solo ese momento" → timed desat.
2. **`sharpen --at/--dur`** — "enfoca solo esa toma" → timed unsharp.
3. **`meta --clear`** — "quita mis datos antes de publicar" → strip all tags.
1. **`invert --at/--dur`** — "un flash invertido" → timed negation.
2. **`blur --at/--dur`** — "borroso solo un momento" → timed defocus.
3. **`title` corner positions** — "texto en la esquina" → top-left/right, bottom-left/right.
1. **`replace --fade`** — "que no empiece de golpe" → afade on the swap.
2. **`audiogram --text`** — "ponle el nombre del episodio" → raster title overlay.
3. **`thumb --width`** — "miniatura chiquita" → scaled still.
1. **`broll --fade`** — "corte suave al b-roll" → alpha edge fades.
2. **`frames --at`** — "captura justo en este segundo" → timestamped stills.
3. **`audiogram --size`** — "para YouTube no Reels" → any canvas.
1. **`split --silence`** — "corta el episodio donde hay pausas" → gap-midpoint segments.
2. **`music --fade`** — "la música entra y sale suave" → bed fades.
3. **`eq --preset`** — "una curva de EQ de una palabra" → voice/podcast/bright/bass.
1. **`subs --burn` style flags** — "subtítulos del tamaño y color que yo quiera".
2. **`audiogram --bg`** — brand-color backdrop.
3. **`overlay --opacity` on stills** — "marca de agua suave".
1. **`cut --drop`** — "quita esa parte del medio" → drop middle sections.
2. **`title --outline`** — "el texto no se lee sobre el video" → glyph stroke.
3. **`fit --color`** — "barras del color de marca" → pad color.
1. **`cut --ranges`** — "quédate solo lo bueno" → multi-range keep+join.
2. **`solid`** — "una pantalla de color de fondo" → color card generator.
3. **`volume --limit`** — "sube sin que clipee" → alimiter ceiling.
1. **`overlay --fade`** — "el logo que entre suave" → alpha-faded overlay.
2. **`subs --shift`** — "字幕快了/慢了" → srt retiming.
3. **`meta` tags** — "ponle álbum/género/track" → full container tags.
1. **`title --fade`** — "que el título entre suave" → alpha fades on the title still.
2. **`grade --hue`** — "corregir dominante de color" → hue rotate.
3. **`silence --end`** — "cola de silencio al final" → append without math.
1. **`thumb`** — "截个封面" → one-frame grab.
2. **`subs --burn`** — "字幕压进画面" → libass render (no own-raster srt parse needed).
3. **`split --parts N`** — "切成 N 段" → equal grid.
1. **`sync`** — "音画不同步" → `adelay`/`atrim` constant-offset fix.
2. **`crop --anchor`** — "9:16 保人脸" → anchored aspect reframe.
3. **`art`** — "给音频加封面" → attached-picture embed.
1. **`qa`** — "压完画质损失多少" → PSNR/SSIM vs reference.
2. **`conform`** — "素材规格统一再拼" → size/fps/loudness normalize.
3. **`overlay --mode`** — "叠光效素材" → screen-blend composite.
1. **`mute`** — "把声音去掉" → `-an` stream-copy (instant, no re-encode).
2. **`timer`** — "画面角落计时" → sprite-driven counter without drawtext.
3. **`hls`** — "传到网页播放" → VOD HLS packaging.
1. **`mix`** — "把两段音频叠在一起" → `amix` equal-level sum (distinct from `music`'s duck-bed and `replace --mix`'s one-source blend).
2. **`caption --position top`** — "字幕放上面" → top safe-zone burn placement.
3. **`grade --gamma`** — "中间调提亮" → `eq=gamma` slider.
1. **`countdown`** — "put a 3-2-1 before the action" → `countdown` (`--from`, `--each`, `--go`, `--at`).
2. **`invert`** — "invert the colors" → `negate` (flash frames, clone-VFX base).
3. **`split --size`** — "cut it under 9MB for Discord" → byte-cap grid (also fixes bare-stem `-o part.mp4` in `split`/`frames`).
1. **`crossfade`** — "片头片尾淡接": `acrossfade` between two audio files; output = A+B−overlap.
2. **`strip`** — "发片前去元数据": `-map_metadata -1 -map_chapters -1 -c copy` privacy pass.
3. **`frames`** — "每5秒截一帧": `fps=1/N` image2 dump with split's `stem_%03d` naming.

*(earlier rounds below)*

1. **`voice`** — "把我声音修好": one-shot `agate`→`acompressor`→`loudnorm` — the three-step chain creators otherwise run by hand.
2. **`deinterlace`** — DV/archive footage: `yadif`, `--mode field` doubles rate for smoothest motion.
3. **`fade --color`** — fade to white (or any color), not just black.

*(earlier rounds below)*

1. **`vocal`** — "去掉原唱留伴奏": `pan` center-channel math (`karaoke` = L−R side, `isolate` = mid-only mono). Stereo only — mono inputs refused.
2. **`remux`** — "只换壳不重编码": `-map 0 -c copy` + faststart for the mp4/mov family; codec-incompatible targets surface ffmpeg's own error text.
3. **`meme`** — top/bottom caption burn-in: rasterizes one PNG per text run and chains `overlay`s (reuses the title rasterizer; no libass).

*(earlier rounds below)*

1. **`silence`** — "片头留 2 秒呼吸": `anullsrc` + `concat` pad insertion into audio files (video frame-holds stay with `freeze`; `anullsrc` must match the input's `cl`/`r` for concat to chain).
2. **`grade --preset`** — "一键电影感": `cinematic`/`vivid`/`vintage`/`soft` named looks stacked under the sliders. `colorbalance` midtones are `bm`/`rm`/`gm` on ffmpeg ≤5 (`ms` is new syntax — verified crash on 4.4).
3. **`transcode --fps`** — was GIF-only; h264/webm retime via `fps={n}` in the `-vf` chain.

*(earlier rounds below)*

1. **`tempo`** — "播客 1.5 倍速听": `atempo` chain, pitch held; refuses video inputs (`speed` retimes those — audio half + video whole would desync).
2. **`leveler`** — "声音忽大忽小": `acompressor` threshold/ratio/makeup, sits before `loudnorm` in the voice chain.
3. **`gate`** — `agate` closes between-word hiss/room noise; creators think in dB, `agate` takes linear amplitude — converted (`10^(dB/20)`).

*(earlier rounds below)*

1. **`waveform`** — waveform PNG export for podcast art/thumbnails/embeds (`showwavespic`; complements `audiogram`'s animated video).
2. **`spectrogram`** — `showspectrumpic` PNG so creators/agents can *see* the noise or hum before picking cleanup settings.
3. **`dehum`** — mains hum (50/60 Hz + harmonics) via `highpass` + Q=12 `equalizer` notch chain; the "电流声/嗡鸣" complaint `denoise`'s broadband path doesn't target.

*(earlier rounds below)*

1. **`vdenoise`** — "暗光素材全是噪点": `nlmeans` spatial denoise (`--strength`); `hqdn3d` was tried first and barely moves strong noise — nlmeans is the usable engine.
2. **`crop`** — manual reframe/crop-out-edge (`--region x:y:w:h`, or `--aspect W:H` center box) — `autocrop` only detects letterbox; creators crop for recompose and watermark-edge removal.
3. **`title --position bottom`** — lower-third placement joins `top`/`center` (one match arm).

*(earlier rounds below)*

1. **`bleep`** — the classic TV-safe beep over a word: silences the source inside `--at`/`--dur` and mixes a delayed `sine` tone (`--freq`, `--level`). `volume --at` mutes but leaves dead air; creators want the audible cover.
2. **`censor --at/--dur`** — faces/plates that only appear for part of the clip (the window pattern, third verb after `delogo`/`volume`).
3. **`grade --warm`** — "my footage is too blue/orange": white-balance warmth −1..1 via `colortemperature` (the `colorbalance` midtone term is a no-op on mid-gray — verified).

*(earlier rounds below)*

1. **`reverb`** — flat phone-mic voiceover sounds dead; `aecho` room/hall/cave presets give it space (one `--wet` knob).
2. **`audiogram --mode`/`--color`** — brand-styled waveform strips (line/p2p/point + colour), the recurring "match my brand" ask on podcast clips.
3. **`delogo --at/--dur`** — watermark/logo boxes that appear mid-clip only (intro cards, end screens).
4. **`meta --rotate`** — "shot fine but plays sideways": fix the display-rotation flag losslessly (version-gated: `-display_rotation` on ffmpeg ≥7, `rotate` metadata below).

*(earlier rounds below)*

1. **`autocrop`** — `cropdetect` scan pass + crop; letterbox removal is a top rewatch ask.
2. **`sheet`** — contact-sheet thumbnails for preview/selection (`fps=N/dur,tile`).
3. **`title --at`** — lower-thirds mid-clip, one flag on an existing verb.

*(round 16: `title --size/--color`, `meta`, `broll --motion`. round 15: `rotate`, `delogo`, `speed --interp`. round 14: `eq`, `zoom --motion`, `broll --still`. round 13: `channel`, `loop --until`, `title --tile`. round 12: `overlay --tile`, `caption --shift`, gif flags. round 11: `split --scenes`, `cutsil`, `grid --audio`. round 10: `replace --duck`, `pitch`, `grade --grain`. round 9: `autocrop`, `sheet`, `title --at`. round 8: `boomerang`, `chapter`, `zoom --at/--dur`, `key --despill`)*

1. **`boomerang`** — social fwd+rev replay (`split`+`reverse`/`areverse`+`concat`).
2. **`chapter`** — named marks for players/YouTube via `ffmetadata` `-c copy` (lossless).
3. **`zoom --at/--dur`**, **`key --despill`** — windowed punch and edge cleanup, cheap flags on existing verbs.

*(round 7: `freeze`, `censor`, `speed --at/--dur`. round 6: `grid`, `progress`, `volume --at/--dur`)*

1. **`freeze`** — hold-frame mid-clip or outro (`tpad` clone; silence under the hold).
2. **`censor`** — "把脸/车牌打码": crop→`pixelize`/`gblur`→overlay on a pixel box.
3. **`speed --at/--dur`** — slow-mo punch / fast-forward one window (3-segment trim/concat).

*(round 6: `grid`, `progress`, `volume --at/--dur`. round 5: `key`, `split --at`)*

1. **`grid`** — reaction / comparison / multi-angle tiles (`xstack`, scale+pad per cell, `amix` when all inputs carry audio).
2. **`progress`** — the engagement trick of a bottom bar filling across the video (drawbox w is init-only → sliding-strip `overlay` recipe).
3. **`volume --at/--dur`** — mute/bleep just a moment (`volume=…:enable='between(t,a,b)'`).

*(round 5: `key`, `split --at`)*

1. **`key`** — "把我 P 到绿幕背景/把产品 P 进场景": `colorkey` the foreground over a `--bg` image/video normalized to the FG canvas; the compositing ask `overlay` alone can't answer (it needs a mask).
2. **`split --at t1,t2`** — chapter splits: same forced-keyframe + `-segment_times` machinery, explicit cut list instead of an even grid.

*(round 4: `split --every`, N-clip `concat --transition` chains. round 3: `caption --chunk`, `slideshow --transition`/`--motion kenburns`, `replace --mix`)*

Next (not this run): true word-highlight karaoke (needs a word-timed source — whisper export or `align`; none in repo), HDR→SDR (needs libzimg — absent on Homebrew/apt), `key` despill (green fringe on edges), `grid` audio-follow-one-input option.

## Already landed (do not redo)

Deliver 1080×1920 −14 LUFS; caption burn without libass + `--safe social` bottom-20%; broll cutaway keeps A-roll audio/duration and plays B from its first frame; rough-cut speech islands (list, then encode only keeps); music duck (aformat dbl pin for sidechaincompress on apt ffmpeg); speed; jumpcut; cover; fade; title; loop; stabilize; reverse; grade/zoom/sharpen/vignette/bw/volume/blur; pipeline `$src`/`$in`/`expect`; GitHub Release zips; English + Chinese README; **0.26.0**: `denoise` (afwtdn/afftdn fallback), `compress --size` two-pass budget, `fit`/`broll --fit blur`, `audiogram`. **0.27.0**: `replace`, `slideshow`, `grade --lut`. **0.28.0**: `caption --chunk`, `slideshow --transition`/`--motion kenburns`, `replace --mix`. **0.29.0**: `split --every`, N-clip `concat --transition` chains. **0.30.0**: `key`, `split --at`. **0.31.0**: `grid`, `progress`, `volume --at/--dur`. **0.32.0**: `freeze`, `censor`, `speed --at/--dur`. **0.33.0**: `boomerang`, `chapter`, `zoom --at/--dur`, `key --despill`. **0.34.0**: `autocrop`, `sheet`, `title --at`. **0.35.0**: `replace --duck`, `pitch`, `grade --grain`. **0.36.0**: `split --scenes`, `cutsil`, `grid --audio`. **0.37.0**: `overlay --tile`, `caption --shift`, gif `--fps/--width`. **0.38.0**: `channel`, `loop --until`, `title --tile`. **0.39.0**: `eq`, `zoom --motion`, `broll --still`. **0.40.0**: `rotate`, `delogo`, `speed --interp`. **0.41.0**: `title --size/--color`, `meta`, `broll --motion`. **0.42.0**: `overlay --at`, `caption --color/--size`, `subs`. **0.43.0**: `reverb`, `audiogram --mode/--color`, `delogo --at/--dur`, `meta --rotate`. **0.44.0**: `bleep`, `censor --at/--dur`, `grade --warm`. **0.45.0**: `vdenoise`, `crop`, `title --position bottom`. **0.46.0**: `waveform`, `spectrogram`, `dehum`. **0.47.0**: `tempo`, `leveler`, `gate`. **0.48.0**: `silence`, `grade --preset`, `transcode --fps`. **0.49.0**: `vocal`, `remux`, `meme`. **0.50.0**: `voice`, `deinterlace`, `fade --color`. **0.51.0**: `crossfade`, `strip`, `frames`. **0.52.0**: `split --size` + bare-stem fix, `countdown`, `invert`. **0.53.0**: `mix`, `caption --position`, `grade --gamma`. **0.54.0**: `timer`, `mute`, `hls`. **0.55.0**: `qa`, `conform`, `overlay --mode`. **0.56.0**: `sync`, `crop --anchor`, `art`. **0.57.0**: `thumb`, `subs --burn`, `split --parts`. **0.58.0**: `title --fade`, `grade --hue`, `silence --end`. **0.59.0**: `overlay --fade`, `subs --shift`, `meta` tags. **0.60.0**: `cut --ranges`, `solid`, `volume --limit`. **0.61.0**: `cut --drop`, `title --outline`, `fit --color`. **0.62.0**: `subs` burn styles, `audiogram --bg`, `overlay --opacity`. **0.63.0**: `split --silence`, `music --fade`, `eq --preset`. **0.64.0**: `broll --fade`, `frames --at`, `audiogram --size`. **0.65.0**: `replace --fade`, `audiogram --text`, `thumb --width`. **0.66.0**: `invert --at/--dur`, `blur --at/--dur`, `title` corners. **0.67.0**: `bw --at/--dur`, `sharpen --at/--dur`, `meta --clear`. **0.68.0**: `vignette --at/--dur`, `grade --at/--dur`, `broll --audio`. **0.69.0**: `subs --font`, `progress --at/--dur`, `audiogram --position`. **0.70.0**: `zoom --out`, `title --shadow`, `extract --width`. **0.71.0**: `countdown --beep`, `leveler --preset`, `spectrogram --color`. **0.72.0**: `music --at/--dur`, `channel --mode invert`, `sheet --pad/--margin`. **0.73.0**: `overlay --angle`, `waveform --scale`, `split --min-silence`. **0.74.0**: `fx`, `boomerang --times`. **0.76.0**: `reverb --at/--dur`, `eq --at/--dur`, `loop --from/--to`. **0.77.0**: `subs --mux/--lang`, `caption --outline`, `audiogram --font`. **0.78.0**: `grid --labels`, `delogo --soft`, `solid --gradient`. **0.75.0**: `fx` echo/lofi/radio, `fx --at` duck/mix, `transcode --preset hevc`, `gate --preset`.

Deliver 1080×1920 −14 LUFS; caption burn without libass + `--safe social` bottom-20%; broll cutaway keeps A-roll audio/duration and plays B from its first frame; rough-cut speech islands (list, then encode only keeps); music duck (aformat dbl pin for sidechaincompress on apt ffmpeg); speed; jumpcut; cover; fade; title; loop; stabilize; reverse; grade/zoom/sharpen/vignette/bw/volume/blur; pipeline `$src`/`$in`/`expect`; GitHub Release zips; English + Chinese README; **0.26.0**: `denoise` (afwtdn/afftdn fallback), `compress --size` two-pass budget, `fit`/`broll --fit blur`, `audiogram`. **0.27.0**: `replace`, `slideshow`, `grade --lut`. **0.28.0**: `caption --chunk`, `slideshow --transition`/`--motion kenburns`, `replace --mix`. **0.29.0**: `split --every`, N-clip `concat --transition` chains. **0.30.0**: `key`, `split --at`. **0.31.0**: `grid`, `progress`, `volume --at/--dur`. **0.32.0**: `freeze`, `censor`, `speed --at/--dur`. **0.33.0**: `boomerang`, `chapter`, `zoom --at/--dur`, `key --despill`. **0.34.0**: `autocrop`, `sheet`, `title --at`. **0.35.0**: `replace --duck`, `pitch`, `grade --grain`. **0.36.0**: `split --scenes`, `cutsil`, `grid --audio`. **0.37.0**: `overlay --tile`, `caption --shift`, gif `--fps/--width`. **0.38.0**: `channel`, `loop --until`, `title --tile`. **0.39.0**: `eq`, `zoom --motion`, `broll --still`. **0.40.0**: `rotate`, `delogo`, `speed --interp`. **0.41.0**: `title --size/--color`, `meta`, `broll --motion`.
- [ ] `gif --loop N` + `gif --bounce` loop controls
- [ ] `split --duration` clip-length cap for social exports
- [ ] `cover --blur` (ambient card)
- [ ] `meme --position` (arbitrary text spots)
- [ ] `grid --audio` polish
- [ ] `grid --time` (same tile timestamps for grid)
- [ ] `subs --merge` combine tracks
- [ ] `concat --audio-fade` (acrossfade between clips)
- [ ] `caption --karaoke` word-by-word
- [ ] `broll --position` PiP corner
- [ ] `grid --audio` exists; `grid --labels` done — `grid --time`? (n/a, multi-input)
- [ ] `progress --position` / `--color`
- [ ] `waveform --duration` clip visuals — covered by `audiogram --from/--to` (round 107)
- [ ] `mix --gain`
- [ ] `gate --at/--dur`, `mix --at/--dur` (same window family) — both shipped
- [ ] `audiogram --subs` burn captions on the waveform video — shipped
- [ ] `broll --volume` scale insert audio — shipped

## Shipped this run (round 150)
- `scroll --at` — comma list replays the roll/ticker at several marks (per-window overlay chain, `extra.windows`); `end` resolves via `resolve_at` (needs --dur). Last `parse_time` `--at` straggler gone.


## Shipped this run (round 149)
- `timer`/`countdown`/`meter --at` now take `end` (and `end-N`-style anchors) — the last three numeric `--at` knobs moved onto `resolve_frame_at`.


## Shipped this run (round 148)
- `hls --poster-at` — poster frame time for the player card (sec or `end`, clamped inside the stream); errors without `--poster`. `deliver --crf` — H.264 quality knob on platform delivery (default 20).

## Shipped this run (round 146)
- `audiogram --at a,b,... --dur N` — the podcast→clips play in one call: each point starts an N-second audiogram written as `<stem>_N.mp4` (`extra.files`); single `--at`/`--from` + `--dur` bounds one clip. Rendering refactored to `render_clip(from, to, out)` per window.



## Shipped this run (round 145)
- `extract --at a,b,...` + `cover --at a,b,...` — comma `--at` grabs one still/cover per timepoint as `<stem>_N.<ext>` (`extra.files`); `cover` applies the same canvas/`--blur`/`--size` to every output. With `thumb` (r144) and `frames`, every still-grabber now takes a beat list in one call.

## Shipped this run (round 147)
- `extract --gif --at a,b,...` — comma `--at` writes one palette GIF per timepoint as `<stem>_N.gif` (`extra.files`); each clip reuses the 2-pass palette path with `--dur`/`--width`/`--bounce`/`--loop` honored.

## Shipped this run (round 143)
- `grid --time` — stamps the same mm:ss readout on every tile's bottom-right (multi-cam/review grids show matching clocks); rendered from one shared digit sprite — no libass/drawtext needed.

## Shipped this run (round 142)
- `concat --audio-fade N` — fades audio out/in at every joint (boundary `afade` per clip: each non-last clip's tail fades out, each non-first clip's head fades in). Keeps every clip's duration — no overlap, no drift, lip sync preserved; forces the re-encode path since stream-copy can't fade.

## Shipped this run (round 141)
- `waveform`/`spectrogram --at a,b,...` — a comma `--at` renders one PNG per window as `<stem>_N.png` (spectrogram slices the audio before each render; waveform crops each rendered strip) and lists them in `extra.outputs`.

## Shipped this run (round 140)
- `rotate --at/--dur` — dutch tilt / flip only inside a window (comma `--at` lists several tilts; `--deg` 90° turns error out — they change the canvas mid-clip).

## Shipped this run (round 139)
- `concat --gap N` — inserts N seconds of black + silence between every pair of clips (beat gap between montage sections); exclusive with `--transition`.

## Shipped this run (round 138)
- `replace --at a,b,...` — comma list swaps the audio track inside several windows; the new track is laid across them **in order** (window i plays the slice after window i−1's), `--fade` eases each window.

## Shipped this run (round 137)
- `key --at/--dur` — green-screen composite only inside a window (comma list for several; talent walks over the keyed section, background shows elsewhere).
- `subs --shift --from/--to` — bound the retiming to cues overlapping a window (only the stretch after an inserted segment is late; `end` ok).

## Shipped this run (round 136)
- `freeze --at a,b,...` — comma list freezes a frame at several points (plain holds; `--ease`/`--reverse`/`--zoom` stay single-point).
- `fade --dip a,b,...` — comma list dips to the color at every mark (one dip per scene cut; `--dip` now takes a string list).

## Shipped this run (round 135)
- `deliver --fps N` — output frame rate override (60 for gameplay/sport uploads; default stays 30).
- `subs --burn --margin N` — exact `MarginV` in px (overrides `--safe`'s computed zone margin).
- `hls --poster` — writes `poster.jpg` (mid-video frame) next to the playlist for the web player's poster frame.

## Shipped this run (round 134)
- `music --at a,b,...` — multi-entrance bed: per-window wet branches (intro sting + outro sting, `end` ok) amixed into one bed before the duck stage.
- `silence --at a,b,...` — comma list pads quiet at several points (pause beats between scenes); alternating `atrim`+`anullsrc` concat.

## Shipped this run (round 133)
- `insert --at a,b,...` — comma list splices the clip at several points (sponsor sting at every chapter; plain splice only, `--transition` stays single-point).
- `broll --at a,b,...` — comma list re-flashes the same cutaway at several points: per-window `split` overlay branches restart the insert, `--fade`/`--audio` apply per window.

## Shipped this run (round 132)
- `concat --transition a,b,...` — comma list picks a different xfade per joint (single value still applies to all).
- `channel --mode split` — stereo → `<stem>_L.wav` + `<stem>_R.wav` mono stems (host/guest mic separation for podcast cleanup).
- `remux --aspect 16:9` — display-aspect fix while stream-copying (anamorphic → square pixels w/o re-encode).

## Shipped this run (round 130)
- Comma-list `--at` on trim/concat verbs — `speed`, `tempo`, `zoom` (alternating normal/FX N-segment graph via new `time::window_list` merge) + `vdenoise` (enable_expr).

## Shipped this run (round 129)
- Comma-list `--at` on every windowed audio-FX verb — `reverb`, `eq`, `dehum`, `denoise`, `leveler`, `gate`, `vocal`, `pitch`, `voice`, `fx` (needs `--dur`). New `engine::audio_window_for` emits a per-window wet branch (`atrim`+`adelay`) and ANDs `1-between` dry gates.

## Shipped this run (round 128)
- Comma-list `--at` on every windowed look verb — `invert`, `blur`, `bw`, `sharpen`, `vignette`, `delogo`, `grade`, `progress`, `overlay` (needs `--dur`). Shared `time::enable_expr`/`enable_windows` helpers replace per-verb `between`/`gte` blocks.

## Shipped this run (round 127)
- `title --at t1,t2 --duration N` — flash the hook card at several marks (per-window alpha fades).
- `meme --at t1,t2 --dur N` — caption at several spots (needs `--dur`).

## Shipped this run (round 126)
- `mute --at t1,t2 --dur N` — comma list silences several coughs/words (OR'd `between` enable; needs `--dur`).
- `volume --at t1,t2 --dur N` — comma list rides several gain windows.

## Shipped this run (round 125)
- `bleep --at t1,t2,...` — comma list censors several words in one pass (one sine + adelay per window, OR'd mute enable).
- `censor --at t1,t2 --dur N` — comma list covers several spots (requires `--dur`).

## Shipped this run (round 124)
- `audiogram --from end`/`end-N` — clip the rendered segment from the tail.
- `caption --from end`/`end-N`, `--to end` — cue-window filter anchored to the tail.

## Shipped this run (round 123)
- `thumb --count --from end-N` — bound the spread to the tail (`end`/`end-N` on `--from`).
- `multicam --at end` — tail switch point (switch back to cam A for the outro).
- `subs --burn --from end-N` — window filter anchored to the tail.

## Shipped this run (round 122)
- `split --fade N` — soft edges around every boundary (vf `fade` + af `afade` per cut, clamps to half the shortest part).
- `spectrogram --separate` — per-channel bands (`showspectrumpic mode=separate`).

## Shipped this run (round 121)
- `cut --fade N` — fade in/out at the trimmed edges (re-encodes, clamps to half the cut).
- `deliver --platform square` — 1080x1080 Instagram-grid / LinkedIn pack.

## Shipped this run (round 120)
- `end` sweep leftovers: `subs --burn --to end`, `audiogram --to end`, `bleep --at end --dur` (via `time::resolve_at`).

## Shipped this run (round 119)
- `channel --mode pan --pan -1..1` — stereo pan (push the mix to one ear for ASMR / placement).
- `grade --exposure` — real EV stops (-3..3, ffmpeg `exposure` filter) to rescue under/over-exposed footage.

## Shipped this run (round 118)
- `sprite --from/--to` — bound the seek-preview thumbnail window (`--to end` ok); VTT cues stay on absolute media times.
- `loop --from/--to` accept `end` — loop the tail section without probing the duration.
- `frames --count N` — N evenly-spaced stills across the clip (same spacing as `thumb --count`).

## Shipped this run (round 117)
- `cut --ranges`/`--drop` accept `end` bounds — `T-end` runs through the tail, `end-N` is the last N seconds (`--drop end-10` trims the outro without probing the duration).
- `thumb --count --from/--to` — bound the even-spread still window (`--to end` ok).

## Shipped this run (round 116)
- `end` sweep completed: `--at end` on `boomerang`, `mute`, `dehum`, `voice`, `vdenoise`, `zoom` (via `time::resolve_at`) and `--at end` = last frame on `thumb`, `cover`, `frames` (new `time::resolve_frame_at`, duration − 0.05s, no `--dur` needed).

## Shipped this run (round 115)
- `chapter --yt` — export marks as YouTube description lines ("0:00 Intro") that paste into a video description for platform seek chapters; `--import` accepts the same `H:MM:SS Title` lines back (round-trip).
- `waveform`/`spectrogram --at end --dur N` — visualize just the tail slice.

## Shipped this run (round 114)
- `sprite` — seek-preview thumbnails for video players: `<stem>-N.jpg` tile sheets + a WebVTT cue file with `#xywh` cell coordinates (`--every`/`--width`/`--cols`/`--rows`/`--vtt`).

## Shipped this run (round 113)
- `--at end` closes out the window family — `speed` (outro slow-mo), `tempo`, `mix` (B track into A's tail), `music` (bed ends with the talk), `eq`, `reverb`, `fx`, `denoise`, `leveler`, `gate`, `vocal`, `pitch` — every `--at/--dur` verb now tail-anchors.

## Shipped this run (round 112)
- `--at end` on every windowed look/gain verb — title (end-cards), blur, bw, invert, sharpen, vignette, grade, delogo, volume, progress — one shared `time::resolve_at`.

## Shipped this run (round 111)
- `zoom --center X,Y` — punch target point in % of frame (zoom the left-third speaker, not just dead center); works in the still punch and the kenburns push.
- `censor/meme/overlay --at end --dur N` — tail-anchored windows: blur the outro QR, flash a meme on the last beat, watermark only the close. Shared `time::resolve_at`.

## Shipped this run (round 110)
- `deliver --platform youtube` — 16:9 landscape pack (1920x1080, same −14 LUFS chain); YouTube was the biggest missing platform.
- `slideshow --bg` — letterbox color behind stills (brand-color photo montages).
- `extract --at end` — grab the last frame (--gif gives the last --dur seconds: outro/reaction GIFs).

## Shipped this run (round 109)
- `vdenoise --at/--dur` — window the nlmeans denoise to just the grainy scene (it's the priciest filter in the kit).
- `cover --size WxH` — poster canvas at any size (1280x720 for YouTube thumbs), not just 1080x1920.
- `grid --bg` — gutters/letterboxes pick up a brand color instead of black.

## Shipped this run (round 108)

- `slideshow --dur N` — total montage runtime spread across the stills (`per = (dur + (n-1)*fade)/n`): "make my 12 photos a 30-second Reel" is one flag instead of per-image math.
- `voice --at/--dur` — window the podcast polish chain (agate→acompressor→loudnorm) via the shared `engine::audio_window` dry/wet graph, matching the rest of the audio verbs.
- `freeze --zoom Z` — slow push-in on the held frame (zoompan, end scale 1.0–2): the editor "punch-in on the freeze" trick without a second tool; mid-clip `--at` freezes only.

## Shipped this run (round 107)

- `transcode --preset wav` / `flac` / `opus` — the audio-only preset family completes: pcm_s16le lossless for DAW/edit handoff, flac for archival, libopus 128k for the smallest voice/music delivery; same `-vn` path and `--copy-audio` passthrough as mp3/aac.
- `audiogram --from` / `--to` — clip a segment of the episode straight into the audiogram (`atrim` + `asplit` so the waveform and the mapped audio share the window); the podcast→clips ask no longer needs a separate `cut` step.

## Shipped this run (round 106)

- `transcode --preset mp3` / `--preset aac` — audio-only delivery (`-vn` + libmp3lame/aac 192k): podcast exports and voice-note uploads without knowing codec flags; `--copy-audio` stream-copies when the container already holds the codec.

## Shipped this run (round 105)

- `chapter --shift SEC` — re-times every mark (negative pulls earlier): after adding/removing an intro card the marks slide once instead of re-authoring the file.
- `broll --at end` — tail cutaway sugar: inserts over the last `--duration` seconds without probing the A-roll first (`insert --at end` parity).

## Shipped this run (round 104)

- `transcode --alpha` — keeps the alpha channel for overlay/lower-third exports: `yuva420p` on webm, ProRes 4444 (`-profile:v 4`, `yuva444p10le`, `apl0` vendor tag); errors on h264/hevc/av1 which can't carry alpha.
- `slideshow --volume` — linear gain on the `--audio` bed (0..=4) so beds sit under narration.

## Shipped this run (round 103)

- `progress --edge left|right` — vertical progress bar (bottom-up fill) for 9:16 shorts; `--bg` track works on the vertical edges too.
- `sheet --title` — header row above the contact sheet (raster PNG card padded on top; combines with `--time` stamps).

## Shipped this run (round 102)

- `bw --strength` — partial desaturation (maps to `hue=s=1-S`): "mute the color" looks without going full monochrome.
- `insert --at end` — appends the clip just before the tail (sugar for outro cards/post-roll without probing the duration).

## Shipped this run (round 101)

- `countdown --bg` — 200-alpha card plate rendered under each numeral PNG (streamable "3-2-1" on busy backdrops).
- `audiogram --fps` — `rate=` on showwaves AND showfreqs; showfreqs only gained `rate` in ffmpeg 7 → on 4.x `--fps` + `--mode spectrum` errors instead of emitting a dead flag (caught on the 4.4.8 keg).

## Shipped this run (round 100)

- `subs --burn --from/--to` — temp-srt rewrite keeps only cues overlapping the window: burn just the translated slice of a long take.
- `caption --from/--to` — same overlap filter on the raster caption path (burn only the hook's captions).

## Shipped this run (round 99)

- `waveform --bg` — `color=c=…` underlay + overlay composite: opaque card behind the wave (thumbnails/podcast art where transparency renders black).
- `broll --border [--border-color]` — `pad=iw+2b:ih+2b` ring on the PiP insert (parity with `overlay --border`).

## Shipped this run (round 98)

- `remux --video` — video-only repack (mute a clip without re-encoding; complement of `--audio`).
- `chapter --remove` — `-map_chapters -1` strip on remux for platforms that mangle embedded marks.
- `scroll --bg` — opaque drawbox bar behind `--mode ticker` (news-crawl look; drawbox uses `iw`/`ih`, not `W`/`H` — caught live on ffmpeg 9).

## Shipped this run (round 97)

- `rough --by-scene` — scene-change cuts inside speech islands: `select=gt(scene,0.4)`+`metadata=print` pts_times split each keep so jump cuts never span a shot boundary (new `src/scene.rs` helper).
- `meter --at/--dur` — windowed EBU meter for QC-ing one slice.

## Shipped this run (round 96)

- `audiogram --fscale lin|log|rlog` — showfreqs frequency axis (spectrum mode; log-freq bars for music).
- `solid --fps N` — card frame rate (24 for filmic grain cards).
- `waveform --full` — `draw=full` every-pixel wave rendering.
- `scroll --wrap N` — word-wrap credit lines (wrap family now across title/caption/meme/solid/scroll).

## Shipped this run (round 95)

- `meter` — live EBU R128 loudness meter video (`ebur128 video=1`; QC pass for podcast/voice delivery).
- `caption --wrap N` — word-wrap cue lines at N columns (shared title wrap helper).
- `title --opacity PCT` — ghost/watermark titles (post-multiplies card alpha).
- `solid --align` — per-line `--text` alignment (completes align across title/caption/meme/scroll/solid).

## Shipped this run (round 94)

- `solid --noise N` — animated film grain on color/gradient cards (`noise=alls=N:allf=t`).
- `countdown --tone HZ` — beep frequency (default 880).
- `audiogram --split` — `split_channels=1` per-channel rows on the wave strip.
- `scroll --align left|center|right` — per-line alignment in the credit block.

## Shipped this run (round 93)

- `waveform --split` — `split_channels=1` per-channel rows (stereo L/R inspection).
- `spectrogram --no-legend` — `legend=0` drops the axis strip for clean thumbnails.
- `dehum --freq HZ` — custom hum fundamental 20–500 Hz (fan/transformer buzz); overrides --mains.
- `meme --align left|center|right` — per-line alignment in wrapped meme cards (plain path).

## Shipped this run (round 92)

- `caption --align left|center|right` — per-line caption card alignment via shared raster `TextAlign` (burn path).
- `spectrogram --scale lin|sqrt|cbrt|log|4thrt|5thrt` — `showspectrumpic` display scale.
- `audiogram --scale lin|log|sqrt|cbrt` — wave amplitude scale on showwaves modes (spectrum excluded).
- `subs --burn --shadow N` — libass `Shadow=N` (0–8) on burned captions.

## Shipped this run (round 91)

- `title --align left|center|right` — per-line alignment inside the card; lower-thirds convention (conflicts `--outline`/`--shadow`).
- `timer --start N` — seed the readout: up counts `N+(t-at)`, down counts `N-(t-at)` (default down-start = window length).
- `waveform --peak` — `showwavespic filter=peak` transient rendering instead of average.
- `solid --wrap N` — folds long `--text` via the shared title wrap helper.

## Shipped this run (round 90)

- `meme --wrap N` — word-wraps long top/bottom text (shared `title::wrap` helper).
- `channel --mode widen` — `extrastereo=m=2.5` stereo widening for flat camera audio.
- `transcode --preset av1` — AV1 delivery: `libsvtav1` on ffmpeg ≥7, `libaom-av1 -cpu-used 4 -row-mt 1` on 4.x (version-gated via `ffmpeg_major`).
- `scroll --mode ticker` — bottom news crawl (`x=W-(W+w)·t/dur`, `y=H-h-40`); canvas widened 6× so lines don't wrap mid-ticker.

## Shipped this run (round 89)

- `thumb --scenes` — grabs a still at frame 0 + every scene change (`select=gt(scene,0.35)`), thumbnail-candidate mining for shorts/YouTube covers.
- `overlay --border PX [--border-color]` — ring around the overlay picture (PiP readability), `pad=` pre-chain; conflicts `--tile`.
- `audiogram --mode spectrum` — `showfreqs` frequency bars (the podcast spectrum look) alongside the showwaves modes.
- `subs --case upper|lower|title` — rewrites cue text case; works on `--burn` (via a temp .srt) and `--convert`.

## Shipped this run (round 88)

- `rotate --angle N` — free-angle dutch tilt (`rotate=a=`, canvas kept via `out_w=iw:out_h=ih`, black fill).
- `subs --burn --align left|center|right` — ASS Alignment 1–3 (7–9 with `--top`) for lower-third / reaction-style captions.
- `chapter --list` — dumps embedded marks (`ffprobe -show_chapters`, time + title) as JSON; pairs with `split --chapters`.
- `conform --anchor top|bottom|left|right` — letterbox anchor on the `--pad` path (slide the picture to an edge for title room).

## Shipped this run (round 87)

- `multicam --transition N` — `xfade`/`acrossfade` chains at every switch (offset `T_k - k·f`) instead of hard concat cuts; stays hard-cut at 0. `--keep-audio` still works.
- `title --wrap N` — greedy word-wrap at N chars before rasterization (long hooks stop overflowing the frame).
- `fade --dip T --dur N` — dip-to-color at T: `fade` out for N/2 ending at T, `fade` back in for N/2 (video + audio).
- `conform --blur` — letterbox filled by a blurred copy of the video (`split` → upscale `crop` + `gblur` bg + `overlay` fg) instead of `--pad` color.

## Shipped this run (round 86)

- `split --chapters` — cuts at the input's embedded chapter marks (`ffprobe -show_chapters`); lectures/courses/books split into one file per chapter.
- `replace --at T --dur N` — windowed audio swap: original track keeps playing outside `[at, at+dur)`, the new audio owns it inside (with `--fade` on the joints).
- `overlay --loop` — loops a short `--video` overlay to cover the whole base (`loop=loop=-1` + `shortest=1` on the composite, so output still ends with the base).
- `subs --burn --box` — `BorderStyle=3,BackColour` plate behind each burned line so captions read on busy frames.

## Shipped this run (round 85)

- `progress --bg COLOR` — a static full-width track bar behind the sliding fill (elapsed-vs-total readability on busy frames).
- `mix --fade N` — `afade` in/out on the B bed inside the `--at/--dur` window (bed stops slamming in and out).
- `insert --volume N` — `volume=` node on the spliced clip's `atrim` arm in both the concat and `--transition` paths (ad reads at half level or muted).
- `solid --fade N` — `fade` in/out on the generated card (with or without `--text`/gradient) for softer intro/outro cards.

## Shipped this run (round 84)

- `multicam --keep-audio` — video still flips angles at `--at` cuts but the audio map stays on camera A's track end-to-end (the interview standard; previously audio cut with the video).
- `conform --pad COLOR` — `pad=W:H:(ow-iw)/2:(oh-ih)/2:c` after the decrease-scale so `--size` output is exactly WxH with a chosen letterbox color (was silently smaller than spec).
- `channel --mode mix51` — ITU fold-down `pan=stereo|FL<FL+0.707*FC+0.707*BL+0.5*LFE|...` pulls dialogue-forward stereo out of 5.1 takes.
- `transcode --colors N` — same `palettegen max_colors=` knob as `extract --colors`, on the `transcode --preset gif` path.

## Shipped this run (round 83)

- `meta --copy SRC` — `-map_metadata 1 -map_chapters 1` pulls every tag + chapter mark out of a sibling export (re-renders stop losing titles and chapters).
- `extract --colors N` — `palettegen max_colors=` for `--gif`: 8-color palettes drop the file hard, 256 keeps gradients clean.
- `broll --loop` — video insert shorter than the cutaway window now replays via `loop=loop=-1:size=F` + restamp instead of freezing on its last frame.
- `align --window SECS` — caps the PCM decode for the cross-correlation search; a 2-hour multicam take no longer correlates over its full length.

## Shipped this run (round 82)

- `remux --audio` — rip just the audio track (`-map 0:a`, `-c:a copy` when the container holds the codec, else re-encode mp3/ogg/wav/aac to fit). Pulling a podcast track out of a recorded video needs no re-encode now.
- `hls --fmp4` — `-hls_segment_type fmp4` CMAF segments (`seg_*.m4s` + `init.mp4`) on the flat and ABR-ladder paths; one package plays on Safari/AirPlay where `.ts` is legacy.
- `loop --fade SECS` — seamless loops: N copies joined by `xfade=duration=F:offset=k*(len-F)` + `acrossfade` joints (re-encodes instead of the default concat copy). The loop point stops being a visible jump — GIF-style replay texture.
- `compress --res HEIGHT` — `scale=-2:H:force_original_aspect_ratio=decrease` folded into both the `--size` two-pass chain and the `--crf` one-pass chain. At small budgets (Discord 8MB on a long clip) downscaling beats starving the bitrate.

## Shipped this run (round 81)

- `subs --all` — extract every subtitle stream in one ffmpeg call (`stem_0.srt`…); `probe` now reports `subtitle_streams`.
- `mix --normalize` — amix `normalize=1` halves the sum when both tracks are hot.
- `fit --strength` — gblur sigma for `--fit blur` (default 30).
- `compress --crf` — quality mode: single-pass libx264 crf instead of the two-pass size budget (clap requirement relaxed to size|target|crf).

## Shipped this run (round 80)

- `silence --detect [--threshold --min]` — report-only silence ranges as JSON extras (no `-o`) so agents can plan cuts before `split --silence`/`jumpcut`.
- `thumb --count N` — N evenly-spaced stills (`stem_01..NN.jpg`, `fps=(n-0.5)/dur` so all frames land before EOF).
- `sheet --from/--to` — contact-sheet sampling window (timestamps re-anchor when `--time` stamps).
- `loudnorm --dynamic` — per-frame dynamic gain (`linear=false`) for speech that linear offset pumps on.

## Shipped this run (round 79)

- `vocal --amount 0..1` — partial center cancel keeps backing bleed; partial isolate blends toward center.
- `eq --tilt -10..10` — one-knob warm↔bright mapped onto the bass/treble shelves.
- `deinterlace --engine yadif|bwdif` — pick the smoother motion-compensated bwdif (nnedi dropped: aborts without an external weights file on both tested ffmpegs).
- `insert --dur N` — splice only the first N seconds of the clip, in both the cut and xfade paths.

## Shipped this run (round 78)

- `mix --duck` — sidechain ducking: B (music bed) compresses under A (voice).
- `compress --target discord|whatsapp|gmail` — platform size presets (8/16/25MB).
- `subs --burn --outline N` — burned-caption stroke width.

## Shipped this run (round 77)

- `insert --transition T --duration D` — xfade into and out of the spliced clip.
- `hls --audio-only` — -vn audio-only HLS (podcast/voice streams).
- `grid --fill` — crop tiles to fill the cell instead of letterboxing.

## Shipped this run (round 76)

- `loudnorm --measure` — report-only loudness (I/TP/LRA in extras, no -o).
- `subs --convert` — .srt ↔ .vtt cue-file conversion.
- `art --extract` — pull embedded cover out to an image.
- `countdown --position` — digit placement (9-pos map).

## Shipped this run (round 75)

- `hls --ladder 1080,720,480` — ABR variant playlists + `master.m3u8` (per-variant bitrate tiers + aac).
- `concat --level LUFS` — one-pass loudnorm on every input before joining.

## Shipped this run (round 74)

- `multicam` — two-camera angle switching across an aligned pair (`--at` cuts flip the angle).
- `insert` — splice a whole clip into the middle of a video at `--at` (scaled to base).
- `split --subs` — write a re-timed per-part .srt next to each split file.

## Shipped this run (round 73)

- `align` — auto multi-cam/recorder sync by audio cross-correlation; reports `offset_ms`.
- `scroll` — rolling end credits (text/file → bottom→top roll over `--dur`).
- `countdown --text` — static label during the count ("STARTING SOON").

## Shipped this run (round 72)

- `conform --crf` — x264 quality knob (0–51, default 18) for delivery exports.
- `grid --gap` — uniform pixel border around every tile.
- `chapter --import` — read chapter marks from a text file (`TIME|TITLE` or `TIME,TITLE`).


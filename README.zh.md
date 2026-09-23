# ffkit

[English](README.md) · [中文](README.zh.md)

给本地 AI Agent 用的 FFmpeg 桥梁：用户跟 Agent 聊要什么成片，Agent 先出方案，再调用 `ffkit` 的手去执行。本机 `ffmpeg` / `ffprobe` 是引擎。素材不上传。

不要把 skill 当成「模糊 / 暗角 / 黑白」按钮面板。多步成片写成 `pipeline` JSON，一次跑完。

## 依赖

- Rust 1.80+（从源码安装 CLI 时才需要）
- 本机 `ffmpeg` 和 `ffprobe` 在 `PATH` 上  
  macOS：`brew install ffmpeg`

能力面等于 **这一份 ffmpeg 的编译选项**。`ffkit doctor --json` 列出实际有的 encoder / filter。`caption --mode burn` 走 overlay 栅格化，不依赖 libass。

## 安装

从 [Releases](https://github.com/zh30/ffkit/releases/latest) 下载**对应系统的 zip**（附件里带 `ffkit` 二进制的那个，不要用 Source code zip），解压后：

```bash
./ffkit doctor --json          # 解压即可用
./install.sh                   # 拷到 PATH 并写入 Agent skill
ffkit version --check
```

| 系统 | 附件 |
|------|------|
| macOS Apple Silicon | `ffkit-*-aarch64-apple-darwin.zip` |
| macOS Intel | `ffkit-*-x86_64-apple-darwin.zip` |
| Linux x86_64 | `ffkit-*-x86_64-unknown-linux-gnu.zip` |

浏览器下载后 macOS 若拦截：`xattr -d com.apple.quarantine ffkit` 再 `./install.sh`。

从源码装（需要 Rust 1.80+）：

```bash
git clone https://github.com/zh30/ffkit && cd ffkit
cargo install --path .
ffkit install-skill
ffkit doctor --json
```

`install-skill` 把 `SKILL.md` 和 `references/` 写入已存在的宿主目录：

| 宿主 | 路径 |
|------|------|
| Grok | `~/.grok/skills/ffkit` |
| Claude Code | `~/.claude/skills/ffkit` |
| Codex | `~/.codex/skills/ffkit` |
| Cursor | `~/.cursor/skills/ffkit` |

Skill 与 CLI **共用一个 SemVer**（`Cargo.toml` 与 `SKILL.md` 的 `version:`，编译期核对）。查看与校验：

```bash
ffkit version --json
ffkit version --check    # 已安装的 skill 拷贝和二进制不一致则失败
```

`install-skill` 会在每个 skill 目录写入 `VERSION` 戳。若 `ffkit version --check` 失败，先 `ffkit install-skill` 再让 Agent reload。变更记在 [`CHANGELOG.md`](CHANGELOG.md)。

每次合入 `main` 都会打一个 GitHub Release（`vX.Y.Z`），附件是可解压即用的 zip。发新版：

```bash
# 在 PR 里：改代码/文档 → 写 CHANGELOG [Unreleased] →  bump
./scripts/bump-version.sh patch   # 或 minor / major / 0.2.0
cargo test
# 合入 main 后 Actions 打包并创建 Release；本地补打：
./scripts/pack-release.sh
./scripts/publish-release.sh
```

## 给 Agent 用

装好之后直接说自然语言即可，例如：

- 把这段 mp4 剪前 5 秒，做成 9:16
- 抽成 wav，响度对齐到 -16 LUFS
- 右上角加 logo.png，再转成 GIF

Agent 应加载 **ffkit** skill（`/ffkit` 或自动触发）：先跟用户把成片说清楚并给出方案，再用 `ffkit pipeline`（或单步动词）执行，最后对照**最初目标**用 `--json` 数字汇报。完整工作流在 [`SKILL.md`](SKILL.md)。

## 自己跑 CLI

旗标以 `ffkit <verb> --help` 为准。写文件的动词都有 `--dry-run`、`--json`、`--json-brief`、`--overwrite`、`--timeout`、`--progress`。数字只信 `--json`。默认不覆盖源文件。

```bash
ffkit probe clip.mp4 --json
ffkit pipeline plan.json --json    # 多步方案（$src / $in / expect）
ffkit look branded.mp4 --at 1 -o frame.png
```

### 动词

| 动词 | 做什么 |
|------|--------|
| `doctor` | 本机 ffmpeg 是否可用、有哪些 encoder/filter |
| `probe` | 时长、分辨率、编码、声道 |
| `look` | 联系表（`--tiles`）或指定时间点（`--at`，可重复） |
| `cut` | 剪切；默认无损 copy，`--accurate`、`--ranges`、`--drop` 才帧精确（边界支持 `end`：`T-end` 到结尾、`end-N` 最后 N 秒）；`--fade N` 切口淡入淡出 |
| `concat` | 拼接 N 段（任意 xfade `--transition`，逗号列表逐接缝选转场）；`--level -14` 先统一各段响度；`--gap N` 段间插入黑场+静音 ；`--audio-fade N` 接缝处音频淡化（边界淡化，时长/同步不变） |
| `fit` | 画幅 / 旋转 / 翻转（9:16、1:1、16:9…）；`--fit blur` 用模糊背景填满 ，`--position` 画面对齐黑边位置 ，`--strength` 模糊力度 |
| `extract` | 抓静帧或 `--gif` 动图（`--bounce` 往返循环） | `--at`（`end` = 最后一帧 / 最后 --dur 秒，逗号 = 每点一张静帧——`--gif` 时每点一条动图）、`--width`、`--fps` ，`--loop` GIF 循环次数、`--colors` 调色板大小 |
| `overlay` | logo/画中画；`--tile N` 全屏草稿水印 | logo、水印、画中画 | `--angle`、`--loop` 短视频循环、`--border` 画中画描边
| `broll` | 切入镜头（`--insert` 视频、`--still` 图片、`--motion kenburns` 推镜） | 切走 B-roll（`--insert --at --duration`）；口播声音和时长不变 ，`--audio` 听插播原声（`--volume` 音量） ，`--position` 画中画角位 + `--scale`，`--border` 描边、`--opacity` 半透明插播；`--at end` 片尾切入，逗号 `--at` 多处重复切入 |
| `caption` | 烧录字幕（`--srt`、`--chunk`、`--karaoke`、`--box-color` 底板、`--wrap` 折行、`--from/--to` 只烧窗口内字幕（支持 `end`/`end-N`）） ，`--fade` 淡入淡出、`--opacity` 半透明水印字幕 |
| `broll` | 切入镜头（`--insert` 视频、`--still` 图片、`--motion kenburns` 推镜） | 切走 B-roll（`--insert --at --duration`）；口播声音和时长不变 ，`--audio` 听插播原声（`--volume` 音量） ，`--position` 画中画角位 + `--scale`，`--border` 描边；`--at end` 片尾切入，逗号 `--at` 多处重复切入 |
| `caption` | 烧录字幕（`--srt`、`--chunk`、`--karaoke`、`--box-color` 底板、`--wrap` 折行、`--from/--to` 只烧窗口内字幕（支持 `end`/`end-N`）） ，`--fade` 淡入淡出、`--opacity` 半透明水印字幕 |
| `broll` | 切入镜头（`--insert` 视频、`--still` 图片、`--motion kenburns` 推镜） | 切走 B-roll（`--insert --at --duration`）；口播声音和时长不变 ，`--audio` 听插播原声（`--volume` 音量） ，`--position` 画中画角位 + `--scale`，`--border` 描边；`--at end` 片尾切入，逗号 `--at` 多处重复切入 |
| `caption` | 烧录字幕（`--srt`、`--chunk`、`--karaoke`（`--highlight` 已唱字高亮色）、`--box-color` 底板、`--wrap` 折行、`--from/--to` 只烧窗口内字幕（支持 `end`/`end-N`）） ，`--fade` 淡入淡出、`--opacity` 半透明水印字幕 |
| `loudnorm` | EBU R128 两遍响度归一（`--target spotify|podcast|broadcast`）；`--measure` 只测不写（`--gate N` 超过 N LUFS 即失败）；`--dynamic` 逐帧动态增益 |
| `denoise` | 音频降噪（`--strength`、`--highpass`、`--engine auto|wavel|fftdn` 指定引擎、`--at/--dur` 窗口），支持 `end`，逗号列表可多段 |
| `transcode` | h264/webm/`--preset gif`（`--fps`/`--width`/`--copy-audio`/`--colors`） | 预设 `h264`/`webm`/`gif`/`hevc`/`mp3`/`aac`/`wav`/`flac`/`opus`/`av1`/`prores`/`dnxhd`；`--fps` 也可给视频变速帧率；`--vbitrate` 峰值码率帽、`--abitrate` 音频码率（人声帖用 64k 更省）；`--alpha` 保留透明通道（webm/prores）；`--range limited|full` 标注色彩范围 |
| `compress` | 压到目标体积（`--size 10MB` 两遍、`--target discord|whatsapp|gmail` 平台预设）；`--crf` 画质单遍、`--res` 缩分辨率腾码率 |
| `deliver` | 一键平台成片（Reels / TikTok / Shorts 为 9:16，`square` 为 1:1，`youtube` 为 16:9；−14 LUFS；`--fps 60` 高帧率，`--crf` 画质，`--subs file.srt` 成片一步烧字幕） |

| `audiogram` | 波形视频 | `--mode`、`--scale` 幅度、`--split` 分声道、`--fscale` 频率轴（spectrum）、`--fps` 帧率、`--text`、`--bg`、`--progress` 进度条 ，`--subs` 烧字幕、`--from`/`--to` 只取一段（`--to` 可用 `end`），`--at a,b --dur N` 每点一条（`stem_N.mp4`）；`--mode spectrum` 频谱条、`--mode scope` 李萨如矢量示波、`--mode cqt` 钢琴卷帘频谱、`--mode spectro` 滚动频谱图 |

| `split` | 按 `--every`/`--at`/`--scenes`/`--size`/`--parts`/`--silence`/`--chapters` 内嵌章节切分；`--subs` 输出重定时分段 .srt；`--fade N` 每段首尾淡化 |
| `slideshow` | 图片 → 配乐幻灯视频（`--per` 每图秒数或 `--dur` 总时长、`--fade`/`--transition` 转场、`--motion kenburns` 推拉、`--audio` 配乐 + `--volume` 音量、`--size` 画布、`--bg` 底边色） |
| `speed` | 变速（`--factor`、`--at/--dur` 窗口（逗号列表可多段）、`--ramp` FROM,TO 渐变），支持 `end` |
| `music` | 铺 BGM，人声出现时压低配乐（`--track`、`--at/--dur` 窗口支持 `end`，逗号 `--at` 多处铺底如 `0,end` 首尾双 sting） |
| `key` | 绿幕合成：`--color` 抠掉后叠到 `--bg` 图片/视频上（`--similarity`、`--blend`、`--despill`、`--mode luma` 改按亮度带抠像（`--threshold` 定亮区/暗区背景）、`--at/--dur` 窗口抠像，支持逗号列表） |
| `grid` | 多画面宫格（`--layout`、`--audio` 选音轨、`--labels`、`--gap`、`--bg` 格缝颜色、`--fill` 裁满代替黑边、`--time` 每格叠加统一 mm:ss 时间戳） |
| `progress` | 任意边进度条，整段或定时窗口（`--color`、`--height`、`--edge` bottom/top/left/right、`--at`、`--dur`、`--reverse` 倒计时缩减、`--opacity` 半透明） |
| `freeze` | 定格画面（`--at` 逗号列表多处定格、`--dur`、`--end`、`--ease` 减速、`--reverse` 倒放、`--zoom` 推近定格） |
| `censor` | 区域打码（`--region x:y:w:h`，逗号列表可多处同时打码；`--mode` pixel|blur|solid（solid = 黑条遮盖）、`--strength` 强度、`--at`/`--dur`（逗号列表，需 `--dur`；`end` 可用）、`--shape circle` 椭圆遮罩） |
| `bleep` | 消音哔声：`--at`/`--dur` 选段（逗号列表可消多处；`end` 可用），`--freq`/`--level` 调音 |
| `boomerang` | 正放+倒放回弹循环（一段，社交平台常见玩法）（`--times` 循环次数） ，`--at/--dur` 局部往返——逗号列表可多处回弹，支持 `end` |
| `chapter` | 在 `TIME|TITLE` 写入章节或 `--import` 导入标记文件（支持 YouTube `H:MM:SS Title` 行）；`--auto` / `--export` ffmeta / `--yt` 导出 YouTube 描述格式；`--list`；`--remove`；`--shift` 平移标记 |
| `autocrop` | 自动检测并裁掉黑边（`cropdetect` 扫描 → `crop`；`--buffer N` 向外扩 N 像素） |
| `sheet` | 宫格预览图（`--cols`x`--rows`、`--time` 每格时间戳、`--title` 标题行、`--from`/`--to` 采样窗口） |
| `sprite` | 播放条预览雪碧图 + WebVTT（`--every` 间隔秒、`--width` 缩略图宽、`--cols`x`--rows` 每张格数、`--vtt` 路径、`--from`/`--to` 限定范围，支持 `end`）——播放器悬停预览 |
| `pitch` | ±12 半音变调不变速（`--at/--dur` 窗口，逗号列表）；`--formant` 保留人声音色不失真（需 librubberband） |
| `cutsil` | 音频掐头去尾静音（`--thresh` dB） |
| `channel --mode ms` | 解码 M/S 录音立体声回 L/R（stereotools ms>lr） |
| `channel` | 声道手术：`--mode dualmono|mono|swap|invert|mix51|pan|widen|split|ambience|mid|side`|haas|surround|base`（`split` 立体声→`_L/_R.wav` 双人声分轨）；`--pan -1..1` 声像定位（`base` 模式下 -1 折叠为单声道、+1 加宽）；`ambience --amount` 削侧链去房间混响；`haas` 延迟法立体声加宽；`surround` 立体声上混 5.1 |
| `eq` | 音频均衡：`--bass`/`--treble`/`--presence`、`--preset` dB（`--at`/`--dur` 局部均衡），`--band` 参量频段，`--curve` 手绘 F,G;F,G 曲线（firequalizer 插值），`--graphic` 18 段图示均衡，`--tilt` 暖↔亮，支持 `end`，逗号列表可多段 |
| `reverb` | 给人声加房间氛围：`--size room\|hall\|cave`，`--wet`（`--at`/`--dur` 局部回声），支持 `end`，逗号列表可多段；`--ir 文件.wav` 卷积混响（脉冲响应包：教堂/大厅/钢板），`--tail` 让尾音延出尾端 |
| `fx` | 音效机架：tremolo/vibrato/flanger/phaser/chorus/echo/lofi/radio/saturate/excite/bass/muffled/crystal/sub/crossfeed/autopan（`--kind`、`--strength`、`--at`/`--dur`），支持 `end`，逗号列表可多段 |
| `rotate` | 旋转 90/180/270 或镜像：`--deg`/`--flip`、`--angle` 任意角度倾斜、`--at`/`--dur` 窗口倾斜（支持逗号列表） |
| `delogo` | 抹掉烧录的台标/水印区域：`--x --y --w --h`，或 `--regions x:y:w:h,...` 一次抹多处；`--at`/`--dur` 只处理窗口，`--at end` 片尾（`--soft` 柔化去除、`--shape circle` 椭圆遮罩） |
| `meta` | 容器标签（`--title`/`--artist`/`--album`/`--genre`/`--date`/`--track`/`--comment`）+ `--rotate`、`--clear` 显示旋转，无损拷贝 |
| `subs` | 提取（`--stream`、`--all` 全部）/烧录/封装字幕（`--shift`（±N；`--from`/`--to` 可只平移窗口内字幕）/`--merge`/`--rate`、烧录样式 + `--outline`/`--box` 衬底/`--align`/`--from`/`--to` 窗口（支持 `end`/`end-N`）、`--margin` 像素边距、`--safe`）；`--convert` .srt↔.vtt 互转；`--case` 大小写；`--burn-si N` 直接烧内嵌第 N 条字幕轨；`--encoding gbk` 解码老编码字幕文件 |
| `thumb` | 抓封面帧（`--at`（`end` = 最后一帧，逗号 `--at` 每点一张）/`--frame`、`--count` 均布 N 张（`--from`/`--to` 限定范围，支持 `end`/`end-N`）、`--width`）→ jpg/png/webp；`--scenes` 场景切换抓帧 |
| `solid` | 纯色视频卡（`--color`、`--size`、`--dur`、`--fps` 帧率，可选静音轨）（`--gradient` 渐变、`--noise` 颗粒） |（`--color`/`--gradient` 支持颜色名与十六进制） ，`--text` 卡片文字（`--wrap` 折行、`--align` 对齐） |
| `replace` | 换音轨（`--mix`、`--duck`、`--fade`、`--loop` 短音源循环、`--at`/`--dur` 局部替换，逗号 `--at` 多段续铺）；也可 `--video` 换画面保音频 |
| `jumpcut` | 剪掉口播里的静音停顿 |
| `rough` | 长素材粗剪：先列出说话段落（`--json`），`-o` 再拼起来（`--merge N` 合并间隔小于 N 秒的段；`--by-scene` 场景切换处再切开；默认只编码要留下的段；`--copy` 无损但按关键帧） |
| `cover` | 封面静帧（`--at`（`end` = 最后一帧，逗号 = 每点一张）、`--blur` 模糊底填充、`--size` 画布——默认 1080x1920） |
| `fade` | 画面和声音淡入淡出（`--in` / `--out`，`--color` 淡出到白等、`--dip T` 场景闪黑转场，逗号列表多处闪黑；`--curve` 音频淡出曲线） |
| `title` | 标题卡烧录（`--at`——`end` = 片尾卡，逗号列表可多次闪现、`--fade`、`--outline`、`--box` 底板、`--wrap` 折行、`--align` 对齐、`--opacity` 半透明、`--margin` 角位像素边距） |
| `loop` | 把成片重复 N 遍（Shorts 循环加长）（`--from`/`--to` 只循环片段，支持 `end`，`--fade` 无缝衔接） |
| `stabilize` | 手持防抖（deshake）——`--rx`/`--ry` 搜索半径，`--edge` 边缘填充 blank|original|clamped|mirror |
| `reverse` | 倒放画面和声音 |
| `grade` | `--preset` 一键风格 + 对比/饱和/亮度/`--gamma`/`--hue`/`--exposure`（EV 档）, `--at`/`--dur`/`--warm`；`--lut look.cube` 套 3D LUT | 预设 `cinematic`/`vivid`/`vintage`/`soft`/`sepia`/`teal`/`noir`/`bleach`/`neon` 叠在滑杆之下 |
| `zoom` | 推近（`--factor 1.25`、`--center X,Y` 靶点；`--at`/`--dur` 局部窗口——逗号列表可多段，支持 `end`） | `--out`
| `sharpen` | USM 锐化，整段或定时窗口（`--amount`、`--at`、`--dur`） |
| `vignette` | 暗角，整段或定时窗口（`--angle`、`--at`、`--dur`） |
| `bw` | 黑白化，整段或定时窗口（`--at`、`--dur`、`--strength` 保留部分色彩） |
| `volume` | 音量 ±dB；`--at/--dur` 局部增益，逗号列表可作用多处（需 `--dur`）（平台响度请用 `loudnorm`） |
| `blur` | 全帧或定时高斯模糊（`--sigma`、`--at`/`--dur`；`--at end` = 片尾） |
| `trail` | 运动拖影：`--mode echo` 跟随残影（`--frames` 2-16、`--at`/`--dur` 窗口），`--mode light` 亮部拖尾（`--decay` 0.5-0.99） |
| `glitch` | 故障风 RGB 错位：`--strength` 0.5-20 控制通道偏移+噪点强度 |
| `bars` | SMPTE 测试卡：`--size`/`--dur`/`--hd`/`--tone`（1kHz 音床），用于质检片头 |
| `scope --mode hist` | 亮度时间直方图——检查长时间曝光/色彩漂移 |
| `scope` | QC 示波器叠加：`--mode vector|wave` 角落小窗（`--position`、`--size` 占比）、`--at` 窗口 |
| `desqueeze` | 变形宽银幕还原：`--factor` 镜头倍率（1.33/1.5/1.8/2.0）、`--axis y|x` |
| `solarize` | 迷幻局部反色：高于 `--threshold` 亮度的像素反色，`--at` 窗口 |
| `pulse` | 呼吸变焦：`--rate` 每秒周期、`--depth` 幅度、`--at` 窗口 |
| `deflicker` | 延时摄影去闪：`--size` 帧时域亮度平滑 |
| `emboss` | 浮雕效果：卷积核，`--amount` 混合原图，`--at` 窗口 |
| `tilt` | 移轴微缩：上下条带虚化，`--band` 清晰区比例、`--blur` 强度、`--at` 窗口 |
| `sway` | 手持漂移：正弦漫游裁切，`--rate`/`--px`，`--at` 窗口 |
| `rack` | 呼吸变焦虚化：正弦混合模糊副本，`--rate`/`--blur` |
| `outline` | 墨线描边：检测边缘压黑，`--strength` 阈值，`--at` 窗口 |
| `night` | 夜视效果：绿色调+噪点+暗角，`--at` 窗口 |
| `snow` | 落雪叠加：滚动噪点色键覆盖，`--density`/`--speed`，`--at` 窗口 |
| `impact` | 节拍冲击：白闪 + 衰减正弦晃动，`--at` 时刻，`--amp`/`--flash` |
| `wave` | 水波横向扭曲（geq 重采样），`--amp`/`--speed`，`--at` 窗口 |
| `spin` | 钟摆式摇摆：画面按缓正弦旋转，`--deg`/`--rate`，`--at` 窗口 |
| `iris` | 聚光圆盘：圆外画面变暗去饱和，`--x`/`--y`/`--radius`，`--at` 窗口 |
| `burst` | 径向变焦拖影：放大模糊副本叠在清晰画面下，`--strength` |
| `thump` | `--at` 时刻低音冲击：55Hz 正弦带快速衰减混入音轨，`--freq`/`--gain`/`--dur` |
| `riser` | 上行音调扫频（200→2000Hz）在 `--at` 时刻落地，`--dur` 上升时长，`--gain` |
| `whoosh` | 棕色噪声气声渐强在 `--at` 落地（转场音效），`--dur`/`--gain` |
| `deesser` | 人声去齿音：压制 4-8kHz 齿音频段，`--amount`/`--freq`/`--at` 窗口 |
| `declip` | 修复削波爆音与爆点：`--engine clip` = `adeclip` 插值重建压平波形峰，`--engine click` = `adeclick` 去黑胶爆点/数字丢样（`--window` 毫秒、`--threshold` 1-100、`--overlap-save`、`--at` 窗口） |
| `deband` | `gradfun` 平滑天空/背景色带，`--strength`/`--radius`/`--at` 窗口 |
| `deblock` | 消除高压缩素材的 DCT 块边界（`--strength` 检测强度、`--at` 窗口） |
| `chromashift` | 按像素平移色度平面，修磁带/采集彩色镶边（`--x`/`--y`、`--edge` wrap/smear、`--at` 窗口） |
| `stack` | 3+ 个同机位视频的中值堆叠——移除只在部分输入出现的东西（游客、传感器噪点）；`--percentile`；音频取自第 1 个输入 |
| `tmedian` | 时间中值——抹去窗口内出现不足一半时长的东西：三脚架画面中的行人/车辆、雨丝（`--radius` 历史帧、`--percentile`、`--at` 窗口；输出首尾各丢 radius 帧） |
| `dedup` | `mpdecimate` 丢弃近似重复帧——压缩静态片段，`--frac` 灵敏度 |
| `audiogram --mode cqt` | 恒 Q 音乐频谱（`showcqt`）——钢琴卷帘式频谱，适合音乐片段 |
| `audiogram --mode spectro` | 滚动频谱图（`showspectrum`）——彩色时频滚动 |
| `scan` | 质检：报告黑屏/冻结区间 + 频闪帧（`flash_frames`/`flash_max_badness`，光敏性癫痫检查；JSON extras，不写媒体） |
| `smooth` | 边缘保留美颜/皮肤模糊（`--engine` smartblur/bilateral——bilateral 边缘更锐利；`--strength`、`--at/--dur` 窗口） |
| `upscale` | 老素材升分辨率：`zscale` spline36（优于 lanczos）+ 轻度锐化，`--factor` 1.05-4（2 = 宽高翻倍），`--strength` 边缘锐度 |
| `v360` | 360° 画面重取景为平面（`--in` 支持 equirect/fisheye/dfisheye/cubemap/EAC/barrel/半等距，`--yaw`/`--pitch`/`--fov`、`--size`） |
| `perspective` | 斜拍屏幕/白板矫正：`--points x0,y0,x1,y1,x2,y2,x3,y3`（源画面四角 TL,TR,BL,BR，像素），`--interp linear|cubic` |
| `wb` | 自动白平衡/去色偏：`--strength` 0..1，`--independence 0` 只调对比保色调，`--smooth` 时间平滑帧，`--at`/`--dur` 局部 |
| `shear` | 画面倾斜（斜体字效果）：`--x`/`--y` 剪切系数 -2..2，`--fill` 边缘填充色，`--interp nearest|bilinear`，`--at`/`--dur` 局部 |
| `gen` | 生成式动态背景（无需输入文件，lavfi 源）：`--pattern mandelbrot`（无限缩放分形）`|gradients`（渐变漂移，`--colors`/`--seed`/`--speed`）`|life`（细胞自动机，`--rule`），`--size`/`--fps`/`--dur` |
| `sharpen --engine cas` | 对比自适应锐化——边缘更脆且无 unsharp 光晕，`--amount` |
| `equalize` | `histeq` 自动对比度，修复灰暗/洗白画面，`--strength`/`--intensity`/`--at` 窗口 |
| `pick` | 主色提取：某时刻均色 + 3x2 分区色板（仅 JSON 报告） |
| `diff` | 双片视觉差异：放大差值混合，`--side` 并排参考 |
| `selective` | 单色保留：仅留 `--color` 其余去饱和，`--similarity` 容差，`--blend` 边缘羽化，`--at` 窗口 |
| `amplify` | 运动放大——细微帧间变化变可见（`--amount` 倍数、`--radius` 历史帧、`--threshold` 差值上限、`--at` 窗口） |
| `cartoon` | 漫画效果：色块化（`--levels` 2-16）+ 边缘墨线，`--at` 窗口 |
| `heat` | 热成像伪彩：`--preset` magma|inferno|plasma|viridis|turbo|cividis|range1|range2|shadows|highlights、`--opacity`、`--at` 窗口 |
| `kaleido` | 左上象限镜像成 2x2 曼陀罗，`--at` 窗口 |
| `strobe` | MV 频闪剪切：`--rate` 每秒闪数、`--duty` 占空比、`--color`、`--at` 窗口 |
| `edge` | 霓虹描边：`--mode wires|colormix`、`--low`/`--high` 阈值、`--at` 窗口 |
| `lens` | 镜头畸变：`--k1`/`--k2` —— 负值鱼眼效果，正值运动相机去鱼眼；`--at` 窗口 |
| `mirror` | 半画面中心镜像（`--axis x`/`y`、`--at`/`--dur` 窗口）——舞蹈/对称效果 |
| `pix` | 复古马赛克像素化：`--strength` 2-64 分块因子（`--at`/`--dur` 窗口） |
| `flip` | 水平/垂直翻转（`--axis x` 自拍去镜像、`--at`/`--dur` 窗口） |
| `poster` | 波普海报化：`--levels` 2-64 调色板色数（`--at`/`--dur` 窗口） |
| `duotone` | 双色调映射：`--shadow`/`--highlight` 亮度渐变（`--at`/`--dur` 窗口） |
| `glow` | 梦幻泛光：模糊副本 screen 混合叠加（`--strength`、`--at`/`--dur` 窗口） |
| `vhs` | 复古磁带：`--strength` 0-3 噪点+色偏+扫描线（`--at`/`--dur` 窗口） |
| `motionblur` | 快门拖影：`--frames` 2-8 帧间混合（`--at`/`--dur` 窗口） |
| `vdenoise` | 视频降噪（`--engine nlmeans|hqdn3d|atadenoise|vaguedenoise|bm3d|dctdnoiz|owdenoise|median|chroma`、`--strength`、`--at` 窗口），支持 `end`，逗号列表可多段 |
| `crop` | 裁剪 `--region` 区域，或 `--aspect` 重构（`--anchor center|top|bottom|left|right` 可选锚点） |
| `waveform` | 音频波形 → PNG（`--size`、`--color`, `--scale`、`--peak` 峰值、`--split` 逐声道、`--full` 密集、`--bg` 不透明底卡、`--vertical` 竖向波形（自上而下，PNG 为 高×宽）），播客封面/缩略图用（`--at/--dur` 只画片段，支持 `end`，逗号 `--at` 每窗一张 `<stem>_N.png`） |
| `spectrogram` | 音频频谱图 → PNG（`--size`），清理前先看嗡鸣/噪声（`--color` magma/viridis…、`--scale` lin/sqrt…、`--no-legend` 去图例、`--separate` 逐声道分带）（`--at/--dur` 只画片段，支持 `end`，逗号 `--at` 每窗一张 `<stem>_N.png`） |
| `meter` | EBU R128 实时响度表视频（`--size`、`--meter 9\|18`、`--at/--dur` 只测片段）——边听边看 I/TP/LRA |
| `dehum` | 市电嗡鸣陷波（`--mains 50|60` 或 `--freq HZ` 自定义频率、`--harmonics`、`--at/--dur`，支持 `end`，逗号列表可多段） |
| `tempo` | 音频变速 `--factor` 0.5–8，不变调（`atempo` 链；视频请用 `speed`） ，`--at/--dur` 局部变速——逗号列表可多段，支持 `end` |
| `leveler --engine mcompand` | 多段压缩预设——低频/人声/空气感分带，抬弱声压峰头 |
| `leveler` | 动态压平（`--preset`、`--engine speechnorm` 自适应人声归一、`--at/--dur` 窗口），支持 `end`，逗号列表可多段 |
| `gate` | 噪声门——低于 `--threshold` dB 的部分静音（`agate`）（`--preset voice|podcast|studio`，`--at/--dur` 局部生效），支持 `end`，逗号列表可多段 |
| `silence` | 在 `--at`（逗号列表多处）或 `--end` 插入 `--dur` 秒静音；`--detect` 以 JSON 报告静音区间 |
| `vocal` | 消/留中置人声（`--mode`、`--amount` 强度、`--at/--dur` 窗口），支持 `end`，逗号列表可多段 |
| `remux` | 换容器不重编码（`-c copy` + faststart）；`--audio` 只提音轨，`--video` 只留视频，`--aspect 16:9` 修显示宽高比 |
| `meme` | 上下说明文字梗图（`--outline`、`--at/--dur` 时间窗，逗号列表可打多处；`--at end` 片尾） ，`--position` 文字块上/中/下；`--wrap` 折行、`--align` 行对齐、`--fade` 窗口边缘淡入淡出（配 --at/--dur）、`--opacity` 半透明文字 |
| `voice` | 播客人声一条龙：`agate` 去嘶声 → `acompressor` 压平 → `loudnorm` 响度（`--threshold`、`--lufs`、`--at`/`--dur` 只处理一段，支持 `end`，逗号列表可多段） |
| `deinterlace` | 修复隔行素材（`--mode`、`--parity` 场序、`--engine` yadif/bwdif/estdif/kerndeint） |
| `crossfade` | 两段音频淡接，`--dur` 秒重叠（`acrossfade`） |
| `strip` | 去掉全部元数据/章节（发片前隐私清理），无损 `-c copy` |
| `frames` | 每 `--every`、`--at` 秒抽一帧（`end` = 最后一帧）、`--count` 均布 N 帧 → `stem_001.png…`（`--width` 缩放） |
| `countdown` | 画面倒数（`--from` 最多 600、`--beep` + `--tone` 蜂鸣频率、`--text`、`--position`、`--bg` 数字底板、`--format` mm:ss/h:mm:ss 长倒计时、`--opacity` 半透明） |
| `invert` | 全帧或定时反色（`--at`、`--dur`——逗号列表可多段） |
| `mix` | 双音轨叠加（`--vol-a/--vol-b`、`--at/--dur`（逗号列表可多次进床）、`--loop`、`--duck` 人声闪避音乐） ，`--normalize` 归一求和、`--fade` 淡入淡出，支持 `end` |
| `mute` | 去掉音轨（其余流直接封装，不重编码） ，`--at/--dur` 局部静音（逗号列表可静多处，需 `--dur`），支持 `end` |
| `timer` | 画面计时器（`--position`、`--format ms`、`--box-color` 底板） （`--format`、`--box-color`、`--down` 倒计时、`--start` 设定起始读数、`--opacity` 半透明）——`--at` 支持 `end` |
| `hls` | 网页 HLS 封装（`--seg`、`--single`、`--copy`、`--ladder` 多码率、`--audio-only` 纯音频、`--fmp4` CMAF、`--poster` 同时输出 poster.jpg 封面，`--poster-at T` 选封面帧，`--encrypt` AES-128 加密分片并写 key.bin/key.info（`--key HEX` 自定义密钥、`--key-uri URI` 播放列表里的密钥地址）→ 私有/付费流） |

| `qa` | 对比参考视频测画质损失（PSNR + SSIM，`--metric`） |
| `conform` | 一键统一规格（`--size WxH`、`--fps 30`、`--lufs -14`、`--crf`、`--pad` 黑边颜色 + `--anchor` 锚点、`--blur` 模糊填充） |
| `sync` | 修复音画同步（`--ms ±N` 垫音/裁音头） |
| `align` | 音频互相关自动对齐第二路录音（多机位/外接录音笔，`--max-lag`） |
| `scroll` | 片尾滚动字幕（`--text`/`--file`、`--at`、`--dur` 或 `--speed` px/s、`--size`、`--color`、`--font`、`--align` 对齐、`--wrap` 折行）；`--mode ticker` 底部新闻条可加 `--bg` 不透明底条；`--at` 逗号列表可多次复播，`end` 亦可；`--opacity` 半透明字幕 |
| `insert` | 在视频中段插入整段素材（`--at`，逗号列表多点插入，`end` 追加到片尾；`--dur` 只取前 N 秒；`--transition` 转场 + `--duration` 两端淡入淡出） |
| `multicam` | 双机位对齐后角度切换：`--at t1,t2,...` 逐点换机位（`end` = 片尾切回）；`--keep-audio` 全程用 A 机位音轨、`--transition` 软切换 |
| `art` | 给音频嵌入封面图；`--extract` 反向导出封面 |
| `batch` | 对目录里每个媒体文件跑同一个动词 |
| `pipeline` | 按 JSON 方案顺序执行多步（`$src` / `$in` / `expect`） |
| `graph` | JSON 滤镜图，见 [`references/graph.md`](references/graph.md) |
| `ffmpeg` | 受保护的原生 ffmpeg，**必须** `--because REASON` |
| `install-skill` | 把 skill 写入宿主目录 |
| `version` | 二进制 / 嵌入 skill / 已安装拷贝；`--check` 校验 |

对话出方案，方案落成 `pipeline`。单步用动词；没有动词再用 `graph`，再不行才 `ffmpeg --because`（写清哪个动词或 graph 字段不够）。

## 开发

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt
```

改动词时同步改 `SKILL.md` Hands 表。行为或安装方式变了，同一 PR 里同步改 `README.md` **和** `README.zh.md`，以及 `SKILL.md` / `references/` / CHANGELOG。版本只通过 `scripts/bump-version.sh` 改，不要手改一处漏另一处。每个合入 `main` 的变更都必须 bump，合入后必须有对应的 GitHub Release。

SemVer：MAJOR = 破坏 CLI 或 skill 工作流（删动词、改 JSON 合同）；MINOR = 新动词或新能力；PATCH = 修复与文档。

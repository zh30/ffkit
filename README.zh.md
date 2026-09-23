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
| `cut` | 剪切；默认无损 copy，`--accurate`、`--ranges`、`--drop` 才帧精确 |
| `concat` | 拼接 N 段（任意 xfade `--transition`、`--audio-fade`）；`--level -14` 先统一各段响度 |
| `fit` | 画幅 / 旋转 / 翻转（9:16、1:1、16:9…）；`--fit blur` 用模糊背景填满 ，`--position` 画面对齐黑边位置 ，`--strength` 模糊力度 |
| `extract` | 抓静帧或 `--gif` 动图（`--bounce` 往返循环） | `--at`、`--dur`、`--width`、`--fps` ，`--loop` GIF 循环次数、`--colors` 调色板大小 |
| `overlay` | logo/画中画；`--tile N` 全屏草稿水印 | logo、水印、画中画 | `--angle`、`--loop` 短视频循环、`--border` 画中画描边
| `broll` | 切入镜头（`--insert` 视频、`--still` 图片、`--motion kenburns` 推镜） | 切走 B-roll（`--insert --at --duration`）；口播声音和时长不变 ，`--audio` 听插播原声（`--volume` 音量） ，`--position` 画中画角位 + `--scale`，`--border` 描边；`--at end` 片尾切入 |
| `caption` | 烧录字幕（`--srt`、`--chunk`、`--karaoke`、`--box-color` 底板、`--wrap` 折行、`--from/--to` 只烧窗口内字幕） ，`--fade` 淡入淡出 |
| `loudnorm` | EBU R128 两遍响度归一（`--target spotify|podcast|broadcast`）；`--measure` 只测不写；`--dynamic` 逐帧动态增益 |
| `denoise` | 音频降噪（`--strength`、`--highpass`、`--at/--dur` 窗口） |
| `transcode` | h264/webm/`--preset gif`（`--fps`/`--width`/`--copy-audio`/`--colors`） | 预设 `h264`/`webm`/`gif`/`hevc`；`--fps` 也可给视频变速帧率 ，`--preset prores` 剪辑交付；`av1` 预设；`--alpha` 保留透明通道（webm/prores） |
| `compress` | 压到目标体积（`--size 10MB` 两遍、`--target discord|whatsapp|gmail` 平台预设）；`--crf` 画质单遍、`--res` 缩分辨率腾码率 |
| `deliver` | 一键 9:16 社交成片（Reels / TikTok / Shorts，−14 LUFS） |

| `audiogram` | 波形视频 | `--mode`、`--scale` 幅度、`--split` 分声道、`--fscale` 频率轴（spectrum）、`--fps` 帧率、`--text`、`--bg`、`--progress` 进度条 ，`--subs` 烧字幕；`--mode spectrum` 频谱条 |

| `split` | 按 `--every`/`--at`/`--scenes`/`--size`/`--parts`/`--silence`/`--chapters` 内嵌章节切分；`--subs` 输出重定时分段 .srt |
| `slideshow` | 图片 → 配乐幻灯视频（`--per` 每图秒数、`--fade`/`--transition` 转场、`--motion kenburns` 推拉、`--audio` 配乐 + `--volume` 音量、`--size` 画布） |
| `speed` | 变速（`--factor`、`--at/--dur` 窗口、`--ramp` FROM,TO 渐变） |
| `music` | 铺 BGM，人声出现时压低配乐（`--track`） |
| `key` | 绿幕合成：`--color` 抠掉后叠到 `--bg` 图片/视频上（`--similarity`、`--blend`、`--despill` 去边缘绿色） |
| `grid` | 多画面宫格（`--layout`、`--audio` 选音轨、`--labels`、`--gap`、`--fill` 裁满代替黑边） |
| `progress` | 任意边进度条，整段或定时窗口（`--color`、`--height`、`--edge` bottom/top/left/right、`--at`、`--dur`） |
| `freeze` | 定格画面（`--at`、`--dur`、`--end`、`--ease` 减速、`--reverse` 倒放） |
| `censor` | 区域打码（`--mode`、`--strength` 强度、`--at`） |
| `bleep` | 消音哔声：`--at`/`--dur` 选段，`--freq`/`--level` 调音 |
| `boomerang` | 正放+倒放回弹循环（一段，社交平台常见玩法）（`--times` 循环次数） ，`--at/--dur` 局部往返 |
| `chapter` | 在 `TIME|TITLE` 写入章节或 `--import` 导入标记文件；`--auto` / `--export`；`--list` 列出；`--remove` 清除全部章节；`--shift` 整体平移标记 |
| `autocrop` | 自动检测并裁掉黑边（`cropdetect` 扫描 → `crop`；`--buffer N` 向外扩 N 像素） |
| `sheet` | 宫格预览图（`--cols`x`--rows`、`--time` 每格时间戳、`--title` 标题行、`--from`/`--to` 采样窗口） |
| `pitch` | ±12 半音变调不变速（`--at/--dur` 窗口） |
| `cutsil` | 音频掐头去尾静音（`--thresh` dB） |
| `channel` | 声道手术：`--mode dualmono|mono|swap|invert|mix51`；`widen` 立体声加宽 |
| `eq` | 音频均衡：`--bass`/`--treble`/`--presence`、`--preset` dB（`--at`/`--dur` 局部均衡） ，`--band` 参量频段，`--tilt` 暖↔亮 |
| `reverb` | 给人声加房间氛围：`--size room\|hall\|cave`，`--wet`（`--at`/`--dur` 局部回声） |
| `fx` | 音效机架：tremolo/vibrato/flanger/phaser/chorus/echo/lofi/radio（`--kind`、`--strength`、`--at`/`--dur`） |
| `rotate` | 旋转 90/180/270 或镜像：`--deg`/`--flip`、`--angle` 任意角度倾斜 |
| `delogo` | 抹掉烧录的台标/水印区域：`--x --y --w --h`；`--at`/`--dur` 只处理窗口（`--soft` 柔化去除） |
| `meta` | 容器标签（`--title`/`--artist`/`--album`/`--genre`/`--date`/`--track`/`--comment`）+ `--rotate`、`--clear` 显示旋转，无损拷贝 |
| `subs` | 提取（`--stream`、`--all` 全部）/烧录/封装字幕（`--shift/--merge/--rate`、烧录样式 + `--outline`/`--box` 衬底/`--align`/`--from`/`--to` 窗口、`--safe`）；`--convert` .srt↔.vtt 互转；`--case` 大小写 |
| `thumb` | 抓封面帧（`--at`/`--frame`、`--count` 均布 N 张、`--width`）→ jpg/png/webp；`--scenes` 场景切换抓帧 |
| `solid` | 纯色视频卡（`--color`、`--size`、`--dur`、`--fps` 帧率，可选静音轨）（`--gradient` 渐变、`--noise` 颗粒） |（`--color`/`--gradient` 支持颜色名与十六进制） ，`--text` 卡片文字（`--wrap` 折行、`--align` 对齐） |
| `replace` | 换音轨（`--mix`、`--duck`、`--fade`、`--loop` 短音源循环、`--at`/`--dur` 局部替换） |
| `jumpcut` | 剪掉口播里的静音停顿 |
| `rough` | 长素材粗剪：先列出说话段落（`--json`），`-o` 再拼起来（`--merge N` 合并间隔小于 N 秒的段；`--by-scene` 场景切换处再切开；默认只编码要留下的段；`--copy` 无损但按关键帧） |
| `cover` | 9:16 封面静帧（`--at`、`--blur` 模糊底填充） |
| `fade` | 画面和声音淡入淡出（`--in` / `--out`，`--color` 淡出到白等、`--dip T` 场景闪黑转场） |
| `title` | 标题卡烧录（`--at`、`--fade`、`--outline`、`--box` 底板、`--wrap` 折行、`--align` 对齐、`--opacity` 半透明） |
| `loop` | 把成片重复 N 遍（Shorts 循环加长）（`--from`/`--to` 只循环片段，`--fade` 无缝衔接） |
| `stabilize` | 手持防抖（deshake）——`--rx`/`--ry` 搜索半径，`--edge` 边缘填充 blank|original|clamped|mirror |
| `reverse` | 倒放画面和声音 |
| `grade` | `--preset` 一键风格 + 对比/饱和/亮度/`--gamma`/`--hue`, `--at`/`--dur`/`--warm`；`--lut look.cube` 套 3D LUT | 预设 `cinematic`/`vivid`/`vintage`/`soft` 叠在滑杆之下 |
| `zoom` | 中心推近（`--factor 1.25`） | `--out`
| `sharpen` | USM 锐化，整段或定时窗口（`--amount`、`--at`、`--dur`） |
| `vignette` | 暗角，整段或定时窗口（`--angle`、`--at`、`--dur`） |
| `bw` | 黑白化，整段或定时窗口（`--at`、`--dur`、`--strength` 保留部分色彩） |
| `volume` | 音量 ±dB（平台响度请用 `loudnorm`） |
| `blur` | 全帧或定时高斯模糊（`--sigma`、`--at`、`--dur`） |
| `vdenoise` | 视频降噪（暗光噪点）：`--strength` 0.5–30（nlmeans，长片较慢） |
| `crop` | 裁剪 `--region` 区域，或 `--aspect` 重构（`--anchor center|top|bottom|left|right` 可选锚点） |
| `waveform` | 音频波形 → PNG（`--size`、`--color`, `--scale`、`--peak` 峰值、`--split` 逐声道、`--full` 密集、`--bg` 不透明底卡），播客封面/缩略图用（`--at/--dur` 只画片段） |
| `spectrogram` | 音频频谱图 → PNG（`--size`），清理前先看嗡鸣/噪声（`--color` magma/viridis…、`--scale` lin/sqrt…、`--no-legend` 去图例）（`--at/--dur` 只画片段） |
| `meter` | EBU R128 实时响度表视频（`--size`、`--meter 9\|18`、`--at/--dur` 只测片段）——边听边看 I/TP/LRA |
| `dehum` | 市电嗡鸣陷波（`--mains 50|60` 或 `--freq HZ` 自定义频率、`--harmonics`、`--at/--dur`） |
| `tempo` | 音频变速 `--factor` 0.5–8，不变调（`atempo` 链；视频请用 `speed`） ，`--at/--dur` 局部变速 |
| `leveler` | 动态压平（`--preset`、`--at/--dur` 窗口） |
| `gate` | 噪声门——低于 `--threshold` dB 的部分静音（`agate`）（`--preset voice|podcast|studio`，`--at/--dur` 局部生效） |
| `silence` | 在 `--at`/`--end` 插入 `--dur` 秒静音；`--detect` 以 JSON 报告静音区间 |
| `vocal` | 消/留中置人声（`--mode`、`--amount` 强度、`--at/--dur` 窗口） |
| `remux` | 换容器不重编码（mkv→mp4 等，`-c copy` + faststart）；`--audio` 只提音轨，`--video` 只留视频 |
| `meme` | 上下说明文字梗图（`--outline`、`--at/--dur` 时间窗） ，`--position` 文字块上/中/下；`--wrap` 折行、`--align` 行对齐 |
| `voice` | 播客人声一条龙：`agate` 去嘶声 → `acompressor` 压平 → `loudnorm` 响度（`--threshold`、`--lufs`） |
| `deinterlace` | 修复隔行素材（`--mode`、`--parity` 场序、`--engine` yadif/bwdif） |
| `crossfade` | 两段音频淡接，`--dur` 秒重叠（`acrossfade`） |
| `strip` | 去掉全部元数据/章节（发片前隐私清理），无损 `-c copy` |
| `frames` | 每 `--every`、`--at` 秒抽一帧 → `stem_001.png…`（`--width` 缩放） |
| `countdown` | 画面倒数（`--from`、`--beep` + `--tone` 蜂鸣频率、`--text`、`--position`、`--bg` 数字底板） |
| `invert` | 全帧或定时反色（`--at`、`--dur`） |
| `mix` | 双音轨叠加（`--vol-a/--vol-b`、`--at/--dur`、`--loop`、`--duck` 人声闪避音乐） ，`--normalize` 归一求和、`--fade` 淡入淡出 |
| `mute` | 去掉音轨（其余流直接封装，不重编码） ，`--at/--dur` 局部静音 |
| `timer` | 画面计时器（`--position`、`--format ms`、`--box-color` 底板） （`--format`、`--box-color`、`--down` 倒计时、`--start` 设定起始读数） |
| `hls` | 网页 HLS 封装（`--seg`、`--single`、`--copy`、`--ladder` 多码率、`--audio-only` 纯音频、`--fmp4` CMAF） |
| `qa` | 对比参考视频测画质损失（PSNR + SSIM，`--metric`） |
| `conform` | 一键统一规格（`--size WxH`、`--fps 30`、`--lufs -14`、`--crf`、`--pad` 黑边颜色 + `--anchor` 锚点、`--blur` 模糊填充） |
| `sync` | 修复音画同步（`--ms ±N` 垫音/裁音头） |
| `align` | 音频互相关自动对齐第二路录音（多机位/外接录音笔，`--max-lag`） |
| `scroll` | 片尾滚动字幕（`--text`/`--file`、`--at`、`--dur`、`--size`、`--color`、`--font`、`--align` 对齐、`--wrap` 折行）；`--mode ticker` 底部新闻条可加 `--bg` 不透明底条 |
| `insert` | 在视频中段插入整段素材（`--at`，`end` 追加到片尾；`--dur` 只取前 N 秒；`--transition` 转场 + `--duration` 两端淡入淡出） |
| `multicam` | 双机位对齐后角度切换：`--at t1,t2,...` 逐点换机位；`--keep-audio` 全程用 A 机位音轨、`--transition` 软切换 |
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

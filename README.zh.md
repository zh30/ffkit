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
| `concat` | 拼接；`--transition` 在每条之间做 xfade 转场（视频 xfade + 音频 acrossfade，支持 N 条） |
| `fit` | 画幅 / 旋转 / 翻转（9:16、1:1、16:9…）；`--fit blur` 用模糊背景填满 |
| `extract` | 按输出扩展名抽音频、帧、字幕 | `--width`
| `overlay` | logo/画中画；`--tile N` 全屏草稿水印 | logo、水印、画中画 | `--angle`
| `broll` | 切入镜头（`--insert` 视频、`--still` 图片、`--motion kenburns` 推镜） | 切走 B-roll（`--insert --at --duration`）；口播声音和时长不变 |
| `caption` | 烧录/封装 `.srt`；`--chunk` 分词、`--shift` 整体平移、`--position top` 置顶 | `--mode mux` 软字幕；`burn` 烧入（overlay 栅格化，不依赖 libass）；`--safe social` 避开底部 20%；`--chunk N` 按 ≤N 词切分字幕 |
| `loudnorm` | EBU R128 两遍（`-I -14` 社交，`-I -16` 播客） |
| `denoise` | 人声降噪（风扇/轰隆/嘶嘶声）；`--video` 顺带画面去噪点 |
| `transcode` | h264/webm/`--preset gif`（`--fps`/`--width`） | 预设 `h264` / `webm` / `gif`；`--fps` 也可给视频变速帧率 |
| `compress` | 两遍编码压到 `--size 10MB`（Discord、WhatsApp 16MB、邮箱约 25MB） |
| `deliver` | 一键 9:16 社交成片（Reels / TikTok / Shorts，−14 LUFS） |

| `audiogram` | 播客音频 → 9:16 波形视频（`--image` 封面，`--mode`/`--color`、`--bg`、`--size`、`--text`, `--position` 波形样式） |

| `split` | 切成分段（`--every` 等长、`--at` 章节点、`--scenes` 镜头、`--size` 大小、`--parts N` 等分）→ `stem_00..` |
| `slideshow` | 图片 → 配乐幻灯视频（`--per` 每图秒数、`--fade`/`--transition` 转场、`--motion kenburns` 推拉、`--audio` 配乐、`--size` 画布） |
| `speed` | 变速（`--factor 2` 加速一倍，口播保持音调） |
| `music` | 铺 BGM，人声出现时压低配乐（`--track`） |
| `key` | 绿幕合成：`--color` 抠掉后叠到 `--bg` 图片/视频上（`--similarity`、`--blend`、`--despill` 去边缘绿色） |
| `grid` | N 路素材拼 `--layout CxR` 宫格（`--size 1920x1080`）；都有音轨时混音输出 |
| `progress` | 底/顶部进度条，整段或定时窗口（`--color`、`--height`、`--edge`、`--at`、`--dur`） |
| `freeze` | `--at T --dur D` 定格某一拍，或 `--end D` 尾帧定格（结尾停留） |
| `censor` | 马赛克/高斯模糊打码区域 `--region x:y:w:h`（`--mode pixel|blur`；`--at`/`--dur` 限定窗口） |
| `bleep` | 消音哔声：`--at`/`--dur` 选段，`--freq`/`--level` 调音 |
| `boomerang` | 正放+倒放回弹循环（一段，社交平台常见玩法） |
| `chapter` | 写入 `--at T|TITLE` 章节标记（`-c copy` 无损元数据） |
| `autocrop` | 自动检测并裁掉黑边（`cropdetect` 扫描 → `crop`） |
| `sheet` | 缩略图墙/预览拼图：`--cols`/`--rows` → 一张 PNG | `--pad`/`--margin`
| `pitch` | `--semitones N` 升降调（时长不变） |
| `cutsil` | 音频掐头去尾静音（`--thresh` dB） |
| `channel` | 声道手术：`--mode dualmono|mono|swap` |
| `eq` | 音频均衡：`--bass`/`--treble`/`--presence`、`--preset` dB |
| `reverb` | 给人声加房间氛围：`--size room\|hall\|cave`，`--wet` |
| `rotate` | 旋转 90/180/270 或镜像：`--deg`/`--flip` |
| `delogo` | 抹掉烧录的台标/水印区域：`--x --y --w --h`；`--at`/`--dur` 只处理窗口 |
| `meta` | 容器标签（`--title`/`--artist`/`--album`/`--genre`/`--date`/`--track`/`--comment`）+ `--rotate`、`--clear` 显示旋转，无损拷贝 |
| `subs` | 抽取内嵌字幕（`--stream`）、`--burn` 压制字幕、`--shift ±N` 整体平移 .srt |
| `thumb` | 抓封面单帧（`--at`/`--frame`、`--width`）→ jpg/png/webp |
| `solid` | 纯色视频卡（`--color`、`--size`、`--dur`，可选静音轨） |
| `replace` | 换掉视频音轨为 `--audio`（领夹麦/干净人声/新配乐）；`--audio-offset` 对齐、`--mix` 保留原音轨 |
| `jumpcut` | 剪掉口播里的静音停顿 |
| `rough` | 长素材粗剪：先列出说话段落（`--json`），`-o` 再拼起来（默认只编码要留下的段；`--copy` 无损但按关键帧） |
| `cover` | 导出 9:16 封面图（1080×1920） |
| `fade` | 画面和声音淡入淡出（`--in` / `--out`，`--color` 淡出到白等） |
| `title` | 屏幕标题 PNG（`--at`、`--position` 含四角、`--fade`、`--outline`, `--shadow`） |
| `loop` | 把成片重复 N 遍（Shorts 循环加长） |
| `stabilize` | 手持防抖（deshake） |
| `reverse` | 倒放画面和声音 |
| `grade` | `--preset` 一键风格 + 对比/饱和/亮度/`--gamma`/`--hue`, `--at`/`--dur`/`--warm`；`--lut look.cube` 套 3D LUT | 预设 `cinematic`/`vivid`/`vintage`/`soft` 叠在滑杆之下 |
| `zoom` | 中心推近（`--factor 1.25`） | `--out`
| `sharpen` | USM 锐化，整段或定时窗口（`--amount`、`--at`、`--dur`） |
| `vignette` | 暗角，整段或定时窗口（`--angle`、`--at`、`--dur`） |
| `bw` | 黑白化，整段或定时窗口（`--at`、`--dur`） |
| `volume` | 音量 ±dB（平台响度请用 `loudnorm`） |
| `blur` | 全帧或定时高斯模糊（`--sigma`、`--at`、`--dur`） |
| `vdenoise` | 视频降噪（暗光噪点）：`--strength` 0.5–30（nlmeans，长片较慢） |
| `crop` | 裁剪 `--region` 区域，或 `--aspect` 重构（`--anchor center|top|bottom|left|right` 可选锚点） |
| `waveform` | 音频波形 → PNG（`--size`、`--color`, `--scale`），播客封面/缩略图用 |
| `spectrogram` | 音频频谱图 → PNG（`--size`），清理前先看嗡鸣/噪声（`--color` magma/viridis…） |
| `dehum` | 消除市电嗡鸣 `--mains 50|60` + `--harmonics`（Q=12 `equalizer` 陷波链） |
| `tempo` | 音频变速 `--factor` 0.5–8，不变调（`atempo` 链；视频请用 `speed`） |
| `leveler` | 人声动态压缩（忽大忽小）：`--threshold`/`--ratio`/`--makeup`（`--preset` voice/podcast/master） |
| `gate` | 噪声门——低于 `--threshold` dB 的部分静音（`agate`） |
| `silence` | 在 `--at` 处插入 `--dur` 秒静音，或 `--end` 追加片尾静音 |
| `vocal` | `--mode karaoke` 消中置人声、`isolate` 只留中置（需立体声源） |
| `remux` | 换容器不重编码（mkv→mp4 等，`-c copy` + faststart） |
| `meme` | 顶部/底部说明文字烧入（`--top`/`--bottom`，`--color`，`--size`） |
| `voice` | 播客人声一条龙：`agate` 去嘶声 → `acompressor` 压平 → `loudnorm` 响度（`--threshold`、`--lufs`） |
| `deinterlace` | 老录像/DV 隔行转逐行（`--mode frame`/`field`） |
| `crossfade` | 两段音频淡接，`--dur` 秒重叠（`acrossfade`） |
| `strip` | 去掉全部元数据/章节（发片前隐私清理），无损 `-c copy` |
| `frames` | 每 `--every`、`--at` 秒抽一帧 → `stem_001.png…`（`--width` 缩放） |
| `countdown` | 片头 3-2-1(-GO) 倒数遮罩（`--from`、`--each`、`--go`、`--at`）（`--beep` 滴答声） |
| `invert` | 全帧或定时反色（`--at`、`--dur`） |
| `mix` | 两段音频等权叠加（`--vol-a`/`--vol-b` 线性电平，`--longest` 按长者收尾） |
| `mute` | 去掉音轨（其余流直接封装，不重编码） |
| `timer` | 角落计时器烧录（`--position`、`--at`、`--dur`） |
| `hls` | 打包成 `index.m3u8` + `seg_*.ts`（`--seg` 秒数） |
| `qa` | 对比参考视频测画质损失（PSNR + SSIM，`--metric`） |
| `conform` | 统一素材规格便于拼接（`--size`、`--fps`、`--lufs`） |
| `sync` | 修复音画同步（`--ms ±N` 垫音/裁音头） |
| `art` | 内嵌封面图（`--image`）→ mp3/m4a/mp4/mkv |
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

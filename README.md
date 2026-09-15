# ffkit

给本地 AI Agent 用的 FFmpeg 桥梁：skill 是大脑，Rust CLI 是手，本机 `ffmpeg` / `ffprobe` 是引擎。素材不上传、不走云 API。

Agent 接到「剪前 5 秒、做成 9:16、加 logo」时，应走 `ffkit`，而不是现场拼一条 ffmpeg。

## 依赖

- Rust 1.80+（安装 CLI）
- 本机 `ffmpeg` 和 `ffprobe` 在 `PATH` 上  
  macOS：`brew install ffmpeg`

能力面等于 **这一份 ffmpeg 的编译选项**。`ffkit doctor --json` 列出实际有的 encoder / filter。烧字幕（`caption --mode burn`）需要带 libass 的构建；没有就用 `--mode mux`。

## 安装

```bash
git clone <this-repo> && cd ffkit
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

发新版：

```bash
./scripts/bump-version.sh patch   # 或 minor / major / 0.2.0
cargo test
cargo install --path . --force
ffkit install-skill
# 可选：git tag v$(ffkit --version | awk '{print $2}')
```

`install-skill` 会在每个 skill 目录写入 `VERSION` 戳。若 `ffkit version --check` 失败，先 `ffkit install-skill` 再让 Agent reload。变更记在 [`CHANGELOG.md`](CHANGELOG.md)。

## 给 Agent 用

装好之后直接说自然语言即可，例如：

- 把这段 mp4 剪前 5 秒，做成 9:16
- 抽成 wav，响度对齐到 -16 LUFS
- 右上角加 logo.png，再转成 GIF

Agent 应加载 **ffkit** skill（`/ffkit` 或自动触发），按 probe → 动词 → look → JSON 数字汇报。完整工作流在 [`SKILL.md`](SKILL.md)。

## 自己跑 CLI

旗标以 `ffkit <verb> --help` 为准。写文件的动词都有 `--dry-run`、`--json`、`--json-brief`、`--overwrite`、`--timeout`、`--progress`。数字只信 `--json`。默认不覆盖源文件。

```bash
ffkit probe clip.mp4 --json
ffkit cut clip.mp4 --start 0 --end 5 -o cut.mp4 --json
ffkit fit cut.mp4 --aspect 9:16 --fit pad -o vertical.mp4 --json
ffkit overlay vertical.mp4 --image logo.png --position top-right -o branded.mp4 --json
ffkit look branded.mp4 --at 1 -o frame.png
ffkit transcode clip.mp4 --preset gif -o preview.gif --json
```

### 动词

| 动词 | 做什么 |
|------|--------|
| `doctor` | 本机 ffmpeg 是否可用、有哪些 encoder/filter |
| `probe` | 时长、分辨率、编码、声道 |
| `look` | 联系表（`--tiles`）或指定时间点（`--at`，可重复） |
| `cut` | 剪切；默认无损 copy，`--accurate` 才帧精确 |
| `concat` | 拼接；两条且 `--transition fade` 为交叉淡化 |
| `fit` | 画幅 / 旋转 / 翻转（9:16、1:1、16:9…） |
| `extract` | 按输出扩展名抽音频、帧、字幕 |
| `overlay` | logo、水印、画中画 |
| `caption` | `--mode mux` 软字幕；`burn` 烧入（overlay 栅格化，不依赖 libass） |
| `loudnorm` | EBU R128 两遍（`-I -14` 社交，`-I -16` 播客） |
| `transcode` | 预设 `h264` / `webm` / `gif` |
| `deliver` | 一键 9:16 社交成片（Reels / TikTok / Shorts，−14 LUFS） |
| `speed` | 变速（`--factor 2` 加速一倍，口播保持音调） |
| `music` | 铺 BGM，人声出现时压低配乐（`--track`） |
| `jumpcut` | 剪掉口播里的静音停顿 |
| `cover` | 导出 9:16 封面图（1080×1920） |
| `fade` | 画面和声音淡入淡出（`--in` / `--out`） |
| `title` | 片头/hook 大字（`--text`，不依赖 libass） |
| `loop` | 把成片重复 N 遍（Shorts 循环加长） |
| `stabilize` | 手持防抖（deshake） |
| `reverse` | 倒放画面和声音 |
| `grade` | 调色（对比/饱和/亮度，Reels 默认微抬） |
| `zoom` | 中心推近（`--factor 1.25`） |
| `sharpen` | 锐化（unsharp） |
| `vignette` | 暗角 |
| `bw` | 黑白 |
| `batch` | 对目录里每个媒体文件跑同一个动词 |
| `graph` | JSON 滤镜图，见 [`references/graph.md`](references/graph.md) |
| `ffmpeg` | 受保护的原生 ffmpeg，**必须** `--because REASON` |
| `install-skill` | 把 skill 写入宿主目录 |
| `version` | 二进制 / 嵌入 skill / 已安装拷贝；`--check` 校验 |

三层路由：动词 → `graph` → `ffkit ffmpeg --because … --`。最后一层要写清「哪个动词或 graph 字段不够」。

## 开发

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt
```

改动词时同步改 `SKILL.md` 路由表。版本只通过 `scripts/bump-version.sh` 改，不要手改一处漏另一处。

SemVer：MAJOR = 破坏 CLI 或 skill 工作流（删动词、改 JSON 合同）；MINOR = 新动词或新能力；PATCH = 修复与文档。

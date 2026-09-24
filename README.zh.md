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
| `probe` | 时长、分辨率、编码、声道、`av_desync_ms` 唇音同步偏移、`timecode` 容器时码、`has_alpha` α 通道、`tags` 元数据审计、`streams` 逐轨清单（index/kind/codec/语种/default 默认轨）、`rotation` 显示矩阵旋转质检、`start_time` 最早流起始时间（采集文件负值/异常起点——`remux --offset` 修） |
| `look` | 联系表（`--tiles`）或指定时间点（`--at`，可重复） |
| `cut` | 剪切；默认无损 copy，`--accurate`、`--ranges`、`--drop` 才帧精确（边界支持 `end`：`T-end` 到结尾、`end-N` 最后 N 秒）；`--fade N` 切口淡入淡出；`--black` 自动切掉 blackdetect ≥0.3s 黑段（死画面剔除，画面版的 cutsil） |
| `concat` | 拼接 N 段（任意 xfade `--transition`，逗号列表逐接缝选转场）；`--level -14` 先统一各段响度；`--gap N` 段间插入黑场+静音 ；`--audio-fade N` 接缝处音频淡化（边界淡化，时长/同步不变）；`--list manifest.txt` 从清单文件读取片段路径（相对路径按清单所在目录解析） |
| `fit` | 画幅 / 旋转 / 翻转（9:16、1:1、16:9…）；`--fit blur` 用模糊背景填满 ，`--position` 画面对齐黑边位置 ，`--strength` 模糊力度 |
| `extract` | 抓静帧或 `--gif` 动图（`--bounce` 往返循环） | `--at`（`end` = 最后一帧 / 最后 --dur 秒，逗号 = 每点一张静帧——`--gif` 时每点一条动图）、`--width`、`--fps` ，`--loop` GIF 循环次数、`--colors` 调色板大小、`--alpha` 提取 α 通道为灰度 PNG（仅限带透明通道输入）、`--audio` 无损抽音轨（流拷贝——`-o` 扩展名选容器，拉音乐/对白轨给剪辑用；`--track N` 选第 N 轨）、`--subs` 抽内嵌字幕轨为文本文件（`-o` .srt/.ass/.vtt 选字幕容器；`--track N` 选语种——成片字幕抽出改时轴/重烧录）、`--chapter N` 按内嵌章节抽段成单独文件（1 基编号即 `chapter --list` 所见——有声书/讲座分段导出）、`--keyframes` 把每个关键帧（I 帧）各导出一张图（裸 `-o` 自动补 `stem_%03d`——GOP 边界静帧：关键帧间隔质检/延时摄影源/场景速览）、`--gif --transparent` GIF 保留透明通道（Discord/Telegram 贴纸——需 prores 4444/qtrle 等含 α 输入）、`--webp` 动画 WebP 片段（比 GIF 小且原生保 α；`--lossless`、`--bounce` 可用） |
| `overlay` | logo/画中画；`--tile N` 全屏草稿水印 | logo、水印、画中画 | `--angle`、`--loop` 短视频循环、`--border` 画中画描边、`--mode` 混合合成（screen/multiply/softlight/dodge/burn/exclusion/hardmix/negation/grainmerge/multiply128/and/xor/freeze/heat… 31 种 Photoshop 式混合 + 颗粒合成 + 逻辑通道） |
| `broll` | 切入镜头（`--insert` 视频、`--still` 图片、`--motion kenburns` 推镜） | 切走 B-roll（`--insert --at --duration`）；口播声音和时长不变 ，`--audio` 听插播原声（`--volume` 音量） ，`--position` 画中画角位 + `--scale`，`--border` 描边、`--opacity` 半透明插播；`--at end` 片尾切入，逗号 `--at` 多处重复切入 |
| `caption` | 烧录字幕（`--srt`、`--chunk`、`--karaoke`、`--box-color` 底板、`--wrap` 折行、`--from/--to` 只烧窗口内字幕（支持 `end`/`end-N`）） ，`--fade` 淡入淡出、`--opacity` 半透明水印字幕 |
| `broll` | 切入镜头（`--insert` 视频、`--still` 图片、`--motion kenburns` 推镜） | 切走 B-roll（`--insert --at --duration`）；口播声音和时长不变 ，`--audio` 听插播原声（`--volume` 音量） ，`--position` 画中画角位 + `--scale`，`--border` 描边；`--at end` 片尾切入，逗号 `--at` 多处重复切入 |
| `caption` | 烧录字幕（`--srt`、`--chunk`、`--karaoke`、`--box-color` 底板、`--wrap` 折行、`--from/--to` 只烧窗口内字幕（支持 `end`/`end-N`）） ，`--fade` 淡入淡出、`--opacity` 半透明水印字幕 |
| `broll` | 切入镜头（`--insert` 视频、`--still` 图片、`--motion kenburns` 推镜） | 切走 B-roll（`--insert --at --duration`）；口播声音和时长不变 ，`--audio` 听插播原声（`--volume` 音量） ，`--position` 画中画角位 + `--scale`，`--border` 描边；`--at end` 片尾切入，逗号 `--at` 多处重复切入 |
| `caption` | 烧录字幕（`--srt`、`--chunk`、`--karaoke`（`--highlight` 已唱字高亮色）、`--box-color` 底板、`--wrap` 折行、`--from/--to` 只烧窗口内字幕（支持 `end`/`end-N`）） ，`--fade` 淡入淡出、`--opacity` 半透明水印字幕 |
| `loudnorm` | EBU R128 两遍响度归一（`--target spotify|podcast|broadcast`）；`--measure` 只测不写（`--gate N` 超过 N LUFS 即失败）；`--dynamic` 逐帧动态增益 |
| `denoise` | 音频降噪（`--strength`、`--highpass`、`--engine auto|wavel|fftdn` 指定引擎、`--at/--dur` 窗口），支持 `end`，逗号列表可多段 | `--ref noise.wav` anlms 自适应消噪 |
| `transcode` | h264/webm/`--preset gif`（`--fps`/`--width`/`--copy-audio`/`--colors`） | 预设 `h264`/`webm`/`gif`/`hevc`/`mp3`/`aac`/`wav`/`flac`/`opus`/`av1`/`prores`/`dnxhd`/`proxy`（`proxy` = ≤540p veryfast x264 剪辑代理，多机位素材丝滑预监）；`--fps` 也可给视频变速帧率；`--vbitrate` 峰值码率帽、`--abitrate` 音频码率（人声帖用 64k 更省）；`--alpha` 保留透明通道（webm/prores）；`--range limited|full` 标注色彩范围；`--interlaced` 标记输出为隔行（il 场交织 + tff 标记，广播母带交付）；`--interlace-mode weave` 真时域隔行（60p→30i 逐帧织场）；`--field-order tff|bff|prog` 不重织场、只改场序标签修标错的母带；`--ar 48000`/`--channels 1|2` 重采样率+声道数（广播 48k 立体声、播客单声道）；`--copy-video` 流拷贝画面只重编码音频（修坏音轨/重打包——视频滤镜参数不适用） |
| `compress` | 压到目标体积（`--size 10MB` 两遍、`--target discord|whatsapp|gmail` 平台预设）；`--crf` 画质单遍、`--res` 缩分辨率腾码率 |
| `deliver` | 一键平台成片（Reels / TikTok / Shorts 为 9:16，`square` 为 1:1，`youtube` 为 16:9，`xhs` 小红书 3:4 1080x1440，`wechat` 视频号 6:7 1080x1260，`douyin` 抖音/`kuaishou` 快手 9:16 1080x1920，`bilibili` B站 16:9 1920x1080，`pinterest` 2:3 1000x1500，`x` 16:9 1280x720，`linkedin`/`vimeo`/`bluesky` 16:9 1920x1080，`threads` 4:5 1080x1350，`mastodon` 16:9 1280x720，`circle` 1:1 640x640 单声道——Telegram 视频便签，`canvas`/`snapchat` 9:16 1080x1920——Spotify Canvas 循环 / Spotlight，`weibo` 微博 16:9 1920x1080；−14 LUFS；`--fps 60` 高帧率，`--crf` 画质，`--subs file.srt` 成片一步烧字幕，`--channels 1` 单声道语音成片，`--preview SEC` 只渲染成片头部给审核 QC，`--logo mark.png` 角落水印随成片一道烧入（`--logo-position` 四角、`--logo-opacity` 透明），`--intro/--outro clip.mp4` 频道片头+片尾卡自动贴到每个导出上（统一归一到平台画布）、`--lufs -16` 覆盖平台响度目标（自定交付规格））；`--platform podcast` 纯音频播客成片（m4a AAC 128k/48k，−16 LUFS 播客平台标准，`--cover art.png` 内嵌 Apple/Spotify 封面）或 `--platform audiobook`（m4b AAC 96k——Apple Books/Audible 有声书）；音频成片吃 `--chapters marks.txt`（容器章节——喂的就是 `chapter --yt` 导出的 `mm:ss 标题` 列表）和作品标签 `--title/--author/--album/--genre/--comment`；`--to rtmp://…`/`tcp://`/`udp://` 把成片直接推给采集端（首播） |

| `audiogram` | 波形视频 ，`--mode phase`（aphasemeter 相位表）| `--mode`、`--scale` 幅度、`--split` 分声道、`--fscale` 频率轴（spectrum）、`--fps` 帧率、`--text`、`--bg`、`--progress` 进度条 ，`--subs` 烧字幕、`--from`/`--to` 只取一段（`--to` 可用 `end`），`--at a,b --dur N` 每点一条（`stem_N.mp4`）；`--mode spectrum` 频谱条、`--mode scope` 李萨如矢量示波、`--mode cqt` 钢琴卷帘频谱、`--mode spectro` 滚动频谱图 | `--mode spatial|volume|bitscope` 表桥示波 | `--mode monitor` 管线统计 | `--mode hist` 振幅直方图（削波/余量质检） |

| `split` | 按 `--every`/`--at`/`--scenes`/`--size`/`--parts`/`--silence`/`--chapters` 内嵌章节/`--black`（blackdetect 死画面→每段非黑保留区各出一片）/`--copy`（segment muxer 无损流拷贝切分，秒级完成但边界对齐下一关键帧）切分；`--subs` 输出重定时分段 .srt；`--fade N` 每段首尾淡化 |
| `slideshow` | 图片 → 配乐幻灯视频（`--per` 每图秒数或 `--dur` 总时长、`--fade`/`--transition` 转场、`--motion kenburns` 推拉、`--audio` 配乐 + `--volume` 音量、`--size` 画布、`--bg` 底边色、`--fit` 蒙太奇正好随歌曲收尾、`--shuffle SEED` 确定性乱序——同种子同顺序，图片堆蒙太奇、`--sort name|mtime` 按文件名或拍摄时间排列相机导出照片、`--list manifest.txt` 清单文件自排顺序、`--titles a,,c` 每张图底部字幕条（逗号槽位按最终图片顺序——空槽跳过该图） |
| `speed` | 变速（`--factor`、`--at/--dur` 窗口（逗号列表可多段）、`--ramp` FROM,TO 渐变），支持 `end`；`--fit SEC` 精确压到目标时长（自动算倍数——90s 素材 `--fit 15` 即 6 倍速） |
| `music` | 铺 BGM，人声出现时压低配乐（`--track`、`--at/--dur` 窗口支持 `end`，逗号 `--at` 多处铺底如 `0,end` 首尾双 sting） |
| `key` | 绿幕合成：`--color` 抠掉后叠到 `--bg` 图片/视频上（`--similarity`、`--blend`、`--despill`、`--mode luma` 改按亮度带抠像（`--threshold` 定亮区/暗区背景）、`--mode chroma` YUV 色域抠像（广播级 chromakey，褶皱/光照不匀的幕布更稳）、`--at/--dur` 窗口抠像，支持逗号列表） | `--mode matte --mask` 外部灰度遮罩→α 通道（prores 4444） |
| `grid` | 多画面宫格（`--layout`、`--audio` 选音轨、`--labels`、`--gap`、`--bg` 格缝颜色、`--fill` 裁满代替黑边、`--time` 每格叠加统一 mm:ss 时间戳、`--focus` 主角布局——首输入占左 2/3 大格，其余纵向排右侧） |
| `progress` | 任意边进度条，整段或定时窗口（`--color`、`--height`、`--edge` bottom/top/left/right、`--at`、`--dur`、`--reverse` 倒计时缩减、`--opacity` 半透明） |
| `freeze` | 定格画面（`--at` 逗号列表多处定格、`--dur`、`--end`、`--ease` 减速、`--reverse` 倒放、`--zoom` 推近定格） |
| `censor` | 区域打码（`--region x:y:w:h`，逗号列表可多处同时打码；`--mode` pixel|blur|solid（solid = 黑条遮盖）、`--strength` 强度、`--at`/`--dur`（逗号列表，需 `--dur`；`end` 可用）、`--shape circle` 椭圆遮罩） |
| `bleep` | 消音哔声：`--at`/`--dur` 选段（逗号列表可消多处；`end` 可用），`--freq`/`--level` 调音 |
| `boomerang` | 正放+倒放回弹循环（一段，社交平台常见玩法）（`--times` 循环次数） ，`--at/--dur` 局部往返——逗号列表可多处回弹，支持 `end` |
| `chapter` | 在 `TIME|TITLE` 写入章节或 `--import` 导入标记文件（支持 YouTube `H:MM:SS Title` 行）；`--auto` / `--export` ffmeta / `--yt` 导出 YouTube 描述格式 / `--cue` CUE 表 / `--podcast` Podcasting 2.0 JSON 章节（有声书/播客播放器）/ `--lrc` LRC 同步歌词标记文件 / `--vtt` WebVTT 章节文件（网页 `<track kind="chapters">` 点击跳段）；`--spread N` 等距网格标记（`--titles a,b,c` 命名——长节目统一目录）；`--import` 自动识别 .json/.cue/.lrc/.vtt 文件；`--list`；`--remove`；`--shift` 平移标记 |
| `autocrop` | 自动检测并裁掉黑边（`cropdetect` 扫描 → `crop`；`--buffer N` 向外扩 N 像素） |
| `sheet` | 宫格预览图（`--cols`x`--rows`、`--time` 每格时间戳、`--title` 标题行、`--from`/`--to` 采样窗口） |
| `sprite` | 播放条预览雪碧图 + WebVTT（`--every` 间隔秒、`--width` 缩略图宽、`--cols`x`--rows` 每张格数、`--vtt` 路径、`--from`/`--to` 限定范围，支持 `end`）——播放器悬停预览 |
| `pitch` | ±12 半音变调不变速（`--at/--dur` 窗口，逗号列表）；`--formant` 保留人声音色不失真（需 librubberband） |
| `cutsil` | 音频掐头去尾静音（`--thresh` dB） |
| `channel --mode ms` | 解码 M/S 录音立体声回 L/R（stereotools ms>lr） |
| `channel` | 声道手术：`--mode dualmono|mono|swap|invert|mix51|pan|widen|split|ambience|mid|side|haas|surround|base|bal|bands|sync|earwax|stereowiden|merge`（`split` 立体声→`_L/_R.wav` 双人声分轨）；`--pan -1..1` 声像定位（`bal` 校正偏听立体声，`base` 模式下 -1 折叠为单声道、+1 加宽）；`ambience --amount` 削侧链去房间混响；`haas` 延迟法立体声加宽；`surround` 立体声上混 5.1；`bands --freqs 300,3000` → `<stem>_bandN.wav` 频段分轨（acrossover，重混低/中/高）；`sync --side right --cm 34` 按拾音距离延迟单侧声道（双麦克梳状滤波修复，34cm≈1ms）；`earwax` 耳机向立体声加宽；`stereowiden` M/S 加宽（`--amount` 控制 crossfeed）；`merge --with B` 双轨交织为一份多声道文件（amerge——单声道+单声道→立体声 主播L/嘉宾R 播客，与 mix 不同它保持声道独立） |
| `eq` | 音频均衡：`--bass`/`--treble`/`--presence`、`--preset` dB（`--at`/`--dur` 局部均衡），`--band` 参量频段，`--curve` 手绘 F,G;F,G 曲线（firequalizer 插值），`--graphic` 18 段图示均衡，`--tilt` 暖↔亮，`--deemph riaa/cd/fm50/fm75` 去黑胶/调频/CD 预加重，`--shelf low|high:FREQ:GAIN` 架式滤波（低频隆隆声削/空气感），`--notch FREQ[:WIDTH]` 陷波除共振，`--brickwall LO,HI` FFT 砖墙带通（电话音/语音带 300,3400），`--lowpass`/`--highpass`/`--bandpass FREQ[:W]` 巴特沃斯谐振滤波（`--linear` = sinc+afir 线性相位 ~60dB 阻带母带级切频，不支持 `--at`），`--subcut`/`--supercut FREQ` 话筒架隆隆声/超声嘶声清理，`--superpass FREQ[:Q]`/`--superstop FREQ` 十阶剃刀频段分离/清除，`--allpass FREQ:W` 相位旋转修不对称人声波形挣余量，支持 `end`，逗号列表可多段 |
| `reverb` | 给人声加房间氛围：`--size room\|hall\|cave`，`--wet`（`--at`/`--dur` 局部回声），支持 `end`，逗号列表可多段；`--ir 文件.wav` 卷积混响（脉冲响应包：教堂/大厅/钢板），`--tail` 让尾音延出尾端 |
| `fx` | 音效机架：tremolo/vibrato/flanger/phaser/chorus/echo/lofi/radio/saturate/excite/bass/muffled/crystal/sub/crossfeed/autopan（`--kind`、`--strength`、`--at`/`--dur`），支持 `end`，逗号列表可多段 | `--kind ringmod` 真环形调制（amultiply 乘正弦载波，`--strength` 扫 25-500Hz） | `--kind crush` 位深+采样率破坏（数字低保真） | `--kind fshift` 移频（金属外星声，`--strength` 扫 50→2000Hz） | `--kind contrast` 动态倾斜（>0.5 更冲击，<0.5 更平稳） | `--kind wah` 自动哇音（asendcmd 驱动谐振 equalizer 峰在 350→2700Hz 扫动，`--strength` 控 LFO 速率） |
| `rotate` | 旋转 90/180/270 或镜像：`--deg`/`--flip`、`--angle` 任意角度倾斜、`--at`/`--dur` 窗口倾斜（支持逗号列表） |
| `delogo` | 抹掉烧录的台标/水印区域：`--x --y --w --h`，或 `--regions x:y:w:h,...` 一次抹多处；`--at`/`--dur` 只处理窗口，`--at end` 片尾（`--soft` 柔化去除、`--shape circle` 椭圆遮罩） | `--image` 手绘遮罩 | `--find logo.png` 自动定位（find_rect 扫前 15 秒，免手填坐标） | `--find + --track` 逐帧追踪移动水印（cover_rect 实时模糊） |
| `meta` | 容器标签（`--title`/`--artist`/`--album`/`--genre`/`--date`/`--track`/`--disc`/`--composer`/`--bpm`/`--lyrics file.lrc`/`--copyright`/`--comment`，另加 `--album-artist`/`--show`/`--season`/`--episode`/`--network` 剧集/播客订阅源标签组，再补 `--creation-time`/`--location` 归档时间戳与拍摄地戳 + `--media-type` iTunes 类型原子 + `--gapless` 无缝专辑标记 + `--description`/`--synopsis` 单集文案与简介 + `--hd` iTunes 高清徽标）+ `--rotate`、`--clear` 显示旋转，无损拷贝 |
| `subs` | 提取（`--stream`、`--all` 全部）/烧录/封装字幕（`--shift`（±N；`--from`/`--to` 可只平移窗口内字幕）/`--merge`/`--rate`、烧录样式 + `--outline`/`--box` 衬底/`--align`/`--from`/`--to` 窗口（支持 `end`/`end-N`）、`--margin` 像素边距、`--safe`）；`--convert` .srt↔.vtt 互转；`--case` 大小写；`--burn-si N` 直接烧内嵌第 N 条字幕轨；`--encoding gbk` 解码老编码字幕文件 |
| `thumb` | 抓封面帧（`--at`（`end` = 最后一帧，逗号 `--at` 每点一张）/`--frame`、`--count` 均布 N 张（`--from`/`--to` 限定范围，支持 `end`/`end-N`）、`--width`）→ jpg/png/webp；`--scenes` 场景切换抓帧 | `--best` 代表帧 |
| `solid` | 纯色视频卡（`--color`、`--size`、`--dur`、`--fps` 帧率，可选静音轨）（`--gradient` 渐变、`--noise` 颗粒） |（`--color`/`--gradient` 支持颜色名与十六进制） ，`--text` 卡片文字（`--wrap` 折行、`--align` 对齐） |
| `replace` | 换音轨（`--mix`、`--duck`、`--fade`、`--loop` 短音源循环、`--at`/`--dur` 局部替换，逗号 `--at` 多段续铺）；也可 `--video` 换画面保音频 |
| `jumpcut` | 剪掉口播里的静音停顿 |
| `rough` | 长素材粗剪：先列出说话段落（`--json`），`-o` 再拼起来（`--merge N` 合并间隔小于 N 秒的段；`--by-scene` 场景切换处再切开；默认只编码要留下的段；`--copy` 无损但按关键帧） |
| `cover` | 封面静帧（`--at`（`end` = 最后一帧，逗号 = 每点一张）、`--blur` 模糊底填充、`--size` 画布——默认 1080x1920） |
| `fade` | 画面和声音淡入淡出（`--in` / `--out`，`--color` 淡出到白等、`--dip T` 场景闪黑转场，逗号列表多处闪黑；`--curve` 音频淡出曲线） |
| `title` | 标题卡烧录（`--text` 或 `--file notes.txt`，`--at`——`end` = 片尾卡，逗号列表可多次闪现、`--fade`、`--outline`、`--box` 底板、`--wrap` 折行、`--align` 对齐、`--opacity` 半透明、`--margin` 角位像素边距） |
| `loop` | 把成片重复 N 遍（Shorts 循环加长）（`--from`/`--to` 只循环片段，支持 `end`，`--fade` 无缝衔接） |
| `stabilize` | 手持防抖（deshake）——`--rx`/`--ry` 搜索半径，`--edge` 边缘填充 blank|original|clamped|mirror | `--engine vidstab` 双程 vid.stab（真抖动更稳），`--smoothing` 平滑窗口帧数 |
| `reverse` | 倒放画面和声音 |
| `grade` | `--preset` 一键风格 + 对比/饱和/亮度/`--gamma`/`--hue`/`--exposure`（EV 档）, `--at`/`--dur`/`--warm`/`--skin`；`--lut look.cube` 套 3D LUT，`--lut look.png` 套 HALD 图像 LUT（haldclut，Darktable/RawTherapee 导出）；`--skin -1..1` 只暖肤色（selectivecolor 红通道），`--vibrance -1..1` 智能饱和（提亮灰暗色、保护肤色） ，含 `--kelvin` 开尔文白平衡、`--split` 青橙分调 | 预设 `cinematic`/`vivid`/`vintage`/`soft`/`sepia`/`teal`/`noir`/`bleach`/`neon` 叠在滑杆之下  `--curve "x/y …"` 自由主曲线（哑光/S曲线） | `--wash C` 色彩轻纱 | `--match ref.mp4` 直方图匹配 | `--lut` 支持 1D LUT | `--color-from ref` 借用色度 | `--mix "rr,rg,rb,…"` 3x3 通道矩阵混色（colorchannelmixer） |
| `zoom` | 推近（`--factor 1.25`、`--center X,Y` 靶点；`--at`/`--dur` 局部窗口——逗号列表可多段，支持 `end`） | `--out`
| `sharpen` | USM 锐化，整段或定时窗口（`--amount`、`--at`、`--dur`） `--engine unsharp\|cas\|halo`（halo=maskedclamp 无过冲） |
| `vignette` | 暗角，整段或定时窗口（`--angle`、`--at`、`--dur`） |
| `bw` | 黑白化，整段或定时窗口（`--at`、`--dur`、`--strength` 保留部分色彩） ，`--weights r,g,b` 胶片通道权重  `--cut 0-1` 硬阈值（复印风） |
| `volume` | 音量 ±dB；`--at/--dur` 局部增益，逗号列表可作用多处（需 `--dur`）（平台响度请用 `loudnorm`） |
| `blur` | 全帧或定时高斯模糊（`--sigma`、`--at`/`--dur`；`--at end` = 片尾） | `--engine gblur|directional|box|avg`（directional `--angle` 速度线；box = 更快的方块核） |
| `trail` | 运动拖影：`--mode echo` 跟随残影（`--frames` 2-16、`--at`/`--dur` 窗口），`--mode light` 亮部拖尾（`--decay` 0.5-0.99） | `--mode diff` 运动残影 |
| `glitch` | 故障风 RGB 错位：`--strength` 0.5-20 控制通道偏移+噪点强度 | `--engine planes` 通道轮换 | `--engine swapuv` 色度翻转 | `--engine stutter` 抽帧抖动 | `--engine pixels` 像素块打散 | `--engine swaprect` 象限互换 | `--engine random` 帧序乱打 |
| `bars` | SMPTE 测试卡：`--size`/`--dur`/`--hd`/`--tone`（1kHz 音床），用于质检片头 | `--kind sd|pal100|pal75|rgb|yuv|allrgb|allyuv|mptest|testsrc` 其他广播测试图（`allrgb`/`allyuv` = 全色域立方体检质图，`mptest` = 编码拷机，`testsrc` = 一体化动态校准卡） |
| `scope --mode hist` | 亮度时间直方图——检查长时间曝光/色彩漂移 |
| `scope` | QC 示波器叠加：`--mode vector|wave` 角落小窗（`--position`、`--size` 占比）、`--at` 窗口 | `--mode mvs` 运动矢量 | `--mode data` 十六进制读数 | `--mode qp` 宏块量化叠加 | `--mode pix` 像素放大网格 | `--mode osc` XY 视频示波器 | `--mode drift` 亮度漂移曲线（曝光爬坡质检） | `--mode loud` 响度-时间曲线（ebur128+adrawgraph） | `--mode cie` CIE-1931 色域马蹄图（709 三角外=超色域） | `--mode graph` 实时滤镜图统计卡（graphmonitor——帧进出+队列，编码管线调试） | `--mode safe` 全帧广播安全框参考线（90% 动作安全黄框 / 80% 字幕安全红框 + 中心十字——构图质检） |
| `desqueeze` | 变形宽银幕还原：`--factor` 镜头倍率（1.33/1.5/1.8/2.0）、`--axis y|x` |
| `solarize` | 迷幻局部反色：高于 `--threshold` 亮度的像素反色，`--at` 窗口 |
| `pulse` | 呼吸变焦：`--rate` 每秒周期、`--depth` 幅度、`--at` 窗口 |
| `deflicker` | 延时摄影去闪：`--size` 帧时域亮度平滑 | `--engine tmide` 时间域均衡 |
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
| `stack` | 3+ 个同机位视频的中值堆叠——移除只在部分输入出现的东西（游客、传感器噪点）；`--percentile`；音频取自第 1 个输入 | `--mode median|max|min|mean`（max=星轨/光绘，min=最暗合成，mean=曝光平均合成——`--weights` 逗号加权自动归一化：`3,1` = 75%/25%）|
| `stereo` | 立体 3D 封装格式互转（SBS↔红蓝↔隔行） |
| `sonify` | 把图片/视频「奏」成声音 — spectrumsynth 按频谱图扫描（亮像素=强泛音，画出来的峰就是音符）；`--dur`/`--speed`/`--sample-rate` |
| `tmedian` | 时间中值——抹去窗口内出现不足一半时长的东西：三脚架画面中的行人/车辆、雨丝（`--radius` 历史帧、`--percentile`、`--at` 窗口；输出首尾各丢 radius 帧） |
| `dedup` | `mpdecimate` 丢弃近似重复帧——压缩静态片段，`--frac` 灵敏度 |
| `repair` | 用参考素材的干净帧替换坏帧/闪帧（`--ref` 参考片，`--at`/`--dur` 坏段，`--ref-at` 取用帧——freezeframes） |
| `audiogram --mode cqt` | 恒 Q 音乐频谱（`showcqt`）——钢琴卷帘式频谱，适合音乐片段 |
| `audiogram --mode spectro` | 滚动频谱图（`showspectrum`）——彩色时频滚动 |
| `scan` | 质检报告：黑场/冻结帧/黑帧计数 + 频闪 `flash_frames`/`flash_max_badness` + 隔行判定 `interlaced`/`frames_tff`/`frames_bff`/`frames_progressive` + 立体声 `phase_corr`（≈-1 表示单声道抵消）（idet；JSON extras；不写媒体）  `audio_max_db`/`audio_mean_db` 峰值/平均电平 | +模糊质检 | `--scenes` 剪切点时间戳 | `luma_min/max` + `illegal_luma`（signalstats 广播范围质检） | `noise_floor`/`noisy` 位平面噪底（码率预算质检） | `has_cc`/`cc_lines` EIA-608 隐藏字幕 | `crop_hint`/`letterboxed` cropdetect 黑边质检 | `vfr`/`vfr_ratio`/`vfr_frames` 可变帧率质检 | `--dupe REF` MPEG-7 签名重复/搬运检测 | `--text` OCR 烧录文字识别（`text`/`text_frames`/`text_confidence`） | `rg_gain_db`/`rg_peak` ReplayGain 标签 | `--motion` VMAF 运动分（`motion_avg`/`motion_max`，码率预算质检） | `--timecode` VITC 时间码读取（`vitc`/`vitc_tc`/`vitc_frames` 广播母带质检） | `--bbox` 内容包围盒（`content_detected`/`content_box`/`content_fill` — 不限黑底的纯色背景也可用） | `sat_mean`/`hue_mean`/`y_mean` 色调质检（发灰/偏色/节目亮度，同趟解析） | `--deadair DB` 死空气段质检（`deadair_secs`/`deadair_ranges`，播客/口播静默图） | `--loud` EBU R128 响度质检（`loud_i`/`loud_lra`/`loud_tp`，平台响度闸口；纯音频文件也可用）、`hdr`/`wide_gamut`/`color_space`/`color_primaries`/`color_transfer` 容器色彩标签直读 HDR/广色域质检 | `--gop` 关键帧间隔质检（`keyframes`/`gop_max_sec`/`gop_avg_sec`/`gop_max_frames`，只读 packet 标记不解码，「关键帧 ≤2s」平台准入规格） | `--hash` 逐帧校验码写 `<input>.framemd5`（归档入库完整性清单） | `timecode` 容器时码质检（mov tmcd / mkv TIMECODE，母带对板核验，不解码） | `--bitrate` packet 图码率曲线（`bitrate_mean_mbps`/`bitrate_peak_mbps`/`bitrate_spike_at`，峰值码率准入规格质检，不解码） | `av_desync_ms` 音/视频 start_time 偏移（容器级唇音同步质检，不解码；配 `remux --audio-delay`/`--video-delay` 即修即验） |
| `smooth` | 边缘保留美颜/皮肤模糊（`--engine` smartblur/bilateral——bilateral 边缘更锐利；`--strength`、`--at/--dur` 窗口） ，含 `deflate`/`inflate` 形态学平滑 | `--engine uspp` 后处理去块 | `--engine pp7` 轻量后处理 | `--engine yaep` 边缘保留 | `--engine spp/fspp` 轻量去块 | `--engine sab` 形状自适应平滑 |
| `upscale` | 老素材升分辨率：`zscale` spline36（优于 lanczos）+ 轻度锐化，`--factor` 1.05-4（2 = 宽高翻倍），`--strength` 边缘锐度 | `--engine spline|xbr|two-xsai`（像素画整数倍缩放） | `--engine hqx` hq2x/3x/4x 像素画放大 | `--engine epx` EPX 2x/3x 像素缩放 |
| `v360` | 360° 画面重取景为平面（`--in` 支持 equirect/fisheye/dfisheye/cubemap/EAC/barrel/半等距，`--yaw`/`--pitch`/`--fov`、`--size`） |
| `perspective` | 斜拍屏幕/白板矫正：`--points x0,y0,x1,y1,x2,y2,x3,y3`（源画面四角 TL,TR,BL,BR，像素），`--interp linear|cubic` |
| `wb` | 自动白平衡/去色偏：`--strength` 0..1，`--independence 0` 只调对比保色调，`--smooth` 时间平滑帧，`--at`/`--dur` 局部 | `--engine greyedge` 灰边光源估计（调色素材更温和） |
| `shear` | 画面倾斜（斜体字效果）：`--x`/`--y` 剪切系数 -2..2，`--fill` 边缘填充色，`--interp nearest|bilinear`，`--at`/`--dur` 局部 |
| `gen` | 生成式动态背景（无需输入文件，lavfi 源）：`--pattern mandelbrot`（无限缩放分形）`|gradients`（渐变漂移，`--colors`/`--seed`/`--speed`）`|life`（细胞自动机，`--rule`）`|sierpinski`（分形），`--size`/`--fps`/`--dur` | `--pattern noise|tone` 音频底噪（`--color` white/pink/brown/blue/violet/velvet 噪色可选） | `--pattern sweep` 20Hz→`--freq` 扬声器测试扫频 | `--pattern silence` 数字静音床（anullsrc 补静音段） | `--pattern hald` 恒等 HALD LUT 图（`--level` 默认 8——在图像编辑器里调色后喂给 `grade --lut`） |
| `sharpen --engine cas` | 对比自适应锐化——边缘更脆且无 unsharp 光晕，`--amount` |
| `equalize` | `histeq` 自动对比度，修复灰暗/洗白画面，`--strength`/`--intensity`/`--at` 窗口 |
| `pick` | 主色提取：某时刻均色 + 3x2 分区色板（仅 JSON 报告） |
| `diff` | 双片视觉差异：放大差值混合，`--side` 并排参考 | `--mode mask --threshold` 纯变化掩码质检 |
| `selective` | 单色保留：仅留 `--color` 其余去饱和，`--similarity` 容差，`--blend` 边缘羽化，`--engine chroma` chromahold 色度空间保留（饱和色更准），`--at` 窗口 |
| `amplify` | 运动放大——细微帧间变化变可见（`--amount` 倍数、`--radius` 历史帧、`--threshold` 差值上限、`--at` 窗口） |
| `cartoon` | 漫画效果：色块化（`--levels` 2-16）+ 边缘墨线，`--at` 窗口 |
| `heat` | 热成像伪彩：`--preset` magma|inferno|plasma|viridis|turbo|cividis|range1|range2|shadows|highlights、`--opacity`、`--at` 窗口 |
| `kaleido` | 左上象限镜像成 2x2 曼陀罗，`--at` 窗口 |
| `strobe` | MV 频闪剪切：`--rate` 每秒闪数、`--duty` 占空比、`--color`、`--at` 窗口 |
| `edge` | 霓虹描边：`--mode wires|colormix`、`--low`/`--high` 阈值、`--at` 窗口 | `--engine edgedetect|sobel|kirsch|roberts|prewitt` | `--engine link` 迟滞连接边缘 |
| `lens` | 镜头畸变：`--k1`/`--k2` —— 负值鱼眼效果，正值运动相机去鱼眼；`--at` 窗口 |
| `mirror` | 半画面中心镜像（`--axis x`/`y`、`--at`/`--dur` 窗口）——舞蹈/对称效果 |
| `pix` | 复古马赛克像素化：`--strength` 2-64 分块因子（`--at`/`--dur` 窗口） |
| `flip` | 水平/垂直翻转（`--axis x` 自拍去镜像、`--at`/`--dur` 窗口） |
| `poster` | 波普海报化：`--levels` 2-64 调色板色数（`--at`/`--dur` 窗口） |
| `duotone` | 双色调映射：`--shadow`/`--highlight` 亮度渐变（`--at`/`--dur` 窗口） |
| `glow` | 梦幻泛光：模糊副本 screen 混合叠加（`--strength`、`--at`/`--dur` 窗口） |
| `vhs` | 复古磁带：`--strength` 0-3 噪点+色偏+扫描线（`--at`/`--dur` 窗口） |
| `motionblur` | 快门拖影：`--frames` 2-8 帧间混合（`--at`/`--dur` 窗口） |
| `vdenoise` | 视频降噪（`--engine nlmeans，新增 `edge`（边缘保护 nlmeans）|hqdn3d|atadenoise|vaguedenoise|bm3d|dctdnoiz|owdenoise|median|chroma`、`--strength`、`--at` 窗口），支持 `end`，逗号列表可多段 | `--engine dotcrawl` 模拟信号净化 | `--engine fftdnoiz` FFT 胶片噪点 | `--engine rg` removegrain 快速逐平面去颗粒 |
| `crop` | 裁剪 `--region` 区域，或 `--aspect` 重构（`--anchor center|top|bottom|left|right` 可选锚点） |
| `waveform` | 音频波形 → PNG（`--size`、`--color`, `--scale`、`--peak` 峰值、`--split` 逐声道、`--full` 密集、`--bg` 不透明底卡、`--vertical` 竖向波形（自上而下，PNG 为 高×宽）），播客封面/缩略图用（`--at/--dur` 只画片段，支持 `end`，逗号 `--at` 每窗一张 `<stem>_N.png`） |
| `spectrogram` | 音频频谱图 → PNG（`--size`），清理前先看嗡鸣/噪声（`--color` magma/viridis…、`--scale` lin/sqrt…、`--no-legend` 去图例、`--separate` 逐声道分带）（`--at/--dur` 只画片段，支持 `end`，逗号 `--at` 每窗一张 `<stem>_N.png`） |
| `meter` | EBU R128 实时响度表视频（`--size`、`--meter 9\|18`、`--at/--dur` 只测片段）——边听边看 I/TP/LRA |
| `dehum` | 市电嗡鸣陷波（`--mains 50|60` 或 `--freq HZ` 自定义频率、`--harmonics`、`--at/--dur`，支持 `end`，逗号列表可多段） |
| `tempo` | 音频变速 `--factor` 0.5–8，不变调（`atempo` 链；视频请用 `speed`） ，`--at/--dur` 局部变速——逗号列表可多段，支持 `end` |
| `leveler --engine mcompand` | 多段压缩预设——低频/人声/空气感分带，抬弱声压峰头 |
| `leveler` | 动态压平（`--preset`、`--engine speechnorm` 自适应人声归一、`--engine limit` alimiter 砖墙上限、`--engine compand` 单段传递曲线（比 acompressor 硬拐点更柔的向上电平）、`--at/--dur` 窗口），支持 `end`，逗号列表可多段 |
| `gate` | 噪声门——低于 `--threshold` dB 的部分静音（`agate`）（`--preset voice|podcast|studio`，`--at/--dur` 局部生效），支持 `end`，逗号列表可多段 |
| `silence` | 在 `--at`（逗号列表多处）或 `--end` 插入 `--dur` 秒静音；`--detect` 以 JSON 报告静音区间 |
| `vocal` | 消/留中置人声（`--mode`、`--amount` 强度、`--at/--dur` 窗口），支持 `end`，逗号列表可多段 |
| `remux` | 换容器不重编码（`-c copy` + faststart）；`--audio` 只提音轨，`--video` 只留视频，`--aspect 16:9` 修显示宽高比，`--frag` 分片 MP4（moof/mfra——还在写入就能播，HLS/DASH 管线用），`--no-subs` 剥掉字幕/数据流（干净交付件），`--itsscale R` 不重编码整容器变速（时间戳 ×R——1.042 PAL 25→24 下拉、0.96 电影→PAL 加速，音频随画面变），`--offset SEC` 设定容器 start_time（修采集文件负值/异常起点），`--from SEC`/`--to SEC` 无损裁剪（关键帧精度——不重编码只打包一段），`--lang jpn` 只保留标记该语言的音轨（多语言发行、配音提取；配 `--audio` 即只抽该语言轨），`--default-audio N` 把第 N 条音轨设为播放器默认轨，`--cover pic` 把图片作为 attached_pic 流挂进成片（纯音频得封面、视频得缩略海报），`--chapters marks.txt` 把 YouTube 格式章节列表嵌成容器章节（就是 `chapter --yt` 导出的那个文件——Apple Podcasts/Books 直接变跳转点），`--title`/`--artist`/`--album`/`--genre`/`--comment`/`--date` 重打包时顺手写音乐库标签，`--lang eng,jpn` 逗号列表保留多条配音轨，`--strip-meta` 抹掉继承来的全部容器标签（隐私清理——配合标签参数可擦完顺手重写）、`--no-cover` 剥掉 attached_pic 封面流（精简有声书/m4a）、`--encrypt` 重打包时 CENC AES-CTR 加密（仅 mp4/mov——ClearKey/Widevine DRM 预处理，`--key`/`--kid` 32 位十六进制可填、不传则随机，JSON 会回显）、`--audio-delay SEC` 音轨整体平移对视频——不重编码修唇音同步（负值为音频提前），`--video-delay SEC` 另一半修法：采集卡画面滞后的救星（正值延后画面、负值提前；与 --audio-delay 互斥），`--tag hvc1` 重写 codec tag 让 HEVC mp4 能在 QuickTime/Safari 播放（仅 mp4/mov），`--attach f` 把二进制附件流嵌进 mkv/webm（字幕字体随文件走；可重复），`--audio-order 1,0` 重排+保留音轨（按轨索引，没列的轨丢弃——只放第 0 轨的老播放器要节目混音轨在最前）） |
| `meme` | 上下说明文字梗图（`--outline`、`--at/--dur` 时间窗，逗号列表可打多处；`--at end` 片尾） ，`--position` 文字块上/中/下；`--wrap` 折行、`--align` 行对齐、`--fade` 窗口边缘淡入淡出（配 --at/--dur）、`--opacity` 半透明文字 |
| `voice` | 播客人声一条龙：`agate` 去嘶声 → `acompressor` 压平 → `loudnorm` 响度（`--threshold`、`--lufs`、`--at`/`--dur` 只处理一段，支持 `end`，逗号列表可多段） |
| `deinterlace` | 修复隔行素材（`--mode`、`--parity` 场序、`--engine` yadif/bwdif/estdif/kerndeint） ，`--engine` 含 `detelecine`（确定节奏反电视电影）、`mcdeint`（运动补偿）| `--engine w3fdif` 三场去隔行 | `--engine separate` 场拆帧 50i→50p（顺滑慢动作源） | `--engine pullup` 反电视电影 IVTC | `--engine phase` 场序调换（场序标错的采集） | `--engine field` 单场提取（半高，最快预览） |
| `dedust` | 去尘埃斑点/坏点：`--size` 1-4，默认去亮点，`--dark` 去暗点；形态学腐蚀/膨胀，不是模糊；`--engine temporal` tlut2 时域去除单帧白点/VHS 断线 | `--at`/`--dur` |
| `extend` | 边缘像素拉伸填充边条：`--left/--right/--top/--bottom` px，`--mode smear|mirror|fixed|reflect|wrap|fade` | - |
| `tonemap` | HDR → SDR：zscale 转线性光 → 色调映射曲线 → bt709（`--algo hable|reinhard|gamma|clip|linear`，`--peak` nits） | - |
| `telecine` | 24p 胶片转 NTSC 隔行场（`--pattern 23` 3:2 下拉，`--field tff|bff`）— fieldmatch 的逆操作 | - |
| `premult` | 直通 α ↔ 预乘 α 就地转换（`--mode premultiply|unpremultiply`）；输出保留 α 的 prores4444 | - |
| `dejudder` | 消除电视电影抖动（`--cycle 4` 对应 3:2 下拉） | - |
| `despill` | 去除抠像边缘绿/蓝溢色（`--type`、`--mix`、`--expand`、`--at`/`--dur`） | - |
| `interp` | 运动补偿插帧：`--fps 60` 上采样、`--slow 0.5` 顺滑慢动作 | - | `--engine minterpolate|framerate` |
| `matrix` | 色彩矩阵转换（`--from bt601 --to bt709` 修 SD 偏绿；源矩阵自动检测；`--engine colorspace` 还换算原色+传递曲线，bt2020↔709 走这条） | - |
| `legalize` | 亮度钳制到广播安全 16-235（`--min`/`--max`、`--at`/`--dur`）；`--flash` 压制光敏性癫痫频闪（scan 可检出 flash_frames） | - |
| `levels` | Photoshop 色阶：`--in-min/--in-max/--out-min/--out-max`（救压暗素材、哑光头） | - |
| `aberrate` | 色散镶边 — `--amount` px（VHS/故障边缘感） | - |
| `displace` | 按第二个素材亮度位移扭曲画面（热扭曲/液体故障）：`--edge` wrap/mirror/smear/blank，`--at`/`--dur` 窗口 | - |
| `eqviz` | 应用 EQ 频段并渲染响应曲线为视频：`--bands "f=200 w=100 g=10 t=h"`（t=h/l/p 搁架/峰值），`--size` | - |
| `crossfade` | 两段音频淡接，`--dur` 秒重叠（`acrossfade`） |
| `strip` | 去掉全部元数据/章节（发片前隐私清理），无损 `-c copy` |
| `frames` | 每 `--every`、`--at` 秒抽一帧（`end` = 最后一帧）、`--count` 均布 N 帧、`--nth N` 每隔 N 帧抽一帧（数据集/质检采样）→ `stem_001.png…`（`--width` 缩放）；`--untile CxR` 把每帧拆成瓦片静帧（还原宫格图/马赛克） |
| `countdown` | 画面倒数（`--from` 最多 600、`--beep` + `--tone` 蜂鸣频率、`--text`、`--position`、`--bg` 数字底板、`--format` mm:ss/h:mm:ss 长倒计时、`--opacity` 半透明、`--target HH:MM` 按本地挂钟倒数到首播/开播时间，每秒一数最多 10 分钟，时间已过自动顺延明天） |
| `invert` | 全帧或定时反色（`--at`、`--dur`——逗号列表可多段） |
| `mix` | 双音轨叠加（`--vol-a/--vol-b`、`--at/--dur`（逗号列表可多次进床）、`--loop`、`--duck` 人声闪避音乐、`--gate` 更狠的门限闪避——说话时底床直接静音） ，`--normalize` 归一求和、`--fade` 淡入淡出，支持 `end` |
| `mute` | 去掉音轨（其余流直接封装，不重编码） ，`--at/--dur` 局部静音（逗号列表可静多处，需 `--dur`），支持 `end` |
| `timer` | 画面计时器（`--position`、`--format ms`、`--box-color` 底板） （`--format`、`--box-color`、`--down` 倒计时、`--start` 设定起始读数、`--opacity` 半透明）——`--at` 支持 `end`；`--tc HH:MM:SS:FF` 烧录走带时码（样片/审片，`;` 前 FF 位表示丢帧意图，仅显示）；`--clock` 烧录本地挂钟 HH:MM:SS（赛事/活动实时钟），`--date` 烧录本地日期 YYYY-MM-DD（归档/播出日期戳——静态读数；和 --clock 同用时排在时间前） |
| `hls` | 网页 HLS 封装（`--seg`、`--single`、`--copy`、`--ladder` 多码率、`--audio-only` 纯音频、`--fmp4` CMAF、`--poster` 同时输出 poster.jpg 封面，`--poster-at T` 选封面帧，`--encrypt` AES-128 加密分片并写 key.bin/key.info（`--key HEX` 自定义密钥、`--key-uri URI` 播放列表里的密钥地址）→ 私有/付费流，`--live` 滑动窗直播播放列表——播放器可中途加入，只保留最新 `--live-window N` 个分片、不写 endlist；`--start N` 重启后从 N 续编号、`--epoch` 用 epoch 时钟播种分片序号（24/7 频道不停表）、`--date` 给分片写 EXT-X-PROGRAM-DATE-TIME 时间戳、`--discontinuity` 标记推流重启点、`--time-names` 分片按墙钟时间命名（归档录像文件名即时间）、`--independent` 打 EXT-X-INDEPENDENT-SEGMENTS 并强制每分片首帧关键帧（拖动/变速播放 VOD） |
| `dash` | DASH 封装 → manifest.mpd + init-/seg-*.m4s（`--seg`、`--copy` 不重编码直接切片、`--single` 每表示层单文件按字节请求、`--webm` vp9+opus 分片、`--window N` 滑动窗直播清单、`--ladder 1080,720,480` ABR 自适应——单一清单里 N 档视频表示层按梯度码率供播放器随带宽切换） |
| `live` | 把片段推向直播采集端：`--to rtmp://…`/`rtmps://`/`tcp://`/`udp://`/`srt://`（SRT/UDP 走 MPEG-TS 封装）（`-re` 实时节奏推流，x264/aac 采集编码），`--codec hevc` 贡献级 HEVC（仅 MPEG-TS 传输），`--subs file.srt` 直播同步烧字幕，`--loop` 无限循环（24/7 音乐台/首播轮播），`--vbitrate`/`--abitrate` 可调码率，`--scale WxH` 大母片降采样到采集规格，`--fps N` 压输出帧率，`--record file.mp4` 推流同时本地存档（tee——一次编码两路封装），`--until SEC` 定时停播，`--list` 清单轮播（concat 清单文件，24/7 轮播台；`--loop` = 无限轮播），`--test` 内置测试卡+1kHz 音（开播前验证推流密钥），`--slate card.png --slate-dur SEC` 开播前先推「即将开始」定场图（首播/定时开场），`--card art.png` 给纯音频源配一张常驻静帧当画面（24/7 lofi 电台流），`--overlay bug.png` 台标钉在角落（`--overlay-position tl|tr|bl|br`、`--overlay-opacity` 半透明），`--restream url` 同时推第二个采集端——一次编码多平台直播，`--gop N` 关键帧间隔对齐采集规格（YouTube 要求 ≤2s），`--preset` x264 速度/画质档位，`--vertical` 信箱到 1080x1920 竖屏直播画布、`--maxrate 4500k`/`--bufsize` CBR 限幅对齐采集端码率规格（Twitch ≤6000k，bufsize 默认取 maxrate 两倍）、`--start T` 从素材第 T 秒开始推流——长录像剪掉头直接从中段重播，`--title "节目名"` 把节目名写进 FLV/TS 流元数据与 --record 存档（采集端面板显示）) |

| `qa` | 对比参考视频测画质损失（PSNR + SSIM + MSAD + VIF，`--metric`） |
| `conform` | 一键统一规格（`--size WxH`、`--fps 30`、`--lufs -14`、`--crf`、`--pad` 黑边颜色 + `--anchor` 锚点、`--blur` 模糊填充，`--hold SEC` 克隆末帧片尾停留 + `--hold-start` 片头预停（音频自动补静音）、`--even` 奇数像素向下取偶（手机/录屏奇数尺寸吃不下 yuv420p 的一键修）；`--ar HZ` 音频重采样率（默认广播 48k；44100 播客/CD、96000 母带）；`--channels 1` 单声道母带 |
| `sync` | 修复音画同步（`--ms ±N` 垫音/裁音头） |
| `align` | 音频互相关自动对齐第二路录音（多机位/外接录音笔，`--max-lag`） | `--check` 只报偏移不渲染：`offset_ms`/`direction`（对轨质检） |
| `scroll` | 片尾滚动字幕（`--text`/`--file`、`--at`、`--dur` 或 `--speed` px/s、`--size`、`--color`、`--font`、`--align` 对齐、`--wrap` 折行）；`--mode ticker` 底部新闻条可加 `--bg` 不透明底条；`--at` 逗号列表可多次复播，`end` 亦可；`--opacity` 半透明字幕 |
| `insert` | 在视频中段插入整段素材（`--at`，逗号列表多点插入，`end` 追加到片尾；`--dur` 只取前 N 秒；`--transition` 转场 + `--duration` 两端淡入淡出；`--replace` 覆盖插入点下方原素材——补录口误，成片保持原时长；`--at chapterN` 按内嵌章节起点插入（编号即 `chapter --list` 所见——章节化文件改一节不返工）) |
| `multicam` | 双机位对齐后角度切换：`--at t1,t2,...` 逐点换机位（`end` = 片尾切回）；`--align` 先用音频互相关自动把 B 机位对齐到 A（省掉单独跑 align——需要宽带同步音频如人声/环境声，纯正弦不相关）；`--keep-audio` 全程用 A 机位音轨、`--transition` 软切换 |
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

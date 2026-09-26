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
| `probe` | 时长、分辨率、编码、声道、`av_desync_ms` 唇音同步偏移、`timecode` 容器时码、`has_alpha` α 通道、`has_data`/`has_attachment` 容器流类标记（data=遥测/定时元数据待剥离对象、attachment=字幕字体/内嵌载荷——`remux --no-attachments`/`extract --attachment` 的前置闸口）、`tags` 元数据审计、`bit_rate` 容器总码率、`streams` 逐轨清单（index/kind/codec/codec_tag 编码标签（验 `remux --tag` 是否落地）/stream_id 容器流 id（mpegts PID 质检）/is_avc+nal_length_size（avcc 与 annex-b 质检——HLS/fMP4 封装前必查）/chroma_location（4:2:0 色度采样位置——广播规格质检）/bits_per_raw_sample（源位深——10/12bit 母带质检）/语种/title 轨名/sample_rate 采样率/profile 编码档/fps 平均帧率/r_fps 真实基准帧率——与 fps 不等即逐轨 VFR/duration 轨时长/bit_rate 码率/pix_fmt 像素格式/alpha 逐轨 α 像素格式（yuva*/rgba 系——标明哪一轨带透明，转码丢通道前必查）/color_space 色彩空间/channel_layout 声道布局/sar 采样宽高比/dar 显示宽高比/field_order 场序（隔行母带检出——去隔行/广播交付规格）/color_primaries 色度基色/color_transfer 传递函数（HDR 规格对：bt2020+PQ/HLG）/color_range 色域范围（tv 限幅/pc 全幅 JPEG 范围）/sample_fmt 采样格式（s16/fltp——不止 codec 名的真实编码）/start_time 逐轨起始时间（某轨起晚=烧进去的同步偏移）/nb_frames 逐轨总帧数（帧预算质检）/level 编码等级（High@L4.0 设备规格）/attached_pic 封面附件轨/coded_width/coded_height（存储 vs 显示尺寸——宏块填充检出）/default 默认轨/forced 强制字幕轨/hearing_impaired SDH 听障轨/comment 评论轨/visual_impaired 口述影像轨/dub 配音轨/original 原声轨）、`rotation` 显示矩阵旋转质检、`start_time` 最早流起始时间（采集文件负值/异常起点——`remux --offset` 修）、`chapter_count` 容器内嵌章节数（交付件目录质检——`chapter --list` 看具体标记）、`program_count` 复用节目数（多业务 mpegts/spts 质检——重打包前先选流）、`probe_score` 容器检测置信度 0-100（<100 即误检或容器受损——坏重打包前拦住）、`programs[]` 复用业务清单（编号/业务名+运营方/成员流——`remux --program N` 前查这个）、`streams[].has_b_frames` B 帧数（0=baseline/实时安全编码——低延迟规格质检）、`streams[].time_base` 逐轨复用时基（mpegts 1/90000 vs mp4 1/15360——包 pts 数学质检）、`chapters[]` 内嵌章节标记清单（{start,end,title}——不用 `extract --chapter` 就能核对落点）、`encrypted` CENC/DRM 容器检测（ffprobe 根本不报加密——按 sinf/encv/enca/schi 原子头尾字节扫描）、`has_subs` 容器内有字幕流（extract --subs / remux --no-subs / deliver --subs 的前置闸口）|
| `look` | 联系表（`--tiles`）或指定时间点（`--at`，可重复） |
| `cut` | 剪切；默认无损 copy，`--accurate`、`--ranges`、`--drop` 才帧精确（边界支持 `end`：`T-end` 到结尾、`end-N` 最后 N 秒）；`--fade N` 切口淡入淡出；`--black` 自动切掉 blackdetect ≥0.3s 黑段（死画面剔除，画面版的 cutsil） |
| `concat` | 拼接 N 段（全部 42 种 4.4 命名 xfade `--transition`，逗号列表逐接缝选转场）；`--copy` 强制无损流拷贝拼接，规格不一致直接失败（保证成片没重编码的质检——规格不符时自动路径会静默重编码）；`--level -14` 先统一各段响度；`--gap N` 段间插入黑场+静音 ；`--audio-fade N` 接缝处音频淡化（边界淡化，时长/同步不变）；`--repeat N` 把整条拼接序列循环 N 次（循环向片头+循环段+片尾结构——转场落在每个循环接缝上，单文件也直接循环）；`--list manifest.txt` 从清单文件读取片段路径（相对路径按清单所在目录解析） |
| `fit` | 画幅 / 旋转 / 翻转（9:16、1:1、16:9…）；`--fit blur` 用模糊背景填满 ，`--position` 画面对齐黑边位置 ，`--strength` 模糊力度 |
| `extract` | 抓静帧或 `--gif` 动图（`--bounce` 往返循环） | `--at`（`end` = 最后一帧 / 最后 --dur 秒，逗号 = 每点一张静帧——`--gif` 时每点一条动图）、`--width`、`--fps` ，`--loop` GIF 循环次数、`--colors` 调色板大小、`--alpha` 提取 α 通道为灰度 PNG（仅限带透明通道输入）、`--audio` 无损抽音轨（流拷贝——`-o` 扩展名选容器，拉音乐/对白轨给剪辑用；`--track N` 选第 N 轨，`--all` 每轨各出一份 `stem_aN.<ext>`）、`--lang jpn` 按语言标签在 `--audio`/`--subs` 上挑单轨（逗号拒收——全轨用 `--all`）、`--from SEC`/`--to SEC` 只抽音轨的一段（剪音乐 cue 仍无损）、`--attachment N` 把附件轨 N 原样抠回文件（`remux --attach` 嵌进去的字幕字体/文件——mkv/webm）、`--cover` 把内嵌封面抠回图片文件（`remux --cover` 的反向操作——核验封装内封面或抠出来改版用；扩展名要跟内嵌编码一致：mjpeg→.jpg、png→.png）、`--subs` 抽内嵌字幕轨为文本文件（`-o` .srt/.ass/.vtt 选字幕容器；`--track N` 选语种，`--all` 每轨各出一份 `stem_sN.<ext>`——成片字幕抽出改时轴/重烧录）、`--chapter N` 按内嵌章节抽段成单独文件（1 基编号即 `chapter --list` 所见——有声书/讲座分段导出）、`--keyframes` 把每个关键帧（I 帧）各导出一张图（裸 `-o` 自动补 `stem_%03d`——GOP 边界静帧：关键帧间隔质检/延时摄影源/场景速览）、`--gif --transparent` GIF 保留透明通道（Discord/Telegram 贴纸——需 prores 4444/qtrle 等含 α 输入）、`--webp` 动画 WebP 片段（比 GIF 小且原生保 α；`--lossless`、`--bounce` 可用） |
| `overlay` | logo/画中画；`--tile N` 全屏草稿水印 | logo、水印、画中画 | `--angle`、`--loop` 短视频循环、`--border` 画中画描边、`--mode` 混合合成（screen/multiply/softlight/dodge/burn/exclusion/hardmix/negation/grainmerge/multiply128/and/xor/freeze/heat… 31 种 Photoshop 式混合 + 颗粒合成 + 逻辑通道） |
| `broll` | 切入镜头（`--insert` 视频、`--still` 图片、`--motion kenburns` 推镜） | 切走 B-roll（`--insert --at --duration`）；口播声音和时长不变 ，`--audio` 听插播原声（`--volume` 音量） ，`--position` 画中画角位 + `--scale`，`--border` 描边、`--opacity` 半透明插播；`--at end` 片尾切入，逗号 `--at` 多处重复切入 |
| `caption` | 烧录字幕（`--srt`、`--chunk`、`--karaoke`、`--box-color` 底板、`--wrap` 折行、`--from/--to` 只烧窗口内字幕（支持 `end`/`end-N`）） ，`--fade` 淡入淡出、`--opacity` 半透明水印字幕 |
| `broll` | 切入镜头（`--insert` 视频、`--still` 图片、`--motion kenburns` 推镜） | 切走 B-roll（`--insert --at --duration`）；口播声音和时长不变 ，`--audio` 听插播原声（`--volume` 音量） ，`--position` 画中画角位 + `--scale`，`--border` 描边；`--at end` 片尾切入，逗号 `--at` 多处重复切入 |
| `caption` | 烧录字幕（`--srt`、`--chunk`、`--karaoke`、`--box-color` 底板、`--wrap` 折行、`--from/--to` 只烧窗口内字幕（支持 `end`/`end-N`）） ，`--fade` 淡入淡出、`--opacity` 半透明水印字幕 |
| `broll` | 切入镜头（`--insert` 视频、`--still` 图片、`--motion kenburns` 推镜） | 切走 B-roll（`--insert --at --duration`）；口播声音和时长不变 ，`--audio` 听插播原声（`--volume` 音量） ，`--position` 画中画角位 + `--scale`，`--border` 描边；`--at end` 片尾切入，逗号 `--at` 多处重复切入 |
| `caption` | 烧录字幕（`--srt`、`--chunk`、`--karaoke`（`--highlight` 已唱字高亮色）、`--box-color` 底板、`--wrap` 折行、`--from/--to` 只烧窗口内字幕（支持 `end`/`end-N`）） ，`--fade` 淡入淡出、`--opacity` 半透明水印字幕 |
| `loudnorm` | EBU R128 两遍响度归一（`--target spotify|podcast|broadcast`）；`--measure` 只测不写（`--gate N` 超过 N LUFS 即失败）；`--dynamic` 逐帧动态增益 |
| `denoise` | 音频降噪（`--strength`、`--highpass`、`--engine auto|wavel|fftdn` 指定引擎、`--at/--dur` 窗口），支持 `end`，逗号列表可多段 | `--ref noise.wav` anlms 自适应消噪 |
| `transcode` | h264/webm/`--preset gif`（`--fps`/`--width`/`--copy-audio`/`--colors`） | 预设 `h264`/`webm`/`gif`/`hevc`/`mp3`/`aac`/`wav`/`flac`/`opus`/`av1`/`prores`/`dnxhd`/`proxy`/`ffv1`/`apng`/`mpeg2`/`mpeg1`/`xvid`/`wmv`/`msmpeg4`/`gpp`/`flv`/`theora`/`ogg`/`alac`/`dv`/`mjpeg`/`amv`/`qtrle`/`v210`/`huffyuv`/`utvideo`/`ffvhuff`/`cinepak`/`svq1`/`zmbv`/`v410`/`ayuv`/`rv10`/`rv20`/`r210`/`v308`/`rpza`/`speedhq`/`roq`/`snow`/`flashsv`/`flashsv2`/`msvideo1`/`cljr`/`ac3`/`eac3`/`tta`/`dca`/`raw`/`aiff`/`pcm24`/`pcm32f`/`mulaw`/`adx`/`adpcm`/`alaw`/`speex`/`pcm8`/`adpcmms`/`g722`/`ra144`/`nelly`/`wv`/`mp2`/`caf`/`w64`/`voc`/`aptx`/`sbc`/`g723`/`hap`/`hapq`/`cfhd`/`vc2`/`magicyuv`/`r10k`/`truehd`/`mlp`/`h261`/`h263`/`avui`/`ts`/`mxf`/`ivf`/`gxf`/`wtv`/`smjpeg`/`nut`/`framemd5`/`y4m`（`ffv1` = .mkv 无损存档母版——视频 ffv1 + 音频 flac；`apng` = 全彩动态 PNG 循环，gif 256 色发带的贴纸/反应动图；`nut` = FFV1+FLAC 装 ffmpeg 原生 .nut 无损交换容器——中间件/存档级；`framemd5` = 逐帧 MD5 校验清单（.framemd5/.md5/.txt——存档级解码保真质检：日后重新解码逐帧比对；文本清单不是媒体）；`y4m` = YUV4MPEG2 原始裸视频流装 .y4m（Avisynth/VapourSynth/x264-CLI 交换格式——只有视频））；`mpeg2` = MPEG-2+MP2 DVD/广播老格式母带（.mpg/.vob——机顶盒/电视台收录；`mpeg1` = MPEG-1+MP2 VCD 时代老格式母带（.mpg——仍在流通的最老数字视频格式）；`xvid` = Xvid/MPEG-4+MP3 老片源母带（.avi——DivX 时代播放器/投影机）；`wmv` = WMV2+WMA Windows Media 时代母带（.wmv/.asf——企业培训老档/旧 PPT 内嵌视频）；`msmpeg4` = MS-MPEG4 v2+MP3 装 .avi（MP42 标签——比 DivX 更早的 Windows 编码：WinME 时代录屏与上古播放器）；`gpp` = H.263+AMR-NB 装 .3gp（功能机时代母带——MMS 时代手机视频；h263 只有 5 种合法画幅自动就近取齐补边、音频强制 8kHz 单声道）；`flv` = FLV1+MP3 装 .flv（Flash 时代网页母带——2005 年 YouTube 上传档）；`theora` = Theora+Vorbis 装 .ogv/.ogg（开放网页母带——WebM 前的 HTML5 视频、维基媒体嵌入）；`ogg` = 纯音频 Ogg Vorbis（.ogg——开放网页音乐/播客上传）；`alac` = ALAC 装 .m4a（Apple Lossless 存档——苹果生态无损，音质与 wav 一致）；`dv` = DV25 装 .dv/.avi（DV 磁带母带——MiniDV/DVCAM 档案、广播收录带；自动取齐 NTSC 合法规格 720x480@30000/1001 yuv411p + PCM 48kHz 立体声，调优参数拒收）；`mjpeg` = Motion JPEG+MP3 装 .avi/.mov（NLE 时代剪辑格式——Digital Betacam 收录、逐帧定位母带，每帧都是内帧 JPEG）；`amv` = AMV 视频+ADPCM 装 .amv（国产 MP3/MP4 播放机母带——2006 年手持设备；死规格同 dv：自动取齐 160x120@25fps + ADPCM 22050Hz 单声道补边，全部调优参数前置拒收）；`qtrle` = QuickTime Animation RLE 装 .mov（无损动画/录屏母版——rgb24，`--alpha` 走 argb）；`v210` = 无压缩 10-bit 4:2:2 + PCM 装 .mov（广播/剪辑棚收录规格——码率参数无意义）；`huffyuv`/`utvideo`/`ffvhuff` = 无损采集中间件 + PCM（NLE 时代剪辑母带——比 ffv1 快；huffyuv/utvideo 装 .avi，ffvhuff 装 .mkv，huffyuv 也吃 .mkv）；`cinepak` = Cinepak+PCM 装 .mov/.avi（CD-ROM 时代编码——90 年代中期 QuickTime/Windows 视频，Myst 时代游戏档）；`svq1` = Sorenson Video 1+PCM 装 .mov（QuickTime 2-4 时代网页视频——Flash 前的互联网标准）；`zmbv` = Zip Motion Blocks Video+PCM 装 .avi（DOSBox 时代录屏档——游戏采集存档）；`v410` = 无压缩 10-bit 4:4:4 + PCM 装 .mov（yuv444p10le——精修套件交换件，调色师/QC 互传的母版）；`ayuv` = 无压缩 8-bit 4:4:4:4 + PCM 装 .mov（yuva444p——必带 α 的无压缩动效图形母版）；`rv10`/`rv20` = RealVideo + RealAudio 1.0 装 .rm（90 年代拨号流媒体编码——ra_144 钉死 8kHz 单声道）；`r210` = 无压缩 10-bit RGB + PCM 装 .mov（gbrp10le——精修无压缩三兄弟里的 RGB 成员）；`v308` = 无压缩 8-bit 4:4:4 + PCM 装 .mov（yuv444p——v410 的 8bit 兄弟）；`rpza` = Apple Video + PCM 装 .mov（rgb555le——QuickTime 1.x 元祖编码）；`speedhq` = NewTek SpeedHQ + PCM 装 .mov/.avi（yuv422p——NDI 时代 NLE 中间件、TriCaster 采集母带）；`roq` = id RoQ + RoQ DPCM 装 .roq（Quake III 时代游戏过场——画幅取齐 2 的幂补边、音频钉死 22050Hz）；`snow` = Snow 小波 + PCM 装 .mkv（ffmpeg 原生实验编码——无损级存档）；`flashsv`/`flashsv2` = Flash Screen Video + MP3 装 .flv（bgr24——录屏时代编码）；`msvideo1` = Microsoft Video 1 + MP3 装 .avi（rgb555le——最古老的 Windows 编码，调色板位深）；`cljr` = Cirrus Logic AccuPak + PCM 装 .mov（yuv411p——90 年代初 QuickTime）；`ac3`/`eac3` = 杜比数字/DD+ 纯音频装 .ac3/.eac3（广播 ATSC 收录 + 流媒体时代环绕声交付规格）；`tta` = TTA 无损纯音频装 .tta（True Audio 存档）；`dca` = DTS 纯音频装 .dts（碟片时代环绕声——4.4 的编码器是实验件，已配 -strict -2）；`raw` = rawvideo + PCM 装 .avi/.mkv（位透交换件——源 pix_fmt 原样穿过不升 4:4:4）；`aiff` = AIFF + 大端 PCM 纯音频装 .aiff（苹果时代无损母带）；`pcm24`/`pcm32f` = 24 位 / 32 位浮点 WAV 纯音频（棚内母带 + DAW 交换件）；`mulaw` = G.711 µ-law 纯音频装 .au 钉死 8kHz 单声道（电话/IVR 收录规格——拒收 --ar/--channels）；`adx` = CRI ADX ADPCM 装 .adx（世嘉时代游戏音频）；`adpcm` = IMA-ADPCM 装 .wav（经典游戏引擎音频）；`alaw` = G.711 A-law 纯音频装 .au 钉死 8kHz 单声道（欧洲电信——mulaw 的姊妹规格）；`speex` = libspeex 装 .spx（Ogg Speex——VoIP/播客时代）；`pcm8` = 8 位无符号 PCM 装 .wav（复古微音频）；`adpcmms` = 微软 ADPCM 装 .wav（经典 Windows/游戏音频）；`g722` = G.722 ADPCM 装 .wav 钉死 16kHz 单声道（宽带电话）；`ra144` = RealAudio 1.0 装 .rm 钉死 8kHz 单声道（拨号规格）；`nelly` = Nellymoser Asao 装 .flv（Flash 时代语音编码）；`wv` = WavPack 无损纯音频装 .wv（发烧友存档）；`mp2` = MPEG Layer II 纯音频装 .mp2（广播/DAB 时代音频）；`caf` = 苹果 CAF 大端 PCM 纯音频装 .caf（GarageBand/Logic 交换件）；`w64` = 索尼 Wave64 24 位 PCM 装 .w64（超 4GB 的 WAV 兄弟——长录音）；`voc` = Creative Voice PCM 装 .voc（DOS 时代游戏音频）；`aptx`/`sbc` = 蓝牙编码纯音频装 .aptx/.sbc（aptX 自动重采样到原生 48k）；`g723` = G.723.1 纯音频装 .tco 钉死 8kHz 单声道（VoIP 时代编码）；`hap`/`hapq` = Vidvox HAP DXT 纹理 + PCM 装 .mov/.avi（live-visual 编码——Resolume/TouchDesigner/VJ 收录件；`--alpha` 升级 `hap` 为 Hap Alpha DXT5，`hapq` 是 YCoCg 高质量变体）；`cfhd` = GoPro CineForm HD + PCM 装 .mov/.avi（运动相机 NLE 中间件 yuv422p10le）；`vc2` = SMPTE VC-2/Dirac + PCM 装 .mov（BBC 广播中间件——探针读 codec `dirac`）；`magicyuv` = MagicYUV 无损 + PCM 装 .avi（NLE 时代快速无损——huffyuv/utvideo 同族）；`r10k` = AJA Kona 10-bit RGB + PCM 装 .mov（gbrp10le 采集卡母带）；`truehd`/`mlp` = 蓝光/HD-DVD 无损纯音频装 .thd/.mlp（实验编码——已配 -strict -2）；`h261`/`h263` = 电信裸流装 .h261/.h263（H.324 视讯会议测试向量——自动取齐 CIF 合法画幅补边，纯视频无音频轨）；`avui` = Avid Meridien 无压缩 + PCM 装 .mov（广播采集卡收录规格——自动取齐 720x486 补边，实验编码 -strict -2）；`ts` = H.264+AAC 装 .ts/.m2ts MPEG-TS（广播收录、IPTV/DVB 归档——--crf/--abitrate 走 h264/aac 路径）；`mxf` = XDCAM/OP1a 广播母带装 .mxf（mpeg2video 4:2:2 + 48kHz 立体声 PCM——死规格，音频参数拒收）；`ivf` = VP9 裸流装 .ivf（纯视频无音轨——MSE/Shaka 测试向量，`--crf` 走真·恒定画质自动补 -b:v 0）；`gxf` = Grass Valley GXF 广播服务器交换规格装 .gxf（mpeg2video 4:2:2 + 48kHz 单声道 PCM——源帧率就近取齐 PAL 720x576@25 或 NTSC 720x480@30000/1001，规格钉死）；`wtv` = Windows Media Center 录制件装 .wtv（mpeg2video+mp2——WMC 时代电视存档）；`smjpeg` = MJPEG+PCM Loki/SDL 游戏过场装 .smjpg（smpeg 时代开源引擎视频）（`proxy` = ≤540p veryfast x264 剪辑代理，多机位素材丝滑预监）；`--gop N` 每 N 帧一个关键帧（平台收录规格 GOP 上限——YouTube 要求 ≤2 秒，短 GOP 拖动更快）；`--profile baseline|main|high` x264 编码档（设备兼容收录规格——老手机/车机/广告机用 baseline；仅 h264/proxy）；`--level 4.1` x264 编码等级、`--bf N` B 帧上限、`--tune film|animation|grain|zerolatency|fastdecode|stillimage|psnr|ssim` x264 内容形态调优（grain 保胶片颗粒、fastdecode 给弱解码端、zerolatency 采集监看管线；仅 h264/proxy）；`--fps` 也可给视频变速帧率；`--vbitrate` 峰值码率帽、`--abitrate` 音频码率（人声帖用 64k 更省）；`--alpha` 保留透明通道（webm/prores/qtrle/hap）；`--range limited|full` 标注色彩范围；`--interlaced` 标记输出为隔行（il 场交织 + tff 标记，广播母带交付）；`--interlace-mode weave` 真时域隔行（60p→30i 逐帧织场）；`--field-order tff|bff|prog` 不重织场、只改场序标签修标错的母带；`--ar 48000`/`--channels 1|2` 重采样率+声道数（广播 48k 立体声、播客单声道）；`--copy-video` 流拷贝画面只重编码音频（修坏音轨/重打包——视频滤镜参数不适用） |
| `compress` | 压到目标体积（`--size 10MB` 两遍——或报名字上限：discord 8MB/nitro 500MB/whatsapp 16MB/gmail/messenger 25MB/wechat 100MB——`--target discord|whatsapp|gmail` 平台预设、`--fps 30` 给 60fps 素材封顶腾运动码率、`--ar 22050` 语音降采样压更紧的预算、`--channels 1` 语音件压单声道）；`--crf` 画质单遍、`--res` 缩分辨率腾码率 |
| `deliver` | 一键平台成片（Reels / TikTok / Shorts 为 9:16，`square` 为 1:1，`youtube` 为 16:9，`xhs` 小红书 3:4 1080x1440，`wechat` 视频号 6:7 1080x1260，`douyin` 抖音/`kuaishou` 快手 9:16 1080x1920，`bilibili` B站 16:9 1920x1080，`pinterest` 2:3 1000x1500，`x` 16:9 1280x720，`linkedin`/`vimeo`/`bluesky` 16:9 1920x1080，`threads` 4:5 1080x1350，`mastodon` 16:9 1280x720，`circle` 1:1 640x640 单声道——Telegram 视频便签，`canvas`/`snapchat` 9:16 1080x1920——Spotify Canvas 循环 / Spotlight，`weibo` 微博 16:9 1920x1080，`twitch` 16:9 1920x1080，`discord` 16:9 1280x720——配 `compress --size discord` 压 10MB 上限，`shopify` 1:1 1080x1080 商品页视频，`amazon` 16:9 1920x1080 商品列表视频，`etsy` 1:1 1080x1080 商品橱窗视频，`rumble` 16:9 1920x1080，`instagram`/`facebook` 4:5 1080x1350——Meta 动态竖屏，`kick`/`vk`/`dailymotion`/`odysee`/`trovo`/`substack` 16:9 1920x1080，`line`/`triller`/`likee`/`moj`/`josh` 9:16 1080x1920（LINE 视频帖/音乐短片/东南亚+印度短视频应用），`lemon8` 3:4 1080x1440，`niconico`/`soop`/`xigua`/`peertube`/`floatplane`/`nebula`/`chzzk`/`douyu`/`huya` 16:9 1920x1080（niconico/SOOP 原 AfreecaTV/西瓜视频/PeerTube/Floatplane/Nebula/CHZZK/斗鱼/虎牙），`weverse`/`kwai`/`snackvideo` 9:16 1080x1920（Weverse Media/快手国际版 Kwai/SnackVideo），`udemy`/`coursera`/`teachable`/`kajabi`/`patreon` 16:9 1920x1080（课程讲座与会员视频帖），`spotify`/`apple`/`amazonmusic`/`iheartradio`/`pandora`/`castbox`/`podbean` 16:9 1920x1080（播客平台的视频单集），`skillshare`/`thinkific`/`podia`/`learnworlds`/`gumroad`/`wistia`/`domestika`/`steam`/`itch`/`shopee`/`lazada`/`taobao`/`dlive`/`minds`/`telegram`/`tidal`/`deezer`/`qobuz`/`yandexmusic`/`napster`/`joox`/`soundcloud`/`mixcloud`/`audiomack`/`bandcamp`/`vevo`/`roku`/`plex`/`iqiyi`/`youku`/`wetv`/`viki`/`crunchyroll`/`funimation`/`mgtv`/`bigo`/`nimo`/`tumblr`/`dribbble`/`behance`/`flickr`/`zhihu`/`kakao`/`naver`/`coub`/`imgur`/`9gag`/`streamable`/`viddsee`/`rutube`/`ok`/`zen`/`openrec`/`twitcasting`/`showroom`/`fc2`/`tving`/`wavve`/`watcha`/`vidio`/`mewatch`/`tver`/`abema`/`hotstar`/`jiotv`/`sonyliv`/`mxplayer`/`zee5`/`showmax`/`shahid`/`truthsocial`/`gettr`/`parler`/`locals`/`utreon`/`caffeine`/`qq`/`tubi`/`pluto`/`dazn`/`espn`/`hulu`/`u-next`/`gyao`/`netflix`/`disney`/`max`/`peacock`/`paramount`/`appletv`/`primevideo`/`globoplay`/`viaplay`/`joyn`/`raiplay`/`atresplayer`/`itvx`/`crave`/`stan`/`mycanal`/`skygo`/`movistar`/`viu`/`voot`/`clarovideo`/`nhk`/`arte`/`tv2play`/`npostart`/`rtve`/`tvp`/`voyo`/`wakanim`/`adn`/`laftel`/`aniplus`/`hidive`/`retrocrush`/`bstation`/`ard`/`zdf`/`nrk`/`svt`/`dr`/`cbc`/`sbs`/`tf1`/`francetv`/`mediaset`/`channel4`/`tenplay`/`nowtv`/`srf`/`fubo`/`sling`/`philo`/`directv`/`xumo`/`vidgo`/`frndly`/`iplayer`/`my5`/`britbox`/`acorntv`/`shudder`/`showtime`/`starz`/`reddit`/`zillow`/`ebay`/`walmart`/`kanopy`/`mubi`/`criterion`/`curiositystream`/`magellantv` 16:9 1920x1080、`poshmark` 1:1 1080x1080、`whatnot`/`tinder`/`bumble`/`hinge` 9:16 1080x1920、`brightcove`/`jwplayer`/`kaltura`/`sproutvideo`/`vidyard`/`uscreen`/`vdocipher` B2B 视频托管 16:9 1920x1080、`mercari`/`vinted`/`depop`/`carousell`/`olx` 二手转卖上架视频 9:16 1080x1920、`jellyfin`/`emby` 媒体服务器 16:9 1920x1080、`onlyfans`/`fansly`/`fanbox`/`cameo`/`subscribestar` 创作者经济订阅帖 9:16 1080x1920、`kofi`/`buymeacoffee` 会员视频帖 16:9 1920x1080、`buzzsprout`/`captivate`/`transistor`/`redcircle`/`sounder`/`acast`/`spreaker` 播客托管平台 16:9 1920x1080、`bandlab`/`distrokid`/`tunecore`/`amuse`/`cdbaby`/`symphonic`/`landr` 音乐分发上传 16:9 1920x1080、`sharechat`/`chingari`/`vmate` 印度短视频 9:16 1080x1920、`blim`/`vix`/`irokotv`/`starzplay` 拉美/非洲/中东流媒体 16:9 1920x1080、`pearvideo`/`haokan`/`miaopai`/`acfun`/`toutiao`/`baijiahao`/`ifeng` 国内视频平台 16:9 1920x1080（梨视频/好看/秒拍/AcFun/头条/百家号/凤凰）、`weishi`/`huoshan`/`quanmin`/`meipai` 国内短视频 9:16 1080x1920（微视/火山/全民/美拍）、`migu`/`pptv`/`letv` 国内流媒体 16:9 1920x1080（咪咕/PPTV/乐视）、`pdd`/`jd`/`vip` 国内电商短视频 9:16 1080x1920（拼多多/京东/唯品会）、`kocowa`/`rakuentv`/`iwanttfc`/`hoichoi` 16:9 1920x1080（韩剧美国 SVOD/乐天 TV 欧 TVOD/菲律宾 iWantTFC/孟加拉 Hoichoi）、`17live`/`pococha`/`mirrativ`/`mxtakatak`/`roposo` 移动直播与印度短视频 9:16 1080x1920、`boomplay`/`sohu` 16:9 1920x1080（非洲音乐平台/搜狐视频）、`pixelfed` 1:1 1080x1080 联邦宇宙方形帖、`artstation` 16:9 1920x1080 作品集托管、`clapper`/`younow`/`meesho`/`bulbul`/`fanvue` 9:16 1080x1920 短视频与直播电商帖、`temu`/`shein`/`aliexpress`/`flipkart`/`zalando`/`coupang`/`mercadolibre` 跨境电商商品视频 9:16 1080x1920、`nba`/`nfl`/`mlb`/`nhl`/`fifa`/`ufc`/`wwe` 联赛高光帖 16:9 1920x1080、`tsn`/`sportsnet`/`beinsports`/`skysports`/`tntsports`/`foxsports`/`cbssports`/`eurosport`/`kayo`/`optussport`/`supersport`/`astro`/`willow`/`premier` 体育转播平台 16:9、`laliga`/`bundesliga`/`seriea`/`ligue1`/`mls`/`championsleague` 联赛高光帖与 `mildom` 日本直播平台切片 16:9 1920x1080、`mtv`/`bet`/`vh1`/`comedycentral`/`nickelodeon`/`cartoonnetwork`/`adultswim` Viacom 电视网切片 16:9 1920x1080、`cnn`/`abc`/`nbc`/`cbs`/`foxnews`/`aljazeera`/`bbcnews` 新闻网切片 16:9 1920x1080、`tmall`/`noon`/`nykaa`/`daraz`/`jumia`/`tiktokshop`/`quikr` 电商商品视频 9:16 1080x1920、`dubizzle`/`wallapop`/`subito`/`kleinanzeigen`/`blocket`/`tradera`/`leboncoin` 分类信息站商品视频 9:16 1080x1920、`dropbox`/`box`/`onedrive`/`gdrive`/`mega`/`wetransfer`/`sendanywhere` 网盘与文件分享视频链接 16:9 1920x1080、`loom`/`tella`/`screenpal`/`vidcast`/`msstream`/`panopto`/`sharepoint` 异步视频与企业视频托管 16:9 1920x1080、`zoom`/`webex`/`gotomeeting`/`bluejeans`/`ringcentral`/`hopin`/`airmeet` 会议与网络研讨会录像上传 16:9 1920x1080、`shutterstock`/`pond5`/`artgrid`/`storyblocks`/`videvo`/`motionarray`/`dissolve` 素材库供稿上传 16:9 1920x1080、`rightmove`/`zoopla`/`realtor`/`redfin`/`domain`/`immoscout`/`idealista` 房产挂牌视频 16:9 1920x1080、`autotrader`/`cargurus`/`carvana`/`carwow`/`mobilede`/`autoscout`/`copart` 车市挂牌视频 16:9 1920x1080、`airbnb`/`booking`/`expedia`/`hotels`/`tripadvisor`/`agoda`/`vrbo` 旅行预订挂牌视频 16:9 1920x1080、`ubereats`/`doordash`/`deliveroo`/`grubhub`/`swiggy`/`zomato`/`meituan` 外卖挂牌视频 16:9 1920x1080、`robinhood`/`etoro`/`webull`/`coinbase`/`binance`/`kraken`/`public` 金融科技推广视频 16:9 1920x1080、`okcupid`/`match`/`grindr`/`eharmony`/`zoosk`/`badoo`/`pof` 交友资料视频 9:16 1080x1920、`medium`/`ghost`/`wordpress`/`squarespace`/`wix`/`webflow`/`framer` 出版/CMS 内嵌视频 16:9 1920x1080、`newgrounds`/`deviantart`/`vsco`/`smugmug`/`zenfolio` 创作者社区与作品集视频帖、`9now`/`7plus` 澳大利亚点播、`plutotv`/`freevee`/`fubotv` FAST 频道与 AVOD 平台、`globo` 巴西流媒体、`pbskids`/`boomerang`/`cartoonito` 少儿电视网切片、`draftkings`/`fanduel`/`bet365`/`williamhill`/`betfair`/`skybet`/`paddypower` 体彩竞猜推广视频、`pga`/`atp`/`wta`/`icc`/`f1`/`motogp`/`nascar` 联赛与赛车高光帖、`orange`/`sfr`/`free`/`proximus`/`swisscom`/`telstra`/`kpn` 电信 OTT/IPTV 机顶盒切片、`indeed`/`glassdoor`/`ziprecruiter`/`seek`/`monster`/`naukri`/`apna` 招聘网站与雇主品牌挂牌视频、`appstore`/`googleplay`/`testflight`/`apkpure`/`galaxystore`/`appgallery`/`fdroid` 应用商店挂牌与内测营销视频——均 16:9 1920x1080（课程、电商、卖场、音乐平台与流媒体/作品集平台视频单）；−14 LUFS；`--fps 60` 高帧率，`--crf` 画质，`--profile`/`--level`/`--bf` 设备兼容三件套（baseline+低等级+0 B 帧给车机/广告机/老手机），`--maxrate`/`--bufsize` CBR 码率帽对（平台收录码率包络——bufsize 缺省 2 倍 maxrate），`--subs file.srt` 成片一步烧字幕，`--channels 1` 单声道语音成片，`--timescale N` 钉死 mp4 视频轨时基（-video_track_timescale——广播收录规格锁时钟的活儿），`--preview SEC` 只渲染成片头部给审核 QC，`--logo mark.png` 角落水印随成片一道烧入（`--logo-position` 四角、`--logo-opacity` 透明），`--intro/--outro clip.mp4` 频道片头+片尾卡自动贴到每个导出上（统一归一到平台画布）、`--lufs -16` 覆盖平台响度目标（自定交付规格））；`--platform podcast` 纯音频播客成片（m4a AAC 128k/48k，−16 LUFS 播客平台标准，`--cover art.png` 内嵌 Apple/Spotify 封面）或 `--platform audiobook`（m4b AAC 96k——Apple Books/Audible 有声书）；成片吃 `--chapters marks.txt`（容器章节——喂的就是 `chapter --yt` 导出的 `mm:ss 标题` 列表——音频成片得 Apple Podcasts 跳转点，视频成片嵌 mp4 章节、YouTube 读成时间线标记）和作品标签 `--title/--author/--album/--genre/--comment`；`--to rtmp://…`/`tcp://`/`udp://` 把成片直接推给采集端（首播）；`--program N` 从多节目传输流里挑一套节目打成片（广播收录直出——`probe.programs[]` 列出各服务） |

| `audiogram` | 波形视频 ，`--mode phase`（aphasemeter 相位表）| `--mode`、`--scale` 幅度、`--split` 分声道、`--fscale` 频率轴（spectrum）、`--fps` 帧率、`--text`、`--bg`、`--progress` 进度条 ，`--subs` 烧字幕、`--from`/`--to` 只取一段（`--to` 可用 `end`），`--at a,b --dur N` 每点一条（`stem_N.mp4`）；`--mode spectrum` 频谱条、`--mode scope` 李萨如矢量示波、`--mode cqt` 钢琴卷帘频谱、`--mode spectro` 滚动频谱图 | `--mode spatial|volume|bitscope` 表桥示波 | `--mode monitor` 管线统计 | `--mode hist` 振幅直方图（削波/余量质检） |

| `split` | 按 `--every`/`--at`/`--scenes`/`--size`/`--parts`/`--silence`/`--chapters` 内嵌章节/`--black`（blackdetect 死画面→每段非黑保留区各出一片）/`--copy`（segment muxer 无损流拷贝切分，秒级完成但边界对齐下一关键帧）切分；`--subs` 输出重定时分段 .srt；`--fade N` 每段首尾淡化 |
| `slideshow` | 图片 → 配乐幻灯视频（`--per` 每图秒数或 `--dur` 总时长、`--fade`/`--transition` 转场、`--motion kenburns` 推拉、`--audio` 配乐 + `--volume` 音量、`--size` 画布、`--bg` 底边色、`--fit` 蒙太奇正好随歌曲收尾、`--shuffle SEED` 确定性乱序——同种子同顺序，图片堆蒙太奇、`--sort name|mtime` 按文件名或拍摄时间排列相机导出照片、`--list manifest.txt` 清单文件自排顺序、`--titles a,,c` 每张图底部字幕条（逗号槽位按最终图片顺序——空槽跳过该图）、`--audio-fade SEC` 配乐尾部长度（默认 0.8——拉长让歌曲余韵留在最后一张图上），`--audio-offset SEC` 配乐从第 T 秒进——跳过前奏用副歌（--fit 按剩余长度量成片），`--audio-loop` 短配乐循环铺满整个蒙太奇（短 jingle 垫长幻灯——与 --fit 互斥），`--audio-fade-in SEC` 配乐头部淡入（与 --audio-fade 尾部淡出对称） |
| `speed` | 变速（`--factor`、`--at/--dur` 窗口（逗号列表可多段）、`--ramp` FROM,TO 渐变），支持 `end`；`--fit SEC` 精确压到目标时长（自动算倍数——90s 素材 `--fit 15` 即 6 倍速） |
| `music` | 铺 BGM，人声出现时压低配乐（`--track`、`--at/--dur` 窗口支持 `end`，逗号 `--at` 多处铺底如 `0,end` 首尾双 sting） |
| `key` | 绿幕合成：`--color` 抠掉后叠到 `--bg` 图片/视频上（`--similarity`、`--blend`、`--despill`、`--mode luma` 改按亮度带抠像（`--threshold` 定亮区/暗区背景）、`--mode chroma` YUV 色域抠像（广播级 chromakey，褶皱/光照不匀的幕布更稳）、`--at/--dur` 窗口抠像，支持逗号列表） | `--mode matte --mask` 外部灰度遮罩→α 通道（prores 4444） |
| `grid` | 多画面宫格（`--layout`、`--audio` 选音轨、`--labels`、`--gap`、`--bg` 格缝颜色、`--fill` 裁满代替黑边、`--time` 每格叠加统一 mm:ss 时间戳、`--focus` 主角布局——首输入占左 2/3 大格，其余纵向排右侧） |
| `progress` | 任意边进度条，整段或定时窗口（`--color`、`--height`、`--edge` bottom/top/left/right、`--at`、`--dur`、`--reverse` 倒计时缩减、`--opacity` 半透明） |
| `freeze` | 定格画面（`--at` 逗号列表多处定格、`--dur`、`--end`、`--ease` 减速、`--reverse` 倒放、`--zoom` 推近定格） |
| `censor` | 区域打码（`--region x:y:w:h`，逗号列表可多处同时打码；`--mode` pixel|blur|solid（solid = 黑条遮盖）、`--strength` 强度、`--at`/`--dur`（逗号列表，需 `--dur`；`end` 可用）、`--shape circle` 椭圆遮罩） |
| `bleep` | 消音哔声：`--at`/`--dur` 选段（逗号列表可消多处；`end` 可用），`--freq`/`--level` 调音 |
| `boomerang` | 正放+倒放回弹循环（一段，社交平台常见玩法）（`--times` 循环次数） ，`--at/--dur` 局部往返——逗号列表可多处回弹，支持 `end` |
| `chapter` | 在 `TIME|TITLE` 写入章节或 `--import` 导入标记文件（支持 YouTube `H:MM:SS Title` 行）；`--auto` / `--export` ffmeta / `--yt` 导出 YouTube 描述格式 / `--cue` CUE 表 / `--podcast` Podcasting 2.0 JSON 章节（有声书/播客播放器）/ `--lrc` LRC 同步歌词标记文件 / `--vtt` WebVTT 章节文件（网页 `<track kind="chapters">` 点击跳段）；`--spread N` 等距网格标记（`--titles a,b,c` 命名——长节目统一目录）；`--csv` 导出 `H:MM:SS.mmm,标题` 行（Resolve/Premiere 标记导入、表格编辑回环——带引号标题也收）；`--edl` 导出 CMX 式 EDL（按成片帧率——NLE 时间线标记导入）；`--fcpxml` 导出 Final Cut Pro XML——FCP/Resolve 把每个标记导入为时间线标记（编辑器原生目录交换）；`--srt` 把标记导出成软字幕——每个章节标题成一条 cue、跨到下一条标记/片尾（烧成字幕预览章节落点）；`--import` 自动识别 .json/.cue/.lrc/.vtt/.csv/.srt/.fcpxml/.ffmeta/.edl 文件（CMX 事件——rec-in 时码按 `--fps` 30 缺省解码）（.srt 字幕稿直接变目录——每条 cue 成为一个章节标记、首行作标题）；`--scenes` scdet 剪切点检测生成标记（同 `split --scenes` 的检测器——无章节母带自动出目录）；`--list`；`--remove`；`--shift` 平移标记；`--rate R` 全部标记时间 ×R（给变速后的成片重对目录——先缩放后 --shift 平移）；`--min-gap SEC` 剔除与上一条保留标记间隔小于 SEC 的标记（密集目录瘦身——`--scenes` 在快剪素材上过触发；首个标记永远保留，extras 报 `min_gap_dropped`）；`--snap` 把每个标记重新对齐到最近关键帧（播放器按关键帧跳转——没对齐的章节落点会跳晚；extras 报 `snapped`）|
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
| `meta` | 容器标签（`--title`/`--artist`/`--album`/`--genre`/`--date`/`--track`/`--disc`/`--composer`/`--bpm`/`--lyrics file.lrc`/`--copyright`/`--comment`，另加 `--album-artist`/`--show`/`--season`/`--episode`/`--network` 剧集/播客订阅源标签组，再补 `--creation-time`/`--location` 归档时间戳与拍摄地戳 + `--media-type` iTunes 类型原子 + `--gapless` 无缝专辑标记 + `--description`/`--synopsis` 单集文案与简介 + `--hd` iTunes 高清徽标 + `--lang-audio eng,jpn`/`--lang-subs eng,fra` 按轨序打语言标签——空槽位跳过，播放器菜单显示音轨/字幕语种 + `--title-audio`/`--title-subs`/`--title-video` 按轨序打显示名——播放器显示轨道名而不是「Track N」（多机位角度名；流标题要 mkv——mp4 会丢））+ `--rotate`、`--clear` 显示旋转，无损拷贝 |
| `subs` | 提取（`--stream`、`--all` 全部）/烧录/封装字幕（`--shift`（±N；`--from`/`--to` 可只平移窗口内字幕）/`--merge`/`--rate`、烧录样式 + `--outline`/`--box` 衬底/`--align`/`--from`/`--to` 窗口（支持 `end`/`end-N`）、`--margin` 像素边距、`--safe`）；`--convert` .srt↔.vtt 互转；`--case` 大小写；`--burn-si N` 直接烧内嵌第 N 条字幕轨；`--encoding gbk` 解码老编码字幕文件；`--sort`/`--fix-overlaps`/`--dedupe` .srt 字幕体检（重排/收齐重叠/剔重复）+ `--min-dur SEC` 闪字幕最小时长地板（顶到下一条起点为止）+ `--min-gap SEC` 相邻 cue 最小间隔（不够就削前一条尾巴——广播规格约 2 帧）+ `--join SEC` 相邻 cue 间隔小于 SEC 即合并成一条（自动转写稿碎句修复——Whisper 式碎块接回整句，文本空格衔接、时轴覆盖合并区间）+ `--cps N` 字幕语速闸口（`over_limit`/`worst_cps`——可读性规格）+ `--max-lines N` 行数报告（广播规格 2 行）+ `--replace OLD,NEW` 全文件查找替换 + `--strip-speakers` 剥 `[NAME]`/`全大写:` 说话人标签（自动转写稿）+ `--wrap N` 按每行 N 字重排版 + `--append b.srt` 把第二个 .srt 接到第一个末尾（配合 `concat` 拼片后合字幕）+ `--convert` 输出 `.txt` 纯文本转写稿或 `.ass` 最小化带样式 ASS（Aegisub/番组字幕管线交接——`.ass` 作输入也行，认 Format 行列序）或 `.lrc` 同步歌词文件（转写稿→卡拉OK/音乐播放器歌词——下一行时间戳当本行收尾）或 `.ttml`/`.dfxp` 最小化 TTML/DFXP（广播/Netflix 字幕交换格式——文本 XML 转义、<br/> 换行；`.ttml`/`.dfxp` 作输入也解析 `<p begin end>` cue）或 `.sbv` YouTube SubViewer 字幕（`H:MM:SS.mmm,H:MM:SS.mmm` 头行——Studio 可编辑上传格式；作输入也回读）或 `.csv` `start,end,"text"` 表格行（引号单元格保留逗号/引号/换行——Sheets/Excel 里改完字幕再转回；`.sub` MicroDVD `{f}{f}text` 帧号式字幕读入（`{1}{1}fps` 声明行优先否则用 `--fps`——老番/老片档字幕）并新增 `.sub` 写出（按 `--fps` 帧率回写，缺省 25）；`.mpl` MPL2 `[s][e]text` 厘秒制波兰老档字幕双向（`|` 折行）；`.smi` SAMI `<SYNC Start=ms>` 双向（Windows Media 时代字幕——cue 到下一条 SYNC 为止；写出时每 cue 一条 `<SYNC><P>`）；`.scc` Scenarist 隐藏字幕读入（608 十六进制对走 ffmpeg demuxer 解码——广播 CC 交付件回读；scc 复用器只透传所以 srt→scc 写出不做）；`.stl` Spruce 广播字幕、`.rt` RealText 与 `.mps` MPlayer start+duration 行读入（同款 demuxer 委托） + `.pjs` Phoenix `start,end,"text"` 输入（厘秒行——老番字幕档案，ffkit 自己解析）；`.psb` PowerSub `{hh:mm:ss.mmm}{hh:mm:ss.mmm}text` 输入（另一种花括号格式——MicroDVD 花括号里是帧号，PSB 花括号里是时间戳；ffkit 自己解析）；`.jss` JACOsub `HH:MM:SS.CC HH:MM:SS.CC text` 输入（厘秒时标行——`{...}` 事件括号剥除、`|` 折行、指令行跳过；4.4 的 demuxer 输出是坏的所以 ffkit 自己解析）；`.sub` 按内容判形——`{f}{f}` MicroDVD 行按帧号解析（缺率仍报 --fps），其余走 ffmpeg 的 subviewer demuxer（SubViewer v1+v2——这个扩展名在野有两种格式）+ `--move N,T` 把第 N 条 cue 平移到 T 起始（按输入文件编号——只挪一条对歪的字幕，时长保留）+ `--split 3,8` 在逗号时间点切 .srt 成 `stem_0.srt`/`stem_1.srt`… 分段（每段 cue 重新对时轴——配合 `split`/`cut` 出的分段拆字幕）+ `--resync O1,O2,N1,N2` 两点线性重同步——旧时刻 O1,O2 映到新时刻 N1,N2（偏移+漂移一趟修，给换剪辑版本的字幕重新对轴）+ `--find needle` 只保留文本含 needle 的 cue（大小写不敏感，extras 报 `found`——定位每一次口癖/口头禅，再围绕它 cut/caption；`--convert`/`--shift`/`--burn` 等独立趟前置拒绝混用）+ `--strip-tags` 剥 cue 文本内联标记——`{\…}` ASS 覆盖块（`<…>` 标签读入时已剥——烧录时会显示成原样的转写稿）+ `--strip-sdh` 剥 `[SOUND]`/`(door creaks)`/`♪` 环境音/音乐注释——烧录用纯对白稿（变空的 cue 会剔除）+ `--strip-emotes` 剥 emoji/图形字符（广播字幕 608/708 链路与台标收录拒收——广播安全转写稿） + `--rtl` 每行 cue 文本包 U+202B..U+202C RTL 标记（阿语/希伯来语字幕没有标记时播放器镜像标点——中东北非字幕交付；extras 报 rtl_wrapped）+ `--clip F,T` 只保留与窗口相交的 cue、边界收齐并重对时轴（`end` 可用——抓 `cut`/`split` 分段对应的字幕块）+ `--drop F,T` 剔除窗口内的 cue 并把尾部提前 (T-F) 秒——`cut --drop` 的字幕镜像（一趟连段带字幕一起删）+ `--dedupe-text` 剔除文本与上一条保留 cue 重复的 cue（自动转写稿重复行修复——时间轴不同也算，大小写/空格不敏感）+ `--fix-cps N` 把超 CPS 的 cue 拉伸到每秒 N 字符为止（`--cps` 闸口的自动修复——顶上以不越下一条 cue 起点为限；extras 报 `stretched`）+ `--fix-lines N` 把超 N 行的 cue 按行界拆成多条连续 cue（`--max-lines` 闸口的自动修复；extras 报 `lines_split`）+ `--speakers` 报转写稿说话人名单（extras：`speakers`/`speaker_count`——与 `--strip-speakers` 配对的报告半）+ `--stats` 报转写稿统计（extras：`cues`/`words`/`chars`/`span_secs`/`median_dur_secs`——朗读节奏/稿长 QC）|
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
| `scan` | 质检报告：黑场/冻结帧/黑帧计数 + 频闪 `flash_frames`/`flash_max_badness` + 隔行判定 `interlaced`/`frames_tff`/`frames_bff`/`frames_progressive` + 立体声 `phase_corr`（≈-1 表示单声道抵消）（idet；JSON extras；不写媒体）  `audio_max_db`/`audio_mean_db` 峰值/平均电平 | +模糊质检 | `--scenes` 剪切点时间戳 | `luma_min/max` + `illegal_luma`（signalstats 广播范围质检） | `noise_floor`/`noisy` 位平面噪底（码率预算质检） | `has_cc`/`cc_lines` EIA-608 隐藏字幕 | `crop_hint`/`letterboxed` cropdetect 黑边质检 | `vfr`/`vfr_ratio`/`vfr_frames` 可变帧率质检 | `--dupe REF` MPEG-7 签名重复/搬运检测 | `--text` OCR 烧录文字识别（`text`/`text_frames`/`text_confidence`） | `rg_gain_db`/`rg_peak` ReplayGain 标签 | `--motion` VMAF 运动分（`motion_avg`/`motion_max`，码率预算质检） | `--timecode` VITC 时间码读取（`vitc`/`vitc_tc`/`vitc_frames` 广播母带质检） | `--bbox` 内容包围盒（`content_detected`/`content_box`/`content_fill` — 不限黑底的纯色背景也可用） | `sat_mean`/`hue_mean`/`y_mean` 色调质检（发灰/偏色/节目亮度，同趟解析） | `--deadair DB` 死空气段质检（`deadair_secs`/`deadair_ranges`，播客/口播静默图） | `--loud` EBU R128 响度质检（`loud_i`/`loud_lra`/`loud_tp`，平台响度闸口；纯音频文件也可用）、`hdr`/`wide_gamut`/`color_space`/`color_primaries`/`color_transfer` 容器色彩标签直读 HDR/广色域质检 | `--gop` 关键帧间隔质检（`keyframes`/`keyframe_times`/`gop_max_sec`/`gop_avg_sec`/`gop_max_frames`，只读 packet 标记不解码，「关键帧 ≤2s」平台准入规格） | `--hash` 逐帧校验码写 `<input>.framemd5`（归档入库完整性清单） | `timecode` 容器时码质检（mov tmcd / mkv TIMECODE，母带对板核验，不解码） | `--bitrate` packet 图码率曲线（`bitrate_mean_mbps`/`bitrate_peak_mbps`/`bitrate_spike_at`，峰值码率准入规格质检，不解码） | `av_desync_ms` 音/视频 start_time 偏移（容器级唇音同步质检，不解码；配 `remux --audio-delay`/`--video-delay` 即修即验） | `--packets` 逐轨真实包数（`streams_counted`/`packet_mismatch`——ffprobe -count_packets 复跑：容器头声明 nb_frames 与实际包数对比——截断文件质检，不解码） | `--verify` 全解码质检趟（`decodes_clean`/`decode_errors`/`first_error`——损坏收录闸口；ffmpeg 把损坏帧按 warning 打日志不报错退出） |
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
| `remux` | 换容器不重编码（`-c copy` + faststart，`--timescale N` 钉死 mp4/mov 视频轨时基——收录规格锁时钟）；`--bsf` 拷贝路径上的编码级位流修复——annexb/hevc-annexb 转 .ts 广播收录、adts 把网络电台 ADTS 抓流装进 .m4a、mjpeg-jpg/fix-subs/redundant-pps/extract-extra 修复包头与字幕时序；`--audio` 只提音轨，`--video` 只留视频，`--aspect 16:9` 修显示宽高比，`--frag` 分片 MP4（moof/mfra——还在写入就能播，HLS/DASH 管线用），`--no-subs` 剥掉字幕/数据流（干净交付件），`--itsscale R` 不重编码整容器变速（时间戳 ×R——1.042 PAL 25→24 下拉、0.96 电影→PAL 加速，音频随画面变），`--offset SEC` 设定容器 start_time（修采集文件负值/异常起点），`--from SEC`/`--to SEC` 无损裁剪（关键帧精度——不重编码只打包一段），`--lang jpn` 只保留标记该语言的音轨（多语言发行、配音提取；配 `--audio` 即只抽该语言轨），`--default-audio N` 把第 N 条音轨设为播放器默认轨，`--cover pic` 把图片作为 attached_pic 流挂进成片（纯音频得封面、视频得缩略海报），`--chapters marks.txt` 把 YouTube 格式章节列表嵌成容器章节（就是 `chapter --yt` 导出的那个文件——Apple Podcasts/Books 直接变跳转点），`--title`/`--artist`/`--album`/`--genre`/`--comment`/`--date` 重打包时顺手写音乐库标签，`--lang eng,jpn` 逗号列表保留多条配音轨，`--strip-meta` 抹掉继承来的全部容器标签（隐私清理——配合标签参数可擦完顺手重写）、`--no-cover` 剥掉 attached_pic 封面流（精简有声书/m4a）、`--encrypt` 重打包时 CENC AES-CTR 加密（仅 mp4/mov——ClearKey/Widevine DRM 预处理，`--key`/`--kid` 32 位十六进制可填、不传则随机，JSON 会回显）、`--audio-delay SEC` 音轨整体平移对视频——不重编码修唇音同步（负值为音频提前），`--video-delay SEC` 另一半修法：采集卡画面滞后的救星（正值延后画面、负值提前；与 --audio-delay 互斥），`--tag hvc1` 重写 codec tag 让 HEVC mp4 能在 QuickTime/Safari 播放（仅 mp4/mov），`--attach f` 把二进制附件流嵌进 mkv/webm（字幕字体随文件走；可重复），`--audio-order 1,0` 重排+保留音轨（按轨索引，没列的轨丢弃——只放第 0 轨的老播放器要节目混音轨在最前），`--sub-lang fra` 只保留标记该语言的字幕轨（逗号列表保多条——多字幕发行件挑语种），`--sub-order 1,0` 按轨索引重排+保留字幕轨（多字幕发行件把观众字幕放最前——没列的轨丢弃），`--keep 0,3` 只保留列出的绝对流索引（`probe.streams` 可查序号——按类型排序表达不了时的兜底项，没列的流丢弃），`--decrypt HEX` 解 CENC 加密源（32 位十六进制密钥——与 `--encrypt` 配对即密钥轮换），`--copy-ts` 原样保留输入时间戳（采集管线要留墙钟 pts——与时间戳改写参数互斥），`--video-order 1,0` 按类内索引重排+保留视频轨（多机位文件选主机位——没列的轨丢弃，配 `--video` 即抽单角度），`--forced-sub N` 把字幕轨 N 标记为 FORCED 强制字幕（电影式强制翻译——播放器按观众语言自动显示；仅 mkv/webm，mp4 表达不了该标记），`--sdh N` 把字幕轨 N 标记为 SDH/听障字幕（无障碍规格——播放器标注「SDH」；仅 mkv/webm），`--commentary N` 把音轨 N 标记为评论轨（导演评论音轨；仅 mkv/webm），`--audio-desc N` 把音轨 N 标记为口述影像/visual_impaired 轨（无障碍 AD 规格——播放器标注「AD」；仅 mkv/webm），`--dub N` 把音轨 N 标记为配音轨，`--original N` 标记为原声轨（媒体服务器靠它分原声/配音——仅 mkv/webm），`--default-video N` 多机位文件选默认视频轨（`--default-audio`/`--default-sub` 管其他轨类），`--no-video` 剥掉视频流但保留音轨/字幕/封面/附件（带封面的音频交付件——`--audio` 是只提音轨，这个是「只去画面」），`--no-audio` 剥掉音轨但保留画面/字幕/封面（静音 B-roll/录屏素材库——`--video` 是只提视频轨），`--no-attachments` 剥掉内嵌字体等附件流（`--no-cover` 只管 attached_pic 封面），`--drop 1,3` 只丢弃列出的绝对流索引（--keep 的逆操作——抽掉一条评论轨/一种语言，其余全保留），`--no-chapters` 重打包时剥掉容器内嵌章节（干净交付件——有的播放器会把坏目录显示出来），`--genpts` 读入时重建缺失/损坏时间戳（-fflags +genpts——相机/截断文件拖不动或探测时长为零的修复件；与 `--copy-ts` 互斥）、`--program N` 从多业务传输流只保留一个节目（广播解复用——`probe.programs[]` 列每个节目的业务名+成员流；与流挑选参数互斥），`--no-data` 剥遥测/定时元数据数据流（GoPro gpmd 私有轨——mpegts data codec 只有 .ts 目标装得下，带它走的输出保持流拷贝），`--muxrate R` 恒定传输流复用码率（-muxrate——广播收录规格把复用垫到固定码率；仅 .ts/.m2ts 目标），`--brand mp42` 改 major_brand 原子（拒收 isom 的设备收录件——仅 mp4/mov），`--service-name X`/`--provider Y` 写 SDT 频道/网络台标、`--service-id N` PAT 节目号、`--tsid N`/`--network-id N` 复用体标识（DVB/IPTV 收录规格——仅 .ts/.m2ts，probe.programs[] 回读台标）、`--start-pid N`/`--pmt-pid N` 分配 PID 计划的基础流/PMT 槽位（DVB/IPTV 收录按 PID 规划频道——`probe.streams[].stream_id` 以十六进制回读）、`--resend-headers` 逐包重发 PAT/PMT（中途加入播放的中流采集件）、`--cmaf` 写 CMAF 互操作分片 mp4（一份分片包 HLS fMP4 与 DASH 通吃）、`--mdta` 全部标签写为 mdta 原子键（udta 写不了自定义元数据键）、`--skip-trailer` 丢弃 `--frag` 包的 mfra 尾录（不回看的直播收录管线）、`--isml` Smooth Streaming piif/uuid 序幕盒（IIS 平滑流收录件）、`--rtphint` 逐轨加 RTP hint 轨（live555/DSS 收录预处理——仅 mp4/mov）) |
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
| `frames` | 每 `--every`、`--at` 秒抽一帧（`end` = 最后一帧）、`--count` 均布 N 帧、`--nth N` 每隔 N 帧抽一帧（数据集/质检采样）、`--number N` 精确抽第 N 帧（0 基解码序——按帧号定点抓坏帧，VFR 上 --at 的时间换算会漂移；逗号列表 `0,5,12` 一趟抓多个帧号）→ `stem_001.png…`（`--width` 缩放）；`--untile CxR` 把每帧拆成瓦片静帧（还原宫格图/马赛克） |
| `countdown` | 画面倒数（`--from` 最多 600、`--beep` + `--tone` 蜂鸣频率、`--text`、`--position`、`--bg` 数字底板、`--format` mm:ss/h:mm:ss 长倒计时、`--opacity` 半透明、`--target HH:MM` 按本地挂钟倒数到首播/开播时间，每秒一数最多 10 分钟，时间已过自动顺延明天，`--utc` 目标改按 UTC 读（跨时区共用同一张播出表）） |
| `invert` | 全帧或定时反色（`--at`、`--dur`——逗号列表可多段） |
| `mix` | 双音轨叠加（`--vol-a/--vol-b`、`--at/--dur`（逗号列表可多次进床）、`--loop`、`--duck` 人声闪避音乐、`--gate` 更狠的门限闪避——说话时底床直接静音） ，`--normalize` 归一求和、`--fade` 淡入淡出，支持 `end` |
| `mute` | 去掉音轨（其余流直接封装，不重编码） ，`--at/--dur` 局部静音（逗号列表可静多处，需 `--dur`），支持 `end` |
| `timer` | 画面计时器（`--position`、`--format ms`、`--box-color` 底板） （`--format`、`--box-color`、`--down` 倒计时、`--start` 设定起始读数、`--opacity` 半透明）——`--at` 支持 `end`；`--tc HH:MM:SS:FF` 烧录走带时码（样片/审片，`;` 前 FF 位表示丢帧意图，仅显示）；`--clock` 烧录本地挂钟 HH:MM:SS（赛事/活动实时钟），`--date` 烧录本地日期 YYYY-MM-DD（归档/播出日期戳——静态读数；和 --clock 同用时排在时间前），`--utc` 按 UTC 读钟/日期（播出日志、跨时区协作） |
| `hls` | 网页 HLS 封装（`--seg`、`--single`、`--copy`、`--ladder` 多码率、`--audio-only` 纯音频、`--video-only` 静音/预览分片、`--fmp4` CMAF、`--poster` 同时输出 poster.jpg 封面，`--poster-at T` 选封面帧，`--encrypt` AES-128 加密分片并写 key.bin/key.info（`--key HEX` 自定义密钥、`--key-uri URI` 播放列表里的密钥地址）→ 私有/付费流，`--rekey` 每个分片重读密钥文件（密钥轮换），`--temp` 分片与播放列表先写临时文件再改名（直播读者/nginx 不会读到写一半的 .ts/.m3u8），`--round-durations` EXTINF 取整秒（老播放器/严格校验器不收小数时长），`--master NAME` 重命名 --ladder 的主播放列表（多频道 ABR 目录各挂各的 master），`--init NAME` 命名 fMP4 初始化分片（-hls_fmp4_init_filename——fMP4 包的 CDN 路径规划；需配 --fmp4），`--live` 滑动窗直播播放列表——播放器可中途加入，只保留最新 `--live-window N` 个分片、不写 endlist；`--start N` 重启后从 N 续编号、`--epoch` 用 epoch 时钟播种分片序号（24/7 频道不停表）、`--date` 给分片写 EXT-X-PROGRAM-DATE-TIME 时间戳、`--discontinuity` 标记推流重启点、`--time-names` 分片按墙钟时间命名（归档录像文件名即时间）、`--independent` 打 EXT-X-INDEPENDENT-SEGMENTS 并强制每分片首帧关键帧（拖动/变速播放 VOD），`--iframes` 打 EXT-X-I-FRAMES-ONLY——播放器读它做拖动预览（配 --independent 让分片边界真是关键帧）、`--base-url URL` 给播放列表里每个分片条目加 URL 前缀——清单留本地、分片走 CDN、`--name PFX` 分片文件名前缀（所有命名方案都生 `v-seg_…`——多个包同目录共存）、`--utc URL` 直播清单的 UTCTiming 时钟（播放器按墙钟同步直播边缘）、`--program N` 多业务传输流里只封装一套节目——广播信号转流媒体采集（probe.programs[] 列节目；叠 --ladder 时用该节目的成员流建梯度）) |
| `dash` | DASH 封装 → manifest.mpd + init-/seg-*.m4s（`--seg`、`--copy` 不重编码直接切片、`--single` 每表示层单文件按字节请求、`--webm` vp9+opus 分片、`--window N` 滑动窗直播清单、`--ladder 1080,720,480` ABR 自适应——单一清单里 N 档视频表示层按梯度码率供播放器随带宽切换、`--streaming` 每帧一个 moof 分片（低延迟 DASH 预处理）、`--frag SEC` 每分片内按 SEC 一个 moof 分片（拖动预览预处理）、`--sidx` 在 `--single` 字节请求文件里写全局 SIDX 索引盒（HTTP 区间拖动）、`--video-only` 静音/预览分片包、`--name PFX` 分片文件名前缀（`v-init-…`/`v-seg-…`——多个包同目录共存）、`--program N` 多业务传输流里只封装一套节目——广播信号转流媒体采集（probe.programs[] 列节目；叠 --ladder 时用该节目的成员流建梯度）、`--dvb` 写 DVB-DASH 广播 profile（广播收录规格点名 DVB profile 而非普通 MPEG-DASH）) |
| `live` | 把片段推向直播采集端：`--to rtmp://…`/`rtmps://`/`tcp://`/`udp://`/`srt://`（SRT/UDP 走 MPEG-TS 封装）（`-re` 实时节奏推流，x264/aac 采集编码），`--codec hevc` 贡献级 HEVC（仅 MPEG-TS 传输），`--subs file.srt` 直播同步烧字幕，`--loop` 无限循环（24/7 音乐台/首播轮播），`--vbitrate`/`--abitrate` 可调码率，`--crf 0-51` 恒定画质编码替代 -b:v（与 --vbitrate/--maxrate/--bufsize 冲突），`--scale WxH` 大母片降采样到采集规格，`--fps N` 压输出帧率，`--record file.mp4` 推流同时本地存档（tee——一次编码两路封装），`--until SEC` 定时停播，`--list` 清单轮播（concat 清单文件，24/7 轮播台；`--loop` = 无限轮播），`--test` 内置测试卡+1kHz 音（开播前验证推流密钥），`--slate card.png --slate-dur SEC` 开播前先推「即将开始」定场图（首播/定时开场），`--card art.png` 给纯音频源配一张常驻静帧当画面（24/7 lofi 电台流），`--overlay bug.png` 台标钉在角落（`--overlay-position tl|tr|bl|br`、`--overlay-opacity` 半透明），`--restream url` 同时推第二个采集端——一次编码多平台直播，`--gop N` 关键帧间隔对齐采集规格（YouTube 要求 ≤2s），`--preset` x264 速度/画质档位，`--vertical` 信箱到 1080x1920 竖屏直播画布、`--maxrate 4500k`/`--bufsize` CBR 限幅对齐采集端码率规格（Twitch ≤6000k，bufsize 默认取 maxrate 两倍）、`--start T` 从素材第 T 秒开始推流——长录像剪掉头直接从中段重播，`--title "节目名"` 把节目名写进 FLV/TS 流元数据与 --record 存档（采集端面板显示），`--volume 0..4` 推流音频增益缩放（BGM 太响不用重渲染直接压；--no-audio 下拒绝），`--channels 1` 单声道推流（语音/电台采集规格——--no-audio 下拒绝），`--audio-delay SEC` 推流音频整体延后（adelay——采集卡声音跑在画面前的修正件；--no-audio 下拒绝），`--hold SEC` 开播前先冻结首帧+音频静默 SEC 秒（tpad 克隆 + adelay——采集端预热期健康检查；与 --slate/--card 冲突），`--loudnorm` 推流音频单趟动态响度归一（广播 −23 LUFS 规格；--no-audio 下拒绝），`--rw-timeout SEC` 采集端卡住超过 SEC 秒即中止推流（socket 停摆看门狗——注意别和全局 `--timeout` 进程超时混了；仅单路推流可用），输入可直接填直播 URL——`live rtmp://…`/`udp://…`/`http://…` 把一路流接力推到另一个采集端（转播；--list/--start/--loop/--slate 仅文件可用会明确拒绝）) |

| `qa` | 对比参考视频测画质损失（PSNR + SSIM + MSAD + VIF，`--metric`） |
| `conform` | 一键统一规格（`--size WxH`、`--fps 30`、`--lufs -14`、`--crf`、`--maxrate`/`--bufsize` CBR 码率帽对（收录码率包络——bufsize 缺省 2 倍 maxrate）、`--pad` 黑边颜色 + `--anchor` 锚点、`--blur` 模糊填充，`--hold SEC` 克隆末帧片尾停留 + `--hold-start` 片头预停（音频自动补静音）、`--even` 奇数像素向下取偶（手机/录屏奇数尺寸吃不下 yuv420p 的一键修）；`--ar HZ` 音频重采样率（默认广播 48k；44100 播客/CD、96000 母带）；`--channels 1` 单声道母带、`--program N` 多业务传输流里只取一套节目走规格趟（广播收录——`probe.programs[]` 列服务，取该业务首视频+首音频成员）、`--profile/--level/--bf` 设备兼容三件套（baseline+低等级+0 B 帧给车机/广告机）、`--rotate 90|180|270` 规格趟内���置画面（竖屏手机素材一趟转成横屏规格——先于 --size 画布计算）、`--timescale N` 钉死 mp4 视频轨时基（广播收录规格锁时钟——仅 mp4/mov） |
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

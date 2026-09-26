use std::path::PathBuf;
use std::time::Duration;

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser, Debug)]
#[command(
    name = "ffkit",
    version,
    about = "FFmpeg hands for AI agents: probe, edit, verify",
    after_help = "Flags for each verb: ffkit <verb> --help. Numbers come from --json."
)]
pub struct Cli {
    /// Plan the command without writing output
    #[arg(long, global = true)]
    pub dry_run: bool,
    /// Full JSON contract on stdout
    #[arg(long, global = true)]
    pub json: bool,
    /// Short JSON contract (status, output, summary, verified)
    #[arg(long, global = true)]
    pub json_brief: bool,
    /// Replace an existing output file
    #[arg(long, global = true)]
    pub overwrite: bool,
    /// Kill ffmpeg after this many seconds (default 1800)
    #[arg(long, global = true, default_value_t = 1800)]
    pub timeout: u64,
    /// Show ffmpeg progress on stderr
    #[arg(long, global = true)]
    pub progress: bool,
    #[command(subcommand)]
    pub cmd: Cmd,
}

#[derive(Clone, Debug)]
pub struct Globals {
    pub dry_run: bool,
    pub json: bool,
    pub json_brief: bool,
    pub overwrite: bool,
    pub timeout: Duration,
    pub progress: bool,
}

impl From<&Cli> for Globals {
    fn from(cli: &Cli) -> Self {
        Self {
            dry_run: cli.dry_run,
            json: cli.json,
            json_brief: cli.json_brief,
            overwrite: cli.overwrite,
            timeout: Duration::from_secs(cli.timeout),
            progress: cli.progress,
        }
    }
}

#[derive(Subcommand, Debug)]
pub enum Cmd {
    /// Report whether ffmpeg/ffprobe and key encoders/filters exist
    Doctor,
    /// Probe a media file (duration, size, codecs, fps)
    Probe {
        /// Media file
        input: PathBuf,
    },
    /// Contact sheet or single frame — look at the picture
    Look(LookArgs),
    /// Cut a time range (lossless copy by default)
    Cut(CutArgs),
    /// Concatenate clips
    Concat(ConcatArgs),
    /// Split into fixed-length parts
    Split(SplitArgs),
    /// Scale, crop, pad, rotate to a frame
    Fit(FitArgs),
    /// Extract audio, a frame, or subtitles
    Extract(ExtractArgs),
    /// Overlay an image or video (logo, PiP)
    Overlay(OverlayArgs),
    /// Cut away to B-roll; keep A-roll audio and duration
    Broll(BrollArgs),
    /// Burn or mux subtitles
    Caption(CaptionArgs),
    /// EBU R128 loudness normalisation (two-pass)
    Loudnorm(LoudnormArgs),
    /// Denoise voice (fan / rumble / hiss); --video also degrains
    Denoise(DenoiseArgs),
    /// Transcode to a delivery preset (h264, webm, gif)
    Transcode(TranscodeArgs),
    /// Shrink to a target file size (two-pass bitrate budget)
    Compress(CompressArgs),
    /// One-shot 9:16 social export (Reels / TikTok / Shorts)
    Deliver(DeliverArgs),
    /// Audio + cover still into a 9:16 waveform video (podcast audiogram)
    Audiogram(AudiogramArgs),
    /// Change playback speed (talking-head 1.1–2×, slow-mo)
    Speed(SpeedArgs),
    /// Mix a music bed under speech, with ducking
    Music(MusicArgs),
    /// Replace a video's audio track (lav mic, clean voice, new music)
    Replace(ReplaceArgs),
    /// Green-screen composite: foreground over a background image/video
    Key(KeyArgs),
    /// Side-by-side / reaction grid over N inputs (xstack)
    Grid(GridArgs),
    /// Bottom/top progress bar filling over the duration
    Progress(ProgressArgs),
    /// Hold a frame mid-clip or freeze the last frame (outro)
    Freeze(FreezeArgs),
    /// Mosaic/blur a region (face, logo, license plate)
    Censor(CensorArgs),
    /// Forward + reversed replay loop
    Boomerang(BoomerangArgs),
    /// Audio FX rack: tremolo/vibrato/flanger/phaser/chorus
    Fx(FxArgs),
    /// Auto-align a second recording to a reference by audio (multi-cam)
    Align(AlignArgs),
    /// Embed chapter markers (lossless metadata pass)
    Chapter(ChapterArgs),
    /// Detect and remove black bars (cropdetect scan + crop)
    Autocrop(AutocropArgs),
    /// Contact sheet: cols×rows thumbnails from the whole clip
    Sheet(SheetArgs),
    /// Seek-preview sprite sheets + WebVTT for video players
    Sprite(SpriteArgs),
    /// Pitch-shift audio by semitones (voice effects, music retune)
    Pitch(PitchArgs),
    /// Strip dead air at the head and tail (audio-only)
    Cutsil(CutsilArgs),
    /// Channel surgery: dual-mono, mono fold-down, L/R swap
    Channel(ChannelArgs),
    /// Audio EQ: bass/treble/presence shelves
    Eq(EqArgs),
    /// Room ambience on a voice (reverb tail)
    Reverb(ReverbArgs),
    /// Mute a moment under a 1 kHz beep (swear / spoiler censor)
    Bleep(BleepArgs),
    /// Rotate a video 90/180/270 deg or mirror it
    Rotate(RotateArgs),
    /// Blur out a burned-in logo/watermark box
    Delogo(DelogoArgs),
    /// Write container metadata tags (title/artist/comment) - lossless copy
    Meta(MetaArgs),
    /// Extract an embedded subtitle track to .srt/.vtt
    Subs(SubsArgs),
    Thumb(ThumbArgs),
    Solid(SolidArgs),
    /// Still images (+ optional music bed) into a video
    Slideshow(SlideshowArgs),
    /// Cut internal silence (talking-head jump cuts)
    Jumpcut(JumpcutArgs),
    /// Map speech islands in a long take, then lossless/cheap assemble
    Rough(RoughArgs),
    /// 9:16 still for Reels / TikTok / Shorts covers
    Cover(CoverArgs),
    /// Fade video and audio in and/or out
    Fade(FadeArgs),
    /// On-screen hook / title card for the first seconds
    Title(TitleArgs),
    /// Repeat the clip (Shorts loop / replay length)
    Loop(LoopArgs),
    /// Reduce handheld shake (deshake)
    Stabilize(StabilizeArgs),
    /// Play the clip backwards
    Reverse(ReverseArgs),
    /// Contrast / saturation / brightness pop (Reels look)
    Grade(GradeArgs),
    /// Punch-in / Ken Burns-style center zoom
    Zoom(ZoomArgs),
    /// Speed audio up/down without changing pitch
    Tempo(TempoArgs),
    Silence(SilenceArgs),
    Vocal(VocalArgs),
    Remux(Box<RemuxArgs>),
    Meme(MemeArgs),
    Voice(VoiceArgs),
    Deinterlace(DeinterlaceArgs),
    Crossfade(CrossfadeArgs),
    Strip(StripArgs),
    Frames(FramesArgs),
    Invert(InvertArgs),
    Countdown(CountdownArgs),
    Mix(MixArgs),
    Timer(TimerArgs),
    Mute(MuteArgs),
    Hls(HlsArgs),
    /// Push the clip live to an RTMP/SRT-style endpoint (rtmp://, tcp://)
    Live(LiveArgs),
    /// DASH packaging: manifest.mpd + .m4s segments (jwplayer/Shaka embeds)
    Dash(DashArgs),
    Qa(QaArgs),
    Conform(ConformArgs),
    Sync(SyncArgs),
    /// Rolling end credits (text scrolls bottom to top)
    Scroll(ScrollArgs),
    /// Splice a whole clip into the middle of a video
    Insert(InsertArgs),
    /// Two-camera angle switching across an aligned pair
    Multicam(MulticamArgs),
    Art(ArtArgs),
    /// Remove dust specks / hot pixels (morphology, no blur of the rest)
    Dedust(DedustArgs),
    /// Remove green/blue screen spill from a keyed edge (no keying — just despill)
    Despill(DespillArgs),
    /// HDR → SDR tone mapping (zscale linear + tonemap + back to bt709)
    Tonemap(TonemapArgs),
    /// Telecine — pull 24p film content up to interlaced NTSC fields
    Telecine(TelecineArgs),
    /// Remove pullup judder from frame-rate-converted footage
    Dejudder(DejudderArgs),
    /// Motion-compensated frame interpolation — 60fps upres or smooth slow-mo
    Interp(InterpArgs),
    /// Convert between color matrices (fix SD-601 footage gone green in a 709
    /// timeline; bt2020 deliveries)
    Matrix(MatrixArgs),
    /// Clamp luma to broadcast-safe levels (limiter 16-235)
    Legalize(LegalizeArgs),
    /// Photoshop-style levels (colorlevels in/out black/white points)
    Levels(LevelsArgs),
    /// Chromatic aberration fringe (rgbashift) — cheap-lens / glitch edge look
    Aberrate(AberrateArgs),
    /// Straight ↔ premultiplied alpha conversion for graphics handoffs
    Premult(PremultArgs),
    /// Stretch edge pixels to fill border strips (chroma-key rims, leftover letterbox)
    Extend(ExtendArgs),
    /// Even out voice dynamic range (compressor)
    Leveler(LevelerArgs),
    /// Noise gate — silence below a threshold
    Gate(GateArgs),
    /// Render the audio waveform to a PNG
    Waveform(WaveformArgs),
    /// Render the audio spectrogram to a PNG
    Spectrogram(SpectrogramArgs),
    /// Live EBU R128 loudness meter video (QC: watch I/TP/LRA bars)
    Meter(MeterArgs),
    /// Remove mains hum (50/60 Hz) and harmonics
    Dehum(DehumArgs),
    /// Spatial video denoise for grainy low-light footage
    Vdenoise(VdenoiseArgs),
    /// Crop a region or reframe to an aspect
    Crop(CropArgs),
    /// Sharpen after social re-encode
    Sharpen(SharpenArgs),
    /// Darken the corners (Reels vignette)
    Vignette(VignetteArgs),
    /// Strip color (black and white)
    #[command(name = "bw")]
    Bw(BwArgs),
    /// Raise or lower gain in dB (not LUFS)
    Volume(VolumeArgs),
    /// Gaussian blur the picture
    Blur(BlurArgs),
    /// Motion trails behind moving subjects (dance/skate/echo smears)
    Trail(TrailArgs),
    /// Datamosh-style RGB-shift glitch look
    Glitch(GlitchArgs),
    /// SMPTE bars test card (+1kHz tone)
    Bars(BarsArgs),
    /// QC scope overlay (vectorscope / waveform) in a corner
    Scope(ScopeArgs),
    /// Anamorphic restore: stretch one axis by the lens factor
    Desqueeze(DesqueezeArgs),
    /// Psychedelic partial invert above a luma threshold
    Solarize(SolarizeArgs),
    /// Breathing zoom bounce (music video)
    Pulse(PulseArgs),
    /// Timelapse flicker removal (temporal luma smoothing)
    Deflicker(DeflickerArgs),
    /// Relief emboss (convolution kernel)
    Emboss(EmbossArgs),
    /// Tilt-shift miniature: blur top/bottom strips
    Tilt(TiltArgs),
    /// Handheld drift shake (slow sine crop wander)
    Sway(SwayArgs),
    /// Rack-focus breathing blur
    Rack(RackArgs),
    /// Ink detected edges black over the footage
    Outline(OutlineArgs),
    /// Night-vision look: green tint + grain + vignette
    Night(NightArgs),
    /// Falling snow overlay
    Snow(SnowArgs),
    /// Report dominant colors at a timestamp (JSON only)
    Pick(PickArgs),
    /// Visual diff between two clips
    Diff(DiffArgs),
    /// Keep one color, desaturate the rest
    Selective(SelectiveArgs),
    /// Motion magnification: turn subtle frame-to-frame change visible
    Amplify(AmplifyArgs),
    /// Impact punch: white flash + decaying shake at a moment
    Impact(ImpactArgs),
    /// Watery horizontal wave distortion
    Wave(WaveArgs),
    /// Pendulum sway: frame rotates by a slow sine
    Spin(SpinArgs),
    /// Spotlight circle: dim everything outside a hard-edged disc
    Iris(IrisArgs),
    /// Radial zoom smear: blurred blown-up copy behind the sharp frame
    Burst(BurstArgs),
    /// Sub-bass drop at a moment (55Hz thump with decay)
    Thump(ThumpArgs),
    /// Rising tonal chirp that lands on --at
    Riser(RiserArgs),
    /// Airy noise swell that lands on --at
    Whoosh(WhooshArgs),
    /// De-ess voice: tame 4-8kHz sibilance band
    Deesser(DeesserArgs),
    /// Repair clipped (blown-out) audio — interpolates flattened peaks
    Declip(DeclipArgs),
    /// Smooth gradient banding (skies, backdrops) windowed
    Deband(DebandArgs),
    /// Drop near-duplicate frames (screen recordings, slide decks)
    Dedup(DedupArgs),
    /// Swap bad/glitched frames for a frame from a reference take
    Repair(RepairArgs),
    /// Auto-contrast for flat/washed footage (histeq)
    Equalize(EqualizeArgs),
    /// QC scan: report black/frozen stretches, writes no media
    Scan(ScanArgs),
    /// Edge-preserving beauty/skin blur (smartblur)
    Smooth(SmoothArgs),
    /// Up-res footage: zscale spline36 + light unsharp (--factor 2 doubles dims)
    Upscale(UpscaleArgs),
    /// Comic look: posterized base + ink outlines
    Cartoon(CartoonArgs),
    /// Thermal / false-color luma map
    Heat(HeatArgs),
    /// 2x2 mirrored mandala from the top-left quadrant
    Kaleido(KaleidoArgs),
    /// Music-video flash cuts: periodic opaque color flashes
    Strobe(StrobeArgs),
    /// Neon edge-detect outline look (wires | colormix)
    Edge(EdgeArgs),
    /// Lens distortion: fisheye look or action-cam defish
    Lens(LensArgs),
    /// Deskew: stretch a filmed screen/whiteboard quad onto the frame
    Perspective(PerspectiveArgs),
    /// Generative animated background: mandelbrot zoom | gradients | life
    Gen(GenArgs),
    /// Slant the picture like italic text (shear transform)
    Shear(ShearArgs),
    /// Reframe 360 equirect footage to a flat viewport (yaw/pitch/fov)
    V360(V360Args),
    /// Auto white balance: remove a color cast (indoor tungsten, mixed light)
    Wb(WbArgs),
    /// Remove DCT block edges from heavily compressed sources
    Deblock(DeblockArgs),
    /// Shift chroma planes by pixels — fix tape/capture chroma misregistration
    Chromashift(ChromashiftArgs),
    /// Temporal median: remove anything present <half the window (moving
    /// people/cars on tripod shots, rain streaks)
    Tmedian(TmedianArgs),
    /// Median-stack 3+ locked-off videos of the same scene — removes objects
    /// present in <half the inputs (tourists, noise)
    Stack(StackArgs),
    /// Convert stereoscopic 3D packed formats — SBS to anaglyph for preview,
    /// anaglyph to interleaved for 3D displays
    Stereo(StereoArgs),
    /// Play an image as audio — spectrumsynth scans the picture like a spectrogram
    Sonify(SonifyArgs),
    /// Mirror half the frame across the center axis (dance/symmetry look)
    Mirror(MirrorArgs),
    /// Chunky retro pixelation over the whole frame
    Pix(PixArgs),
    /// Flip the frame horizontally or vertically (unmirror selfie footage)
    Flip(FlipArgs),
    /// Pop-art posterization (quantize to N palette colors)
    Poster(PosterArgs),
    /// Two-color duotone map (shadows -> highlight ramp)
    Duotone(DuotoneArgs),
    /// Dreamy bloom: blurred copy screen-blended back (highlights bleed)
    Glow(GlowArgs),
    /// Retro tape look: noise + chroma shift + scanlines
    Vhs(VhsArgs),
    /// Shutter smear: temporal frame average
    #[command(name = "motionblur")]
    MotionBlur(MotionBlurArgs),
    /// Warp the picture by a second clip's displacement map (heat ripple, liquid glitch)
    Displace(DisplaceArgs),
    /// Apply an EQ and render its response curve as the video (mix QC cards)
    #[command(name = "eqviz")]
    Eqviz(EqvizArgs),
    /// Run one verb on every media file in a directory
    Batch(BatchArgs),
    /// Run a structured filter graph from JSON
    Graph {
        /// Graph plan JSON file
        plan: PathBuf,
    },
    /// Run a multi-step edit plan (the agent's scheme)
    Pipeline {
        /// Pipeline JSON (goal + steps)
        plan: PathBuf,
    },
    /// Guarded raw ffmpeg (verb and graph first; --because required)
    Ffmpeg {
        /// Why no verb or graph field covers this (one line)
        #[arg(long, required = true)]
        because: String,
        /// Native ffmpeg arguments; last path is the output
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Install SKILL.md into detected agent skill directories
    #[command(name = "install-skill")]
    InstallSkill,
    /// Binary, embedded skill, and installed skill-copy versions
    Version {
        /// Exit failed if an installed skill copy does not match this binary
        #[arg(long)]
        check: bool,
    },
}

#[derive(clap::Args, Debug)]
pub struct LookArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: Option<PathBuf>,
    /// Tile grid, e.g. 3x2
    #[arg(long, default_value = "3x2")]
    pub tiles: String,
    /// Frame at timestamp; repeat for a strip (overlay/caption checkpoints)
    #[arg(long, action = clap::ArgAction::Append)]
    pub at: Vec<String>,
}

#[derive(clap::Args, Debug)]
pub struct CutArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    #[arg(long)]
    pub start: Option<String>,
    #[arg(long)]
    pub end: Option<String>,
    #[arg(long)]
    pub duration: Option<String>,
    /// Re-encode for frame-exact cuts
    #[arg(long)]
    pub accurate: bool,
    /// Fade in/out N seconds at the trimmed edges (forces re-encode)
    #[arg(long)]
    pub fade: Option<f64>,
    /// Keep several ranges joined into one file ("10-20,40-50", seconds or h:mm:ss)
    #[arg(long)]
    pub ranges: Option<String>,
    /// Comma list of ranges to DROP, keeps the rest joined: "a-b,c-d"
    #[arg(long)]
    pub drop: Option<String>,
    /// Auto-detect and excise black stretches (blackdetect ≥0.3s, 98%
    /// black) — dead-air trim for talking-head/event footage, the video
    /// twin of cutsil
    #[arg(long)]
    pub black: bool,
}

#[derive(clap::Args, Debug)]
pub struct ConcatArgs {
    /// Input clips, in order
    pub inputs: Vec<PathBuf>,
    /// Read clip paths from a manifest file — one path per line, '#'
    /// comments allowed, relative paths resolve against the list's
    /// directory; replaces the positional clips for script-generated cuts
    #[arg(long)]
    pub list: Option<PathBuf>,
    #[arg(short, long)]
    pub output: PathBuf,
    /// xfade between clips: fade | wipe* | slide* | dissolve | radial | circleopen —
    /// comma list picks a different transition per joint (one per junction)
    #[arg(long)]
    pub transition: Option<String>,
    /// Transition duration in seconds
    #[arg(long, default_value_t = 0.5)]
    pub duration: f64,
    /// Loudnorm every input to this I target before joining (mixed-source
    /// loudness, e.g. -14) — one-pass per clip
    #[arg(long, allow_hyphen_values = true)]
    pub level: Option<f64>,
    /// Insert N seconds of black+silence between clips (beat gap between
    /// montage sections) — exclusive with --transition
    #[arg(long)]
    pub gap: Option<f64>,
    /// Fade audio out/in at each joint by N seconds (boundary fades — keeps
    /// every clip's duration and sync; no overlap drift)
    #[arg(long)]
    pub audio_fade: Option<f64>,
    /// Chapter each input clip at its join point (title = filename stem) —
    /// audiobook/podcast multi-file → chaptered single deliverable
    #[arg(long)]
    pub chapters: bool,
    /// Force the lossless stream-copy join — fail when inputs differ in
    /// codec/size/rate instead of silently re-encoding (verify an assembly
    /// stayed lossless; conflicts with --transition/--level/--gap/--audio-fade)
    #[arg(long)]
    pub copy: bool,
    /// Repeat the joined sequence N times — loop-intended assemblies
    /// (the whole A+B+C chain repeats N times, transitions included;
    /// extras: repeated)
    #[arg(long)]
    pub repeat: Option<u32>,
}

#[derive(clap::Args, Debug)]
pub struct SplitArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Max seconds per part (story/WhatsApp chunks)
    #[arg(long)]
    pub every: Option<f64>,
    /// Cut at these timestamps instead (comma list, e.g. --at 30,90,150)
    #[arg(long, value_delimiter = ',')]
    pub at: Vec<String>,
    /// Auto-detect scene cuts at this threshold (0.1–0.9; 0.3 typical)
    #[arg(long)]
    pub scenes: Option<f64>,
    /// Aim each part under this size (e.g. 9MB for Discord) — computes --every
    #[arg(long)]
    pub size: Option<String>,
    /// Split into N equal-length parts
    #[arg(long)]
    pub parts: Option<u32>,
    /// Cut at silence midpoints under this dB threshold (e.g. --silence=-35)
    #[arg(long, allow_hyphen_values = true)]
    pub silence: Option<f64>,
    /// With --silence: minimum gap length in seconds (default 0.4)
    #[arg(long)]
    pub min_silence: Option<f64>,
    /// Also write a per-part .srt next to each split file (cues re-timed)
    #[arg(long)]
    pub subs: Option<PathBuf>,
    /// Cut at the input's embedded chapter marks (lectures, courses, books)
    #[arg(long)]
    pub chapters: bool,
    /// Fade audio+video N seconds around every boundary (soft story chunks)
    #[arg(long)]
    pub fade: Option<f64>,
    /// Auto-split at black stretches (blackdetect ≥0.3s) — every non-black
    /// keep segment becomes its own part file: dead-air chapterization
    /// for event/talking-head footage
    #[arg(long)]
    pub black: bool,
    /// Stream-copy the parts through the segment muxer — no re-encode, so
    /// splitting a long 4K recording is instant, but each boundary snaps
    /// forward to the next keyframe (not frame-exact). No --fade/--black.
    #[arg(long)]
    pub copy: bool,
}

#[derive(clap::Args, Debug)]
pub struct FitArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Target aspect, e.g. 9:16, 1:1, 16:9
    #[arg(long)]
    pub aspect: Option<String>,
    #[arg(long)]
    pub width: Option<u32>,
    #[arg(long)]
    pub height: Option<u32>,
    #[arg(long, value_enum, default_value_t = FitMode::Pad)]
    pub fit: FitMode,
    /// Pad bar color (RRGGBB hex, default black)
    #[arg(long)]
    pub color: Option<String>,
    /// Blur sigma for --fit blur (default 30; lower keeps edges readable)
    #[arg(long)]
    pub strength: Option<f64>,
    /// Where the picture sits in the padded frame: center (default), top,
    /// bottom, left, right, or a corner (top-left…bottom-right)
    #[arg(long)]
    pub position: Option<String>,
    #[arg(long)]
    pub rotate: Option<u32>,
    #[arg(long, value_enum)]
    pub flip: Option<FlipMode>,
    #[arg(long)]
    pub fps: Option<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum FitMode {
    Pad,
    Crop,
    /// Letter/pillarbox filled with a blurred copy of the frame (repurpose look)
    Blur,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum EqPreset {
    /// gentle low end + presence for talking-head speech
    Voice,
    /// rolled lows + strong 3 kHz presence for spoken-word podcasts
    Podcast,
    /// treble-forward sparkle
    Bright,
    /// bass-heavy boost
    Bass,
    /// warm/rounded: low boost + slight presence cut (thins harshness)
    Warm,
    /// top-end air: strong treble shelf, small presence lift
    Air,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum FlipMode {
    H,
    V,
}

#[derive(clap::Args, Debug)]
pub struct ExtractArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Timestamp for a still frame — `end` = last frame / last --dur seconds; comma list = one still per time
    #[arg(long)]
    pub at: Option<String>,
    /// Scale the still to this width (height follows aspect)
    #[arg(long)]
    pub width: Option<u32>,
    /// Animated GIF clip instead of a still (2-pass palette)
    #[arg(long)]
    pub gif: bool,
    /// Palindrome loop: forward + reversed (needs --gif)
    #[arg(long)]
    pub bounce: bool,
    /// GIF repeat count: -1 = play once, 0 or unset = loop forever (needs --gif)
    #[arg(long = "loop", allow_negative_numbers = true)]
    pub loop_count: Option<i64>,
    /// GIF clip length in seconds (needs --gif)
    #[arg(long)]
    pub dur: Option<f64>,
    /// GIF frames per second (default 10)
    #[arg(long)]
    pub fps: Option<u32>,
    /// GIF palette size 2–256 (needs --gif; smaller = tinier file, banding)
    #[arg(long)]
    pub colors: Option<u32>,
    /// Extract the alpha channel as a grayscale image (matte export/QC —
    /// prores 4444, transparent webm, chroma-keyed deliverables)
    #[arg(long)]
    pub alpha: bool,
    /// Extract the audio track losslessly (stream copy — pull the music/
    /// dialog track without re-encoding; -o extension picks the container)
    #[arg(long)]
    pub audio: bool,
    /// Extract an embedded subtitle track to a text file (-o .srt/.ass/.vtt
    /// picks the caption container; pull captions out of a finished export
    /// for re-timing or re-burning)
    #[arg(long)]
    pub subs: bool,
    /// With --audio or --subs: which track to pull, 0-based (default first —
    /// pick the commentary/stem or a language out of a multi-track file)
    #[arg(long)]
    pub track: Option<u32>,
    /// With --audio or --subs: pull EVERY track — writes stem_aN/stem_sN
    /// files next to -o (multitrack dubs/captions out in one pass)
    #[arg(long)]
    pub all: bool,
    /// With --audio or --subs: pull the track tagged with this language
    /// (eng/jpn/…) — pick the dub by name instead of index; fails when
    /// no track matches
    #[arg(long)]
    pub lang: Option<String>,
    /// With --audio: rip only from this second onward (segment of the
    /// track — pull the hook out of a song; stream copy)
    #[arg(long)]
    pub from: Option<f64>,
    /// With --audio: stop the rip at this second (stream copy)
    #[arg(long)]
    pub to: Option<f64>,
    /// Keep alpha in the GIF (needs --gif + an alpha-channel input like
    /// prores 4444/qtrle/webm — Discord/Telegram sticker exports)
    #[arg(long)]
    pub transparent: bool,
    /// Pull the Nth embedded chapter (1-based — audiobook/lecture segment
    /// export: `chapter --list` shows the numbering)
    #[arg(long)]
    pub chapter: Option<u32>,
    /// Rip the Nth attachment stream (0-based, per-type index) to a file
    /// — pull fonts/files embedded by `remux --attach` back out (mkv/webm)
    #[arg(long)]
    pub attachment: Option<u32>,
    /// Pull embedded cover art (attached_pic stream) back out as an image
    /// file — verify or re-deliver feed art; extension should match the
    /// embedded codec (mjpeg art → .jpg, png → .png)
    #[arg(long)]
    pub cover: bool,
    /// Dump every keyframe (I-frame) as an image — -o needs a %03d-style
    /// template (GOP-boundary stills: keyframe-interval QC, timelapse
    /// source, scene-jump scouting without decoding the whole file)
    #[arg(long)]
    pub keyframes: bool,
    /// Animated WebP clip instead of a still (libwebp — smaller than GIF,
    /// keeps alpha natively; --bounce works too)
    #[arg(long)]
    pub webp: bool,
    /// Lossless WebP encode (needs --webp; bigger file, pixel-exact)
    #[arg(long)]
    pub lossless: bool,
}

#[derive(clap::Args, Debug)]
pub struct OverlayArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    #[arg(long)]
    pub image: Option<PathBuf>,
    #[arg(long)]
    pub video: Option<PathBuf>,
    #[arg(long, default_value = "top-right")]
    pub position: String,
    /// Overlay width in pixels
    #[arg(long)]
    pub scale: Option<u32>,
    #[arg(long, default_value_t = 20)]
    pub margin: i32,
    #[arg(long)]
    pub x: Option<String>,
    #[arg(long)]
    pub y: Option<String>,
    /// Tile the overlay N times across the frame (draft watermark); 0 = off
    #[arg(long, default_value_t = 0)]
    pub tile: u32,
    /// Ring around the overlay picture in px (PiP readability)
    #[arg(long)]
    pub border: Option<u32>,
    /// Border color for --border (default white)
    #[arg(long)]
    pub border_color: Option<String>,
    /// Blend the overlay as a full-frame composite: screen|addition|multiply|
    /// lighten|darken|overlay|difference (light leaks, particles, LUTs-textures)
    #[arg(long)]
    pub mode: Option<String>,
    /// Blend strength 0..=1 for --mode (default 1.0)
    #[arg(long, default_value_t = 1.0)]
    pub opacity: f64,
    /// Show the overlay only from this time — comma list for several spots (needs --dur)
    #[arg(long)]
    pub at: Option<String>,
    /// Fade the overlay in/out over N seconds (0 = hard cut)
    #[arg(long, default_value_t = 0.0)]
    pub fade: f64,
    /// ..until this many seconds after --at (default: to the end)
    #[arg(long)]
    pub dur: Option<f64>,
    /// Rotate the overlay N degrees (diagonal watermarks)
    #[arg(long)]
    pub angle: Option<f64>,
    /// Loop the overlay video when it is shorter than the input
    #[arg(long = "loop")]
    pub loop_track: bool,
}

#[derive(clap::Args, Debug)]
pub struct BrollArgs {
    /// A-roll (talking head)
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// B-roll clip to cut away to
    #[arg(long)]
    pub insert: PathBuf,
    /// Start of the cutaway on the A-roll (or `end` for the tail)
    #[arg(long)]
    pub at: String,
    /// How long the cutaway lasts
    #[arg(long)]
    pub duration: f64,
    #[arg(long, value_enum, default_value_t = FitMode::Crop)]
    pub fit: FitMode,
    /// Treat --insert as a still image (looped over the cutaway window)
    #[arg(long)]
    pub still: bool,
    /// Animate a --still insert (kenburns = slow push over the window)
    #[arg(long, value_enum)]
    pub motion: Option<SlideMotion>,
    /// Fade the cutaway in/out over N seconds (0 = hard cut)
    #[arg(long, default_value_t = 0.0)]
    pub fade: f64,
    /// Also mix in the insert's audio during its window (b-roll sound)
    #[arg(long)]
    pub audio: bool,
    /// Linear gain on the insert audio 0..=4 (with --audio; default 1.0)
    #[arg(long)]
    pub volume: Option<f64>,
    /// Ring the PiP insert with an N-px border (with --position)
    #[arg(long)]
    pub border: Option<u32>,
    /// Border color for --border (default white; name or RRGGBB)
    #[arg(long)]
    pub border_color: Option<String>,
    /// PiP mode: place the insert in a corner/edge instead of full-screen
    /// (top-left, top, top-right, left, center, right, bottom-left, bottom, bottom-right)
    #[arg(long)]
    pub position: Option<String>,
    /// PiP width as a fraction of the frame (default 0.30, with --position)
    #[arg(long)]
    pub scale: Option<f64>,
    /// PiP edge margin in px (default 24, with --position)
    #[arg(long)]
    pub margin: Option<i32>,
    /// Loop a video --insert shorter than the cutaway window
    /// (default freezes on its last frame)
    #[arg(long = "loop")]
    pub loop_insert: bool,
    /// Blend the insert at this % opacity (ghost b-roll)
    #[arg(long)]
    pub opacity: Option<f64>,
}

#[derive(clap::Args, Debug)]
pub struct CaptionArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    #[arg(long)]
    pub srt: PathBuf,
    #[arg(long, value_enum, default_value_t = CaptionMode::Burn)]
    pub mode: CaptionMode,
    /// Burn-in placement: `social` clears bottom 20% / top 15% (TikTok/Reels chrome); `off` is the old 15% bottom margin
    #[arg(long, value_enum, default_value_t = CaptionSafe::Social)]
    pub safe: CaptionSafe,
    /// Split each cue into ≤N-word chunks spread evenly over its time (burn only)
    #[arg(long, value_name = "N")]
    pub chunk: Option<u32>,
    #[arg(long)]
    pub font: Option<String>,
    /// Shift every cue by SEC (negative pulls captions earlier)
    #[arg(long, allow_hyphen_values = true, default_value_t = 0.0)]
    pub shift: f64,
    /// Keep only cues overlapping [FROM,TO) — caption just a slice
    #[arg(long)]
    pub from: Option<String>,
    /// Window end (h:mm:ss or seconds; needs --from)
    #[arg(long)]
    pub to: Option<String>,
    /// Karaoke-style word-by-word reveal inside each cue (burn only)
    #[arg(long)]
    pub karaoke: bool,
    /// Fade each caption in/out over N seconds (0 = hard cut)
    #[arg(long, default_value_t = 0.0)]
    pub fade: f64,
    /// Filled card behind each caption: RRGGBB hex or color name
    #[arg(long)]
    pub box_color: Option<String>,
    /// Draw captions at this % opacity (0-100 — ghost/watermark captions)
    #[arg(long)]
    pub opacity: Option<f64>,
    /// Burn-in placement: bottom (default) or top of frame
    #[arg(long, value_enum, default_value_t = CaptionPosition::Bottom)]
    pub position: CaptionPosition,
    /// Text color as RRGGBB hex (default ffffff)
    #[arg(long)]
    pub color: Option<String>,
    /// Text size multiplier (default 1.0)
    #[arg(long, default_value_t = 1.0)]
    pub size: f64,
    /// Stroke color around each glyph as RRGGBB (burn only)
    #[arg(long)]
    pub outline: Option<String>,
    /// Per-line alignment inside each caption card (burn only)
    #[arg(long, value_enum)]
    pub align: Option<crate::raster::TextAlign>,
    /// Word-wrap each cue line at N columns (≥4, burn only)
    #[arg(long)]
    pub wrap: Option<u32>,
    /// Pixels from the chosen edge (overrides the percent offset)
    #[arg(long)]
    pub margin: Option<u32>,
    /// Karaoke sung-word color as RRGGBB (needs --karaoke)
    #[arg(long)]
    pub highlight: Option<String>,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum CaptionMode {
    Burn,
    Mux,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum CaptionSafe {
    Social,
    Off,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum CaptionPosition {
    Bottom,
    Top,
}

#[derive(clap::Args, Debug)]
pub struct DenoiseArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Denoise strength 0–1 (0.5 ≈ afftdn nr=12)
    #[arg(long, default_value_t = 0.5)]
    pub strength: f64,
    /// High-pass frequency in Hz for rumble (0 = off)
    #[arg(long, default_value_t = 90.0)]
    pub highpass: f64,
    /// Also run hqdn3d on the picture (grainy footage)
    #[arg(long)]
    pub video: bool,
    /// Engine: auto (wavelet if built, else fft), wavel (afwtdn), fftdn
    /// (afftdn spectral). Auto picks the strongest available.
    #[arg(long, value_enum, default_value_t = DenoiseEngine::Auto)]
    pub engine: DenoiseEngine,
    /// Noise-reference recording (room tone mic / second recorder): anlms
    /// adaptively cancels what matches the reference — real denoise for
    /// lavalier+room-mic rigs. --strength tunes the filter order.
    #[arg(long)]
    pub ref_: Option<PathBuf>,
    /// Denoise only inside this window — comma list for several (needs --dur)
    #[arg(long)]
    pub at: Option<String>,
    /// Window length in seconds (default: to the end)
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(Clone, Copy, Debug, Default, clap::ValueEnum)]
pub enum DenoiseEngine {
    /// afwtdn wavelet when built (ffmpeg ≥5.1), else afftdn nf=-20
    #[default]
    Auto,
    /// afwtdn wavelet — strongest broadband cut (errors when not built)
    Wavel,
    /// afftdn spectral — works on every ffmpeg build
    Fftdn,
}

#[derive(clap::Args, Debug)]
pub struct CompressArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Target size — number + KB/MB/GB, or a named app cap:
    /// discord 8MB, nitro 500MB, whatsapp 16MB, gmail/email 25MB,
    /// messenger 25MB, wechat 100MB
    #[arg(long, required_unless_present_any = ["target", "crf"])]
    pub size: Option<String>,
    /// Size preset by platform: discord(8MB) whatsapp(16MB) gmail(25MB)
    #[arg(long, value_enum, required_unless_present_any = ["size", "crf"])]
    pub target: Option<CompressTarget>,
    /// Audio bitrate budget in kbps
    #[arg(long, default_value_t = 96.0)]
    pub audio_kbps: f64,
    /// Quality mode instead of a size target: single-pass libx264 crf (0–51)
    #[arg(long)]
    pub crf: Option<u32>,
    /// Downscale to this height first (frees bitrate at small sizes)
    #[arg(long)]
    pub res: Option<u32>,
    /// Cap the frame rate (a 60fps capture at 30 frees motion bitrate
    /// under a messaging cap)
    #[arg(long)]
    pub fps: Option<u32>,
    /// Resample audio to this rate (voice notes fit a tighter budget at
    /// 22050/16000 — pair with --size on speech content)
    #[arg(long)]
    pub ar: Option<u32>,
    /// Audio channel count — 1 mono halves the speech bitrate share
    /// (podcast/voice notes under a messaging cap)
    #[arg(long)]
    pub channels: Option<u8>,
    /// Drop the audio track entirely (-an) — every bit of the size
    /// budget goes to video (silent social previews; conflicts with
    /// --ar/--channels)
    #[arg(long)]
    pub no_audio: bool,
}

#[derive(clap::Args, Debug)]
pub struct AudiogramArgs {
    /// Audio file (podcast clip, wav/mp3/m4a)
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Cover still behind the waveform; flat colour when omitted
    #[arg(long)]
    pub image: Option<PathBuf>,
    /// Waveform drawing style
    #[arg(long, value_enum, default_value_t = WaveMode::Cline)]
    pub mode: WaveMode,
    /// Waveform colour (ffmpeg name or 0xRRGGBB)
    #[arg(long, default_value = "white")]
    pub color: String,
    /// Wave amplitude scale: lin (default)|log|sqrt|cbrt — log shows quiet detail
    #[arg(long)]
    pub scale: Option<String>,
    /// Draw each channel on its own row (stereo split view)
    #[arg(long)]
    pub split: bool,
    /// Frequency scale for --mode spectrum: lin|log|rlog (music→log)
    #[arg(long)]
    pub fscale: Option<String>,
    /// Output frame rate (default 30; 60 for smooth bars)
    #[arg(long)]
    pub fps: Option<f64>,
    /// Background colour when no --image (name or 0xRRGGBB, default 101418)
    #[arg(long)]
    pub bg: Option<String>,
    /// Canvas WxH (default 1080x1920; use 1920x1080 for YouTube)
    #[arg(long, default_value = "1080x1920")]
    pub size: String,
    /// Show this text near the top (podcast name / episode title)
    #[arg(long)]
    pub text: Option<String>,
    /// Wave band vertical placement: top / center / bottom (default bottom)
    #[arg(long)]
    pub position: Option<String>,
    /// Font file for --text (ttf/otf)
    #[arg(long)]
    pub font: Option<String>,
    /// Moving progress bar along the bottom edge
    #[arg(long)]
    pub progress: bool,
    /// Burn this .srt's cues onto the audiogram (bottom strip)
    #[arg(long)]
    pub subs: Option<PathBuf>,
    /// Start the clip at this time (h:mm:ss or seconds) — the
    /// podcast→clip ask: render just the best segment as an audiogram
    #[arg(long)]
    pub from: Option<String>,
    /// ..and stop at this time (default: end of the audio)
    #[arg(long)]
    pub to: Option<String>,
    /// Clip start point(s) — comma list renders one audiogram per point
    /// as <stem>_N.mp4 (`end`/`end-N` anchors work)
    #[arg(long)]
    pub at: Option<String>,
    /// Seconds each --at/--from clip runs (default: to the end)
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum WaveMode {
    Point,
    Line,
    P2p,
    #[default]
    Cline,
    /// Frequency bars (showfreqs) — spectrum audiogram look
    Spectrum,
    /// Lissajous vectorscope (avectorscope) — trippy stereo scope
    Scope,
    /// Constant-Q music spectrum (showcqt) — piano-roll look
    Cqt,
    /// Scrolling spectrogram (showspectrum) — colour time/frequency roll
    Spectro,
    /// Phase meter (aphasemeter) — stereo scope: razor-thin line = mono,
    /// wide cloud = decorrelated. Mono-compat QC + looks
    Phase,
    /// Spatial spectrogram (showspatial) — stereo field mapped over time
    Spatial,
    /// Per-channel VU bars (showvolume) — broadcast meter-bridge look
    Volume,
    /// Bit-pattern scope (abitscope) — bit-depth/gauge visualiser
    Bitscope,
    /// Filtergraph stats monitor (agraphmonitor) — audio pipeline health viz
    Monitor,
    /// Sample-level histogram (ahistogram) — amplitude distribution over
    /// time: clipping/headroom QC as a video
    Hist,
}

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum SharpenEngine {
    #[default]
    Unsharp,
    /// Contrast-adaptive sharpening — crisper edges, no white halos
    Cas,
    /// Unsharp clamped to a blurred base (maskedclamp) — strongest, zero halo
    Halo,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum TextCase {
    Upper,
    Lower,
    Title,
}

#[derive(clap::Args, Debug)]
pub struct LoudnormArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: Option<PathBuf>,
    /// Integrated loudness target (LUFS, default -14; --target sets platform values)
    #[arg(short = 'I', long, allow_hyphen_values = true)]
    pub i: Option<f64>,
    /// True peak (dBTP, default -1.5)
    #[arg(long, allow_hyphen_values = true)]
    pub tp: Option<f64>,
    #[arg(long, allow_hyphen_values = true)]
    pub lra: Option<f64>,
    /// Platform preset: spotify|youtube (-14 LUFS), podcast (-16), broadcast (-23)
    #[arg(long, value_enum)]
    pub target: Option<LoudnormTarget>,
    /// Measure-only: report I/TP/LRA in extras without writing a file
    #[arg(long)]
    pub measure: bool,
    /// Fail when the measured integrated loudness exceeds N LUFS (needs --measure)
    #[arg(long, allow_hyphen_values = true)]
    pub gate: Option<f64>,
    /// Dynamic normalization (per-frame gain) instead of the default linear offset
    #[arg(long)]
    pub dynamic: bool,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
pub enum LoudnormTarget {
    /// -14 LUFS / -1.5 dBTP
    Spotify,
    /// -14 LUFS / -1.5 dBTP
    Youtube,
    /// -16 LUFS / -1.5 dBTP
    Podcast,
    /// -23 LUFS / -2.0 dBTP, LRA 7 (EBU R128)
    Broadcast,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
pub enum CompressTarget {
    /// 8 MB — Discord free-tier upload limit
    Discord,
    /// 16 MB — WhatsApp video limit
    Whatsapp,
    /// 25 MB — Gmail attachment limit
    Gmail,
}

#[derive(clap::Args, Debug)]
pub struct TranscodeArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    #[arg(long, value_enum)]
    pub preset: Option<TranscodePreset>,
    #[arg(long)]
    pub crf: Option<u8>,
    /// Frames per second (GIF default 10; h264/webm retimes to N fps)
    #[arg(long)]
    pub fps: Option<u32>,
    /// GIF-only: output width (default 480)
    #[arg(long)]
    pub width: Option<u32>,
    /// Keep the original audio bitstream (no re-encode) while transcoding video
    #[arg(long)]
    pub copy_audio: bool,
    /// Keep the original video bitstream (no re-encode) while transcoding
    /// audio — fix bad audio / repack without touching the picture.
    /// Video filter flags (--fps/--range/--interlaced/--field-order/--alpha)
    /// can't apply to a copied stream
    #[arg(long)]
    pub copy_video: bool,
    /// GIF-only: palette size 2–256 (smaller = tinier file, banding)
    #[arg(long)]
    pub colors: Option<u32>,
    /// Keep the alpha channel (webm/prores only — overlays & lower thirds
    /// exported for editors); h264/hevc can't carry transparency
    #[arg(long)]
    pub alpha: bool,
    /// Peak video bitrate cap like `8M`/`3500k` — sets -maxrate R -bufsize 2R
    #[arg(long)]
    pub vbitrate: Option<String>,
    /// Audio bitrate like `64k`/`128k` (voice posts → 64k frees video bitrate)
    #[arg(long)]
    pub abitrate: Option<String>,
    /// Tag output as limited|full range (broadcast masters want limited)
    #[arg(long, value_enum)]
    pub range: Option<TranscodeRange>,
    /// Mark the output interlaced (il field interleave + tff flag) —
    /// broadcast/interlaced-masters delivery
    #[arg(long)]
    pub interlaced: bool,
    /// Interlace engine with --interlaced: il = same-frame field
    /// interleave (progressive picture, tagged for air); weave = real
    /// temporal weave — pairs consecutive frames into fields, so feed
    /// double-rate progressive (50p → 25i is the true broadcast cut)
    #[arg(long, value_enum)]
    pub interlace_mode: Option<InterlaceKind>,
    /// Relabel the field-order flag WITHOUT re-weaving frames (fixes
    /// masters tagged with the wrong parity — a metadata-only repair)
    #[arg(long, value_enum)]
    pub field_order: Option<FieldOrder>,
    /// Resample audio to this Hz on re-encode (48000 broadcast, 44100
    /// podcast/CD)
    #[arg(long)]
    pub ar: Option<u32>,
    /// Force channel count on re-encode (1 = mono podcast voice, 2 = stereo)
    #[arg(long)]
    pub channels: Option<u8>,
    /// Keyframe interval in frames (-g N) — ingest specs cap GOP size
    /// (YouTube wants <= 2s), shorter GOPs seek faster (conflicts with
    /// --copy-video and the audio presets)
    #[arg(long)]
    pub gop: Option<u32>,
    /// x264 encode profile — device-compat ingest specs (baseline for old
    /// phones/car/kiosk players; h264/proxy encodes only, conflicts with
    /// --copy-video and every non-x264 preset)
    #[arg(long)]
    pub profile: Option<TranscodeProfile>,
    /// x264 encode level (e.g. 4.1, 3.1, 4) — device-compat ingest specs
    /// cap level on top of profile (older decoders refuse High@L5+);
    /// h264/proxy presets only, conflicts with --copy-video and every
    /// non-x264 preset
    #[arg(long)]
    pub level: Option<String>,
    /// Max B-frames on the x264 encode (-bf N) — mobile/baseline ingest
    /// specs cap reorder delay (0 = decode-order = display-order);
    /// h264/proxy presets only
    #[arg(long)]
    pub bf: Option<u32>,
    /// x264 tune (-tune) — content-shaped encoder bias: grain keeps film
    /// grain, fastdecode for slow players, zerolatency for capture-monitor
    /// pipelines; h264/proxy presets only
    #[arg(long)]
    pub tune: Option<TranscodeTune>,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
pub enum TranscodeTune {
    /// Film — default-ish live-action bias
    Film,
    /// Animation — flat-shaded content (deblock-tuned)
    Animation,
    /// Grain — preserves film grain/noise (grain-heavy masters band without it)
    Grain,
    /// Zerolatency — no lookahead/reorder (capture-monitor pipelines)
    Zerolatency,
    /// Fastdecode — simpler entropy coding for weak CPUs
    Fastdecode,
    /// Stillimage — single-frame/still optimize
    Stillimage,
    /// Psnr — objective-metric tune (analysis encodes)
    Psnr,
    /// Ssim — objective-metric tune (analysis encodes)
    Ssim,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
pub enum FieldOrder {
    /// Top field first (broadcast HD default)
    Tff,
    /// Bottom field first (DV/SD legacy)
    Bff,
    /// Progressive — clears a wrong interlace flag
    Prog,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
pub enum InterlaceKind {
    /// il field interleave — progressive picture tagged interlaced
    Il,
    /// weave — two consecutive progressive frames woven into one
    /// interlaced frame (real field motion; halves output fps)
    Weave,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
pub enum TranscodeRange {
    /// tv / MPEG range 16-235 — broadcast + most player-safe
    Limited,
    /// pc / JPEG range 0-255 — computer playback
    Full,
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub enum TranscodeProfile {
    /// Baseline — old phones, car units, kiosk players (no B-frames/CABAC-trellis)
    Baseline,
    /// Main — mid-range device compat
    Main,
    /// High — default x264 profile, best compression
    High,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum TranscodePreset {
    H264,
    /// H.265 ~50% smaller at same quality; hvc1 tag keeps QuickTime happy
    Hevc,
    Webm,
    Gif,
    /// ProRes 422 HQ in .mov — the FCP/Premiere edit delivery format
    Prores,
    /// DNxHR HQ in .mov — the Avid/Resolve edit delivery format
    Dnxhd,
    /// AV1 (svt-av1 on ffmpeg ≥7, libaom on 4.x) — smallest web delivery
    Av1,
    /// Edit proxy — 540p max, veryfast x264, aac 96k (scrub-friendly NLE dailies)
    Proxy,
    /// Audio-only MP3 (podcast/voice delivery; -vn, libmp3lame)
    Mp3,
    /// Audio-only AAC in .m4a (Apple uploads, voice notes)
    Aac,
    /// Audio-only WAV (pcm_s16le lossless — DAW/edit handoff)
    Wav,
    /// Audio-only FLAC (lossless archival, ~50% smaller than wav)
    Flac,
    /// Audio-only Opus (libopus 128k — smallest voice/music delivery)
    Opus,
    /// FFV1 lossless in .mkv — archival/intermediate master (museum-grade
    /// mathematically lossless; audio -> flac)
    Ffv1,
    /// Animated PNG — full-color sticker/reaction loops where gif's pal8
    /// banding shows (.apng target, loops forever, -vn implied)
    Apng,
    /// MPEG-2 video + MP2 audio — DVD/broadcast legacy master (.mpg/.vob;
    /// set-top players, TV ingest, archival interop)
    Mpeg2,
    /// MPEG-1 video + MP2 audio — VCD-era legacy master (.mpg; the oldest
    /// digital video format still in playback circulation)
    Mpeg1,
    /// Xvid/MPEG-4 part 2 in .avi — the legacy-rip master (2000s DivX-era
    /// players, projectors, and set-top boxes that only read .avi)
    Xvid,
    /// WMV2 + WMA in .wmv/.asf — Windows Media-era master (corporate
    /// training archives, old PowerPoint-embedded video, Windows-only gear)
    Wmv,
    /// MS-MPEG4 v2 + MP3 in .avi — the pre-DivX Windows codec
    /// (MP42 tag: Windows ME-era screen captures, ancient players)
    Msmpeg4,
    /// H.263 + AMR-NB in .3gp — the feature-phone master
    /// (MMS-era mobile video, J2ME handsets; audio drops to 8kHz mono)
    Gpp,
    /// FLV1 + MP3 in .flv — the Flash-era web master
    /// (YouTube 2005-era uploads, Flash video archives)
    Flv,
    /// Theora + Vorbis in .ogv — the open-web master
    /// (pre-WebM HTML5 video, Wikipedia/Wikimedia embeds)
    Theora,
    /// Audio-only Ogg Vorbis (.ogg — open-web music/podcast upload,
    /// Bandcamp/Jamendo-era delivery)
    Ogg,
    /// Audio-only ALAC in .m4a/.mp4 (Apple Lossless — lossless music
    /// archive for the Apple ecosystem, no quality loss vs wav)
    Alac,
    /// DV25 in .dv/.avi — the camcorder-tape master (MiniDV/DVCAM
    /// archives, NLE-era broadcast decks; snaps to the DV-legal
    /// NTSC canvas 720x480@30000/1001, PCM 48kHz stereo audio)
    Dv,
    /// Motion JPEG + MP2 in .avi/.mov (NLE-era editing format —
    /// Digital Betacam captures, frame-accurate scrub masters)
    Mjpeg,
    /// AMV in .amv — the Chinese handheld-player master (MP3/MP4
    /// players circa 2006; snaps to the AMV-legal 160x120 canvas,
    /// ADPCM 22050Hz mono audio)
    Amv,
    /// QuickTime Animation RLE in .mov — lossless animation/screencast
    /// master (rgb24, argb with --alpha)
    Qtrle,
    /// Uncompressed 10-bit 4:2:2 broadcast master + PCM in .mov —
    /// v210 (edit-bay/broadcast ingest spec; bitrate flags meaningless)
    V210,
    /// HuffYUV lossless capture intermediate + PCM in .avi/.mkv —
    /// NLE-era lossless edit masters (faster than ffv1)
    Huffyuv,
    /// Ut Video lossless intermediate + PCM in .avi — the
    /// VirtualDub/NLE-era fast lossless codec (CM Ultra specs)
    Utvideo,
    /// FFVHuff lossless intermediate + PCM in .mkv — FFmpeg's own
    /// Huffyuv variant, the matroska-native lossless sibling
    Ffvhuff,
    /// Cinepak + PCM in .mov/.avi — the CD-ROM-era codec (mid-90s
    /// QuickTime/Windows video, Myst-era game archives)
    Cinepak,
    /// Sorenson Video 1 + PCM in .mov — QuickTime 2-4 era web video
    /// (the pre-Flash internet standard)
    Svq1,
    /// Zip Motion Blocks Video + PCM in .avi — DOSBox-era screencast
    /// recordings (game-capture archives, rgb24)
    Zmbv,
    /// Uncompressed 10-bit 4:4:4 + PCM in .mov — finishing-suite
    /// interchange (v410 yuv444p10le; the colour-managed master
    /// colorists/QC pass around)
    V410,
    /// Uncompressed 8-bit 4:4:4:4 + PCM in .mov — always carries alpha
    /// (ayuv yuva444p; motion-graphics interchange when prores4444 is
    /// too compressed and ffv1 is too exotic)
    Ayuv,
    /// RealVideo 1.0 video-only in .rm — the late-90s dial-up streaming
    /// codec (the container has no usable audio encoder, so it's
    /// picture-only)
    Rv10,
    /// RealVideo 2.0 video-only in .rm — the second-gen dial-up
    /// streaming codec
    Rv20,
    /// Uncompressed 10-bit RGB 4:4:4 + PCM in .mov (r210 gbrp10le —
    /// the RGB member of the uncompressed finishing-suite trio)
    R210,
    /// Uncompressed 8-bit 4:4:4 + PCM in .mov (v308 yuv444p — the
    /// 8-bit sibling of v410)
    V308,
    /// Apple Video "Road Pizza" + PCM in .mov (rpza — QuickTime 1.x
    /// codec, the oldest QuickTime video)
    Rpza,
    /// NewTek SpeedHQ + PCM in .mov/.avi (NDI-era NLE intermediate —
    /// TriCaster/NDI capture masters)
    Speedhq,
    /// id RoQ + RoQ DPCM in .roq (Quake III-era game video — picture
    /// dims snap to powers of two, audio forced to 22050Hz)
    Roq,
    /// Snow wavelet + PCM in .mkv (ffmpeg's native experimental codec —
    /// mathematically lossless-quality archival)
    Snow,
    /// Flash Screen Video + MP3 in .flv (screen-recording era codec)
    #[value(name = "flashsv")]
    Flashsv,
    /// Flash Screen Video 2 + MP3 in .flv (the later screen codec)
    #[value(name = "flashsv2")]
    Flashsv2,
    /// Microsoft Video 1 + MP3 in .avi (the oldest Windows video codec,
    /// 8-bit palettized)
    #[value(name = "msvideo1")]
    Msvideo1,
    /// Cirrus Logic AccuPak + PCM in .mov (early-90s QuickTime codec)
    #[value(name = "cljr")]
    Cljr,
    /// Dolby Digital audio-only in .ac3 (broadcast ATSC/DVD audio spec)
    #[value(name = "ac3")]
    Ac3,
    /// Dolby Digital Plus audio-only in .eac3 (streaming-era enhanced spec)
    #[value(name = "eac3")]
    Eac3,
    /// TTA lossless audio-only in .tta (True Audio archival)
    #[value(name = "tta")]
    Tta,
    /// DTS audio-only in .dts (disc-era surround spec)
    #[value(name = "dca")]
    Dca,
    /// Raw uncompressed video + PCM in .avi/.mkv (bit-exact interchange —
    /// keeps the source pixel format, no conversion)
    #[value(name = "raw")]
    Raw,
    /// AIFF + PCM big-endian audio-only in .aiff (Apple-era lossless master)
    #[value(name = "aiff")]
    Aiff,
    /// 24-bit PCM WAV audio-only in .wav (studio master)
    #[value(name = "pcm24")]
    Pcm24,
    /// 32-bit float WAV audio-only in .wav (DAW interchange)
    #[value(name = "pcm32f")]
    Pcm32f,
    /// G.711 mu-law audio-only in .au, pinned 8kHz mono (telephony/IVR spec)
    #[value(name = "mulaw")]
    Mulaw,
    /// CRI ADX ADPCM audio-only in .adx (Sega-era game audio)
    #[value(name = "adx")]
    Adx,
    /// IMA-ADPCM audio-only in .wav (classic game-engine audio)
    #[value(name = "adpcm")]
    Adpcm,
    /// G.711 A-law audio-only in .au, pinned 8kHz mono (Euro telephony spec)
    #[value(name = "alaw")]
    Alaw,
    /// Speex audio-only in .spx via libspeex (Ogg Speex — VoIP/podcast era)
    #[value(name = "speex")]
    Speex,
    /// 8-bit unsigned PCM audio-only in .wav (retro micro-audio)
    #[value(name = "pcm8")]
    Pcm8,
    /// Microsoft ADPCM audio-only in .wav (classic Windows/game audio)
    #[value(name = "adpcmms")]
    Adpcmms,
    /// G.722 ADPCM audio-only in .wav, pinned 16kHz mono (wideband telephony)
    #[value(name = "g722")]
    G722,
    /// RealAudio 1.0 audio-only in .rm, pinned 8kHz mono (dial-up era spec)
    #[value(name = "ra144")]
    Ra144,
    /// Nellymoser Asao audio-only in .flv (Flash-era voice codec)
    #[value(name = "nelly")]
    Nelly,
    /// WavPack lossless audio-only in .wv (audiophile lossless archival)
    #[value(name = "wv")]
    Wv,
    /// MPEG Layer II audio-only in .mp2 (broadcast/DAB-era audio)
    #[value(name = "mp2")]
    Mp2,
    /// Apple CAF big-endian PCM audio-only in .caf (GarageBand/Logic interchange)
    #[value(name = "caf")]
    Caf,
    /// Sony Wave64 24-bit PCM audio-only in .w64 (>4GB long recordings)
    #[value(name = "w64")]
    W64,
    /// Creative Voice PCM audio-only in .voc (DOS-era game audio)
    #[value(name = "voc")]
    Voc,
    /// aptX audio-only in .aptx (Bluetooth codec delivery)
    #[value(name = "aptx")]
    Aptx,
    /// SBC audio-only in .sbc (Bluetooth A2DP baseline codec)
    #[value(name = "sbc")]
    Sbc,
    /// G.723.1 audio-only in .tco, pinned 8kHz mono (VoIP-era codec)
    #[value(name = "g723")]
    G723,
    /// Vidvox HAP (DXT1) + PCM in .mov/.avi (live-visual codec — VJ/Resolume/TouchDesigner ingest; --alpha upgrades to Hap Alpha DXT5)
    #[value(name = "hap")]
    Hap,
    /// Vidvox Hap Q (DXT5-YCoCg) + PCM in .mov/.avi (higher-quality live-visual variant)
    #[value(name = "hapq")]
    Hapq,
    /// GoPro CineForm HD + PCM in .mov/.avi (action-cam NLE intermediate)
    #[value(name = "cfhd")]
    Cfhd,
    /// SMPTE VC-2/Dirac + PCM in .mov (BBC broadcast intermediate codec)
    #[value(name = "vc2")]
    Vc2,
    /// MagicYUV lossless + PCM in .avi (NLE-era fast lossless intermediate)
    #[value(name = "magicyuv")]
    Magicyuv,
    /// AJA Kona 10-bit RGB + PCM in .mov (broadcast capture-card master)
    #[value(name = "r10k")]
    R10k,
    /// Dolby TrueHD audio-only in .thd (Blu-ray lossless — experimental encoder, -strict -2)
    #[value(name = "truehd")]
    Truehd,
    /// Meridian Lossless Packing audio-only in .mlp (HD-DVD era lossless — experimental encoder, -strict -2)
    #[value(name = "mlp")]
    Mlp,
    /// H.261 raw elementary in .h261 (QCIF/CIF videoconference test vectors — snaps to legal canvas, video-only)
    #[value(name = "h261")]
    H261,
    /// H.263 raw elementary in .h263 (H.324 videoconference test vectors — snaps to legal canvas, video-only)
    #[value(name = "h263")]
    H263,
    /// Avid Meridien uncompressed + PCM in .mov (broadcast capture-card ingest — snaps to 720x486, experimental -strict -2)
    #[value(name = "avui")]
    Avui,
    /// H.264 + AAC in .ts/.m2ts MPEG-TS — broadcast ingest, IPTV/DVB
    /// archives, and the container smart TVs capture
    Ts,
    /// MPEG-2 4:2:2 + 48kHz stereo PCM in .mxf — XDCAM/OP1a broadcast
    /// master (the interchange file decks and QC rooms hand around)
    Mxf,
    /// VP9 in .ivf elementary stream (no audio — MSE/Shaka test
    /// vectors and the raw stream WebRTC tooling expects)
    Ivf,
    /// MPEG-2 4:2:2 + 48kHz stereo PCM in .gxf — General eXchange
    /// Format, the Grass Valley broadcast-server interchange spec
    /// (fixed PAL/NTSC canvas, auto-snapped by source rate)
    Gxf,
    /// MPEG-2 + MP2 in .wtv — Windows Media Center recordings
    /// (WMC-era TV archives playable on Windows Media Player)
    Wtv,
    /// MJPEG + PCM in .smjpg — Loki/SDL-game video
    /// (smpeg-era open-source game FMV)
    Smjpeg,
    /// FFV1 + FLAC in .nut — ffmpeg's own lossless swap container
    /// (intermediate/archive grade, everything ffmpeg reads back)
    Nut,
    /// Per-frame MD5 checksum manifest (-f framemd5 — archival
    /// decode-fidelity verification: every decoded frame's hash in text)
    Framemd5,
    /// YUV4MPEG2 elementary video in .y4m (Avisynth/VapourSynth/x264-CLI
    /// era interchange — raw uncompressed, video only)
    Y4m,
}

#[derive(clap::Args, Debug)]
pub struct DeliverArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Destination canvas; all map to 1080x1920 / -14 LUFS
    #[arg(long, value_enum, default_value_t = DeliverPlatform::Social)]
    pub platform: DeliverPlatform,
    /// Output frame rate (default 30; use 60 for gameplay/sport uploads)
    #[arg(long)]
    pub fps: Option<u32>,
    /// H.264 quality level (default 20; lower = sharper/larger, 18 visually lossless)
    #[arg(long)]
    pub crf: Option<u32>,
    /// x264 encode profile for old devices (baseline|main|high —
    /// car players, kiosks, old phones reject High)
    #[arg(long)]
    pub profile: Option<TranscodeProfile>,
    /// x264 encode level, e.g. 3.1 (device-compat ingest specs)
    #[arg(long)]
    pub level: Option<String>,
    /// Max B-frames (0 = decode order = display order — baseline/mobile spec)
    #[arg(long)]
    pub bf: Option<u32>,
    /// Peak video bitrate the encode may burst to, like `4500k`/`6M` —
    /// platform ingest cap (Instagram ~25M, Twitch ≤6000k); paired with
    /// --bufsize for a real CBR envelope
    #[arg(long)]
    pub maxrate: Option<String>,
    /// Rate-control buffer like `9000k`/`12M` (defaults to 2x --maxrate
    /// when only --maxrate is given — the standard CBR pairing)
    #[arg(long)]
    pub bufsize: Option<String>,
    /// Burn this .srt/.vtt onto the delivery canvas (captioned Reels in one pass)
    #[arg(long)]
    pub subs: Option<PathBuf>,
    /// Integrated loudness target in LUFS (overrides the platform default:
    /// -14 video, -16 podcast) — Spotify -14, Apple Podcasts -16, custom specs
    #[arg(long, allow_hyphen_values = true)]
    pub lufs: Option<f64>,
    /// Push the rendered pack live instead of writing a file — rtmp://,
    /// rtmps://, tcp://, udp:// (render-and-stream premieres in one pass)
    #[arg(long)]
    pub to: Option<String>,
    /// Force channel count on the pack (1 = mono podcast voice feed,
    /// 2 = stereo)
    #[arg(long)]
    pub channels: Option<u8>,
    /// Render only the first SEC seconds of the pack (approval/QC preview)
    #[arg(long)]
    pub preview: Option<f64>,
    /// Burn this PNG/JPG as a corner watermark during the pack render
    /// (channel branding on every export in one pass)
    #[arg(long)]
    pub logo: Option<PathBuf>,
    /// Watermark corner (default br — bottom-right)
    #[arg(long, value_enum)]
    pub logo_position: Option<LogoPos>,
    /// Watermark opacity 0..=1 (default 1.0; ~0.6 reads as a ghost mark)
    #[arg(long)]
    pub logo_opacity: Option<f64>,
    /// Prepend this clip before the pack render — channel intro/bumper
    /// baked into every export (normalized to the platform canvas)
    #[arg(long)]
    pub intro: Option<PathBuf>,
    /// Append this clip after the pack render — outro/CTA card
    #[arg(long)]
    pub outro: Option<PathBuf>,
    /// Attach this image as embedded cover art on the podcast feed pack
    /// (--platform podcast: attached_pic on the m4a — Apple/Spotify art)
    #[arg(long)]
    pub cover: Option<PathBuf>,
    /// Embed chapter markers from a YouTube-format list ("mm:ss title" per
    /// line — same file `chapter --yt` exports) as real container chapters:
    /// podcast/audiobook feeds get Apple Podcasts seek stops, video packs get
    /// mp4/m4b chapters — YouTube and players read them as timeline markers
    #[arg(long)]
    pub chapters: Option<PathBuf>,
    /// Title metadata tag written into the pack (Apple Podcasts title)
    #[arg(long)]
    pub title: Option<String>,
    /// Author/artist metadata tag (Apple Podcasts author)
    #[arg(long)]
    pub author: Option<String>,
    /// Album/show metadata tag (podcast show name, audiobook title)
    #[arg(long)]
    pub album: Option<String>,
    /// Genre metadata tag
    #[arg(long)]
    pub genre: Option<String>,
    /// Comment/description metadata tag (audiobook synopsis, show notes)
    #[arg(long)]
    pub comment: Option<String>,
    /// Package one service out of a multi-program transport stream
    /// (broadcast pickup ingest — `probe.programs[]` lists services;
    /// picks program NUMBER, not an index; the pack takes that
    /// service's first video/audio member)
    #[arg(long)]
    pub program: Option<u32>,
    /// Pin the video track timescale (-video_track_timescale N —
    /// broadcast pickup specs that lock the mp4 clock to 90000/30000;
    /// video packs only)
    #[arg(long)]
    pub timescale: Option<u32>,
    /// Force a colr atom into the mp4 master (+write_colr — platform QC
    /// that requires the atom even when color metadata is unspecified;
    /// not for --to streaming)
    #[arg(long)]
    pub colr: bool,
    /// Keyframe interval in frames on the pack encode (-g N — platform
    /// ingest specs like "IDR at least every 2s" / seek granularity)
    #[arg(long)]
    pub gop: Option<u32>,
    /// Pick the audio track with this ISO-639-2 language for the pack
    /// (`--lang jpn` — multi-language masters make one pack per dub:
    /// the language-tagged track becomes the pack's audio)
    #[arg(long)]
    pub lang: Option<String>,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum LogoPos {
    Tl,
    Tr,
    Bl,
    Br,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum DeliverPlatform {
    Social,
    Reels,
    Tiktok,
    Shorts,
    /// 1:1 feed grid (1080x1080, -14 LUFS)
    Square,
    /// 16:9 landscape upload (1920x1080, -14 LUFS)
    Youtube,
    /// 小红书 3:4 portrait feed (1080x1440, -14 LUFS)
    Xhs,
    /// 微信视频号 6:7 portrait feed (1080x1260, -14 LUFS)
    Wechat,
    /// 抖音 9:16 vertical feed (1080x1920, -14 LUFS)
    Douyin,
    /// 快手 9:16 vertical feed (1080x1920, -14 LUFS)
    Kuaishou,
    /// B站 16:9 landscape upload (1920x1080, -14 LUFS)
    Bilibili,
    /// Pinterest idea-pin 2:3 portrait feed (1000x1500, -14 LUFS)
    Pinterest,
    /// X/Twitter feed video 16:9 landscape (1280x720, -14 LUFS)
    X,
    /// LinkedIn feed video 16:9 landscape (1920x1080, -14 LUFS)
    Linkedin,
    /// Vimeo upload 16:9 landscape (1920x1080, -14 LUFS)
    Vimeo,
    /// Bluesky feed video 16:9 landscape (1920x1080, -14 LUFS)
    Bluesky,
    /// Meta Threads feed video 4:5 portrait (1080x1350, -14 LUFS)
    Threads,
    /// Instagram feed video 4:5 portrait (1080x1350, -14 LUFS — the
    /// non-Reels feed slot; Reels is the 9:16 `reels` platform)
    Instagram,
    /// Facebook feed video 4:5 portrait (1080x1350, -14 LUFS)
    Facebook,
    /// Mastodon feed video 16:9 landscape (1280x720, -14 LUFS)
    Mastodon,
    /// Telegram video-note circle (640x640 1:1, mono audio — кружок spec)
    Circle,
    /// Audio-only podcast pack (m4a, AAC 128k/48k, -16 LUFS — feed spec)
    Podcast,
    /// Audiobook pack (m4b, AAC 96k/48k, -16 LUFS — Apple Books/Audible;
    /// pair with --chapters/--title/--author for a finished book)
    Audiobook,
    /// Spotify Canvas loop 9:16 (1080x1920, -14 LUFS — the 3-8s vertical
    /// loop behind a track; cut the loop first, then deliver)
    Canvas,
    /// Snapchat Spotlight 9:16 (1080x1920, -14 LUFS)
    Snapchat,
    /// 微博 feed video 16:9 landscape (1920x1080, -14 LUFS)
    Weibo,
    /// WhatsApp Status 9:16 (1080x1920, -14 LUFS)
    Whatsapp,
    /// Twitch VOD/clip 16:9 landscape (1920x1080, -14 LUFS)
    Twitch,
    /// Discord server video 16:9 landscape (1280x720, -14 LUFS — pair
    /// with `compress --size discord` for the 10MB upload cap)
    Discord,
    /// Shopify product video 1:1 square (1080x1080, -14 LUFS — product
    /// pages use square or 4:5)
    Shopify,
    /// Amazon listing video 16:9 landscape (1920x1080, -14 LUFS)
    Amazon,
    /// Etsy listing video 1:1 square (1080x1080, -14 LUFS — product
    /// gallery thumbnails crop square)
    Etsy,
    /// Rumble/Odysee video 16:9 landscape (1920x1080, -14 LUFS)
    Rumble,
    /// Kick stream clip/VOD 16:9 landscape (1920x1080, -14 LUFS)
    Kick,
    /// LINE video post 9:16 vertical (1080x1920, -14 LUFS)
    Line,
    /// VK clip 16:9 landscape (1920x1080, -14 LUFS)
    Vk,
    /// Dailymotion upload 16:9 landscape (1920x1080, -14 LUFS)
    Dailymotion,
    /// Odysee upload 16:9 landscape (1920x1080, -14 LUFS)
    Odysee,
    /// Trovo stream clip 16:9 landscape (1920x1080, -14 LUFS)
    Trovo,
    /// Substack video post 16:9 landscape (1920x1080, -14 LUFS)
    Substack,
    /// Triller video 9:16 vertical (1080x1920, -14 LUFS)
    Triller,
    /// Lemon8 post 3:4 portrait (1080x1440, -14 LUFS)
    Lemon8,
    /// Niconico upload 16:9 landscape (1920x1080, -14 LUFS)
    Niconico,
    /// SOOP (AfreecaTV) stream clip 16:9 landscape (1920x1080, -14 LUFS)
    Soop,
    /// Xigua video 16:9 landscape (1920x1080, -14 LUFS)
    Xigua,
    /// Likee short video 9:16 vertical (1080x1920, -14 LUFS)
    Likee,
    /// Moj short video 9:16 vertical (1080x1920, -14 LUFS)
    Moj,
    /// Josh short video 9:16 vertical (1080x1920, -14 LUFS)
    Josh,
    /// PeerTube video 16:9 landscape (1920x1080, -14 LUFS)
    Peertube,
    /// Floatplane video 16:9 landscape (1920x1080, -14 LUFS)
    Floatplane,
    /// Nebula video 16:9 landscape (1920x1080, -14 LUFS)
    Nebula,
    /// CHZZK (Naver) clip 16:9 landscape (1920x1080, -14 LUFS)
    Chzzk,
    /// Douyu clip 16:9 landscape (1920x1080, -14 LUFS)
    Douyu,
    /// Huya clip 16:9 landscape (1920x1080, -14 LUFS)
    Huya,
    /// Weverse fan clip 9:16 vertical (1080x1920, -14 LUFS)
    Weverse,
    /// Kwai (international Kuaishou) 9:16 vertical (1080x1920, -14 LUFS)
    Kwai,
    /// SnackVideo 9:16 vertical (1080x1920, -14 LUFS)
    Snackvideo,
    /// Udemy course lecture 16:9 landscape (1920x1080, -14 LUFS)
    Udemy,
    /// Coursera course lecture 16:9 landscape (1920x1080, -14 LUFS)
    Coursera,
    /// Teachable course lecture 16:9 landscape (1920x1080, -14 LUFS)
    Teachable,
    /// Kajabi course lecture 16:9 landscape (1920x1080, -14 LUFS)
    Kajabi,
    /// Patreon post video 16:9 landscape (1920x1080, -14 LUFS)
    Patreon,
    /// Skillshare course video 16:9 landscape (1920x1080, -14 LUFS)
    Skillshare,
    /// Thinkific course video 16:9 landscape (1920x1080, -14 LUFS)
    Thinkific,
    /// Podia course/digital-download video 16:9 landscape (1920x1080, -14 LUFS)
    Podia,
    /// LearnWorlds course video 16:9 landscape (1920x1080, -14 LUFS)
    Learnworlds,
    /// Gumroad digital-product video 16:9 landscape (1920x1080, -14 LUFS)
    Gumroad,
    /// Wistia business-host video 16:9 landscape (1920x1080, -14 LUFS)
    Wistia,
    /// Domestika course video 16:9 landscape (1920x1080, -14 LUFS)
    Domestika,
    /// Steam store-page video 16:9 landscape (1920x1080, -14 LUFS)
    Steam,
    /// itch.io game-page video 16:9 landscape (1920x1080, -14 LUFS)
    Itch,
    /// Shopee product video 16:9 landscape (1920x1080, -14 LUFS)
    Shopee,
    /// Lazada product video 16:9 landscape (1920x1080, -14 LUFS)
    Lazada,
    /// Taobao product video 16:9 landscape (1920x1080, -14 LUFS)
    Taobao,
    /// DLive stream video 16:9 landscape (1920x1080, -14 LUFS)
    Dlive,
    /// Minds video 16:9 landscape (1920x1080, -14 LUFS)
    Minds,
    /// Telegram channel/chat video 16:9 landscape (1920x1080, -14 LUFS)
    Telegram,
    /// TIDAL video single 16:9 landscape (1920x1080, -14 LUFS)
    Tidal,
    /// Deezer video single 16:9 landscape (1920x1080, -14 LUFS)
    Deezer,
    /// Qobuz video single 16:9 landscape (1920x1080, -14 LUFS)
    Qobuz,
    /// Yandex Music video single 16:9 landscape (1920x1080, -14 LUFS)
    Yandexmusic,
    /// Napster video single 16:9 landscape (1920x1080, -14 LUFS)
    Napster,
    /// JOOX video single 16:9 landscape (1920x1080, -14 LUFS)
    Joox,
    /// SoundCloud video upload 16:9 landscape (1920x1080, -14 LUFS)
    Soundcloud,
    /// Mixcloud DJ-set video 16:9 landscape (1920x1080, -14 LUFS)
    Mixcloud,
    /// Audiomack release video 16:9 landscape (1920x1080, -14 LUFS)
    Audiomack,
    /// Bandcamp track-page video 16:9 landscape (1920x1080, -14 LUFS)
    Bandcamp,
    /// VEVO music video 16:9 landscape (1920x1080, -14 LUFS)
    Vevo,
    /// Roku channel video 16:9 landscape (1920x1080, -14 LUFS)
    Roku,
    /// Plex library video 16:9 landscape (1920x1080, -14 LUFS)
    Plex,
    /// iQIYI upload 16:9 landscape (1920x1080, -14 LUFS)
    Iqiyi,
    /// Youku upload 16:9 landscape (1920x1080, -14 LUFS)
    Youku,
    /// WeTV (Tencent overseas) 16:9 landscape (1920x1080, -14 LUFS)
    Wetv,
    /// Viki (Rakuten) video 16:9 landscape (1920x1080, -14 LUFS)
    Viki,
    /// Crunchyroll video 16:9 landscape (1920x1080, -14 LUFS)
    Crunchyroll,
    /// Funimation video 16:9 landscape (1920x1080, -14 LUFS)
    Funimation,
    /// Mango TV / MGTV video 16:9 landscape (1920x1080, -14 LUFS)
    Mgtv,
    /// BIGO LIVE stream 16:9 landscape (1920x1080, -14 LUFS)
    Bigo,
    /// Nimo TV gaming stream 16:9 landscape (1920x1080, -14 LUFS)
    Nimo,
    /// Tumblr video post 16:9 landscape (1920x1080, -14 LUFS)
    Tumblr,
    /// Dribbble shot video 16:9 landscape (1920x1080, -14 LUFS)
    Dribbble,
    /// Behance project video 16:9 landscape (1920x1080, -14 LUFS)
    Behance,
    /// Flickr video upload 16:9 landscape (1920x1080, -14 LUFS)
    Flickr,
    /// Zhihu video answer 16:9 landscape (1920x1080, -14 LUFS)
    Zhihu,
    /// Kakao (KakaoTalk/Channel) video 16:9 landscape (1920x1080, -14 LUFS)
    Kakao,
    /// Naver TV/Blog video 16:9 landscape (1920x1080, -14 LUFS)
    Naver,
    /// Coub looping video 16:9 landscape (1920x1080, -14 LUFS)
    Coub,
    /// Imgur video post 16:9 landscape (1920x1080, -14 LUFS)
    Imgur,
    /// 9GAG video post 16:9 landscape (1920x1080, -14 LUFS)
    #[value(name = "9gag")]
    NineGag,
    /// Streamable clip host 16:9 landscape (1920x1080, -14 LUFS)
    Streamable,
    /// Viddsee short-film platform 16:9 landscape (1920x1080, -14 LUFS)
    Viddsee,
    /// Rutube video host 16:9 landscape (1920x1080, -14 LUFS)
    Rutube,
    /// Odnoklassniki (ok.ru) video 16:9 landscape (1920x1080, -14 LUFS)
    Ok,
    /// Yandex Zen channel 16:9 landscape (1920x1080, -14 LUFS)
    Zen,
    /// OPENREC.tv JP game streamer clips 16:9 landscape (1920x1080, -14 LUFS)
    Openrec,
    /// TwitCasting JP live host 16:9 landscape (1920x1080, -14 LUFS)
    Twitcasting,
    /// SHOWROOM JP streamer clips 16:9 landscape (1920x1080, -14 LUFS)
    Showroom,
    /// FC2 video host 16:9 landscape (1920x1080, -14 LUFS)
    Fc2,
    /// TVING KR streaming 16:9 landscape (1920x1080, -14 LUFS)
    Tving,
    /// Wavve KR streaming 16:9 landscape (1920x1080, -14 LUFS)
    Wavve,
    /// Watcha KR streaming 16:9 landscape (1920x1080, -14 LUFS)
    Watcha,
    /// Vidio ID streaming 16:9 landscape (1920x1080, -14 LUFS)
    Vidio,
    /// meWATCH SG streaming 16:9 landscape (1920x1080, -14 LUFS)
    Mewatch,
    /// TVer JP catch-up TV 16:9 landscape (1920x1080, -14 LUFS)
    Tver,
    /// ABEMA JP streaming 16:9 landscape (1920x1080, -14 LUFS)
    Abema,
    /// Spotify video podcast 16:9 landscape (1920x1080, -14 LUFS)
    Spotify,
    /// Apple Podcasts video episode 16:9 landscape (1920x1080, -14 LUFS)
    Apple,
    /// Amazon Music video podcast 16:9 landscape (1920x1080, -14 LUFS)
    Amazonmusic,
    /// iHeartRadio video podcast 16:9 landscape (1920x1080, -14 LUFS)
    Iheartradio,
    /// Pandora video podcast 16:9 landscape (1920x1080, -14 LUFS)
    Pandora,
    /// Castbox video podcast 16:9 landscape (1920x1080, -14 LUFS)
    Castbox,
    /// Podbean video podcast 16:9 landscape (1920x1080, -14 LUFS)
    Podbean,
    /// Hotstar IN streaming 16:9 landscape (1920x1080, -14 LUFS)
    Hotstar,
    /// JioTV IN streaming 16:9 landscape (1920x1080, -14 LUFS)
    Jiotv,
    /// SonyLIV IN streaming 16:9 landscape (1920x1080, -14 LUFS)
    Sonyliv,
    /// MX Player IN streaming 16:9 landscape (1920x1080, -14 LUFS)
    Mxplayer,
    /// ZEE5 IN streaming 16:9 landscape (1920x1080, -14 LUFS)
    Zee5,
    /// Showmax ZA streaming 16:9 landscape (1920x1080, -14 LUFS)
    Showmax,
    /// SHAHID MENA streaming 16:9 landscape (1920x1080, -14 LUFS)
    Shahid,
    /// Tubi free streaming 16:9 landscape (1920x1080, -14 LUFS)
    Tubi,
    /// Pluto TV free streaming 16:9 landscape (1920x1080, -14 LUFS)
    Pluto,
    /// DAZN sports streaming 16:9 landscape (1920x1080, -14 LUFS)
    Dazn,
    /// ESPN sports streaming 16:9 landscape (1920x1080, -14 LUFS)
    Espn,
    /// Hulu streaming 16:9 landscape (1920x1080, -14 LUFS)
    Hulu,
    /// U-NEXT JP streaming 16:9 landscape (1920x1080, -14 LUFS)
    #[value(name = "u-next")]
    UNext,
    /// GYAO! JP free streaming 16:9 landscape (1920x1080, -14 LUFS)
    Gyao,
    /// Netflix SVOD 16:9 landscape (1920x1080, -14 LUFS)
    Netflix,
    /// Disney+ SVOD 16:9 landscape (1920x1080, -14 LUFS)
    Disney,
    /// Max (HBO) SVOD 16:9 landscape (1920x1080, -14 LUFS)
    Max,
    /// Peacock SVOD 16:9 landscape (1920x1080, -14 LUFS)
    Peacock,
    /// Paramount+ SVOD 16:9 landscape (1920x1080, -14 LUFS)
    Paramount,
    /// Apple TV+ SVOD 16:9 landscape (1920x1080, -14 LUFS)
    Appletv,
    /// Prime Video SVOD 16:9 landscape (1920x1080, -14 LUFS)
    Primevideo,
    /// Globoplay BR streaming 16:9 landscape (1920x1080, -14 LUFS)
    Globoplay,
    /// Viaplay Nordic streaming 16:9 landscape (1920x1080, -14 LUFS)
    Viaplay,
    /// Joyn DE streaming 16:9 landscape (1920x1080, -14 LUFS)
    Joyn,
    /// RaiPlay IT streaming 16:9 landscape (1920x1080, -14 LUFS)
    Raiplay,
    /// Atresplayer ES streaming 16:9 landscape (1920x1080, -14 LUFS)
    Atresplayer,
    /// ITVX UK streaming 16:9 landscape (1920x1080, -14 LUFS)
    Itvx,
    /// Crave CA streaming 16:9 landscape (1920x1080, -14 LUFS)
    Crave,
    /// Stan AU streaming 16:9 landscape (1920x1080, -14 LUFS)
    Stan,
    /// myCANAL FR streaming 16:9 landscape (1920x1080, -14 LUFS)
    Mycanal,
    /// Sky Go DE/UK/IT streaming 16:9 landscape (1920x1080, -14 LUFS)
    Skygo,
    /// Movistar+ ES streaming 16:9 landscape (1920x1080, -14 LUFS)
    Movistar,
    /// Viu pan-Asia streaming 16:9 landscape (1920x1080, -14 LUFS)
    Viu,
    /// Voot IN streaming 16:9 landscape (1920x1080, -14 LUFS)
    Voot,
    /// Claro Video LatAm streaming 16:9 landscape (1920x1080, -14 LUFS)
    Clarovideo,
    /// NHK+ JP streaming 16:9 landscape (1920x1080, -14 LUFS)
    Nhk,
    /// ARTE FR/DE culture channel 16:9 landscape (1920x1080, -14 LUFS)
    Arte,
    /// TV 2 Play DK streaming 16:9 landscape (1920x1080, -14 LUFS)
    Tv2play,
    /// NPO Start NL public streaming 16:9 landscape (1920x1080, -14 LUFS)
    Npostart,
    /// RTVE Play ES public streaming 16:9 landscape (1920x1080, -14 LUFS)
    Rtve,
    /// TVP Stream PL public streaming 16:9 landscape (1920x1080, -14 LUFS)
    Tvp,
    /// Voyo CZ/SK streaming 16:9 landscape (1920x1080, -14 LUFS)
    Voyo,
    /// Wakanim EU anime streaming 16:9 landscape (1920x1080, -14 LUFS)
    Wakanim,
    /// ADN FR anime streaming 16:9 landscape (1920x1080, -14 LUFS)
    Adn,
    /// Laftel KR anime streaming 16:9 landscape (1920x1080, -14 LUFS)
    Laftel,
    /// Aniplus ASIA anime streaming 16:9 landscape (1920x1080, -14 LUFS)
    Aniplus,
    /// HIDIVE anime streaming 16:9 landscape (1920x1080, -14 LUFS)
    Hidive,
    /// RetroCrush classic anime streaming 16:9 landscape (1920x1080, -14 LUFS)
    Retrocrush,
    /// Bstation SEA anime streaming 16:9 landscape (1920x1080, -14 LUFS)
    Bstation,
    /// ARD German public broadcaster 16:9 landscape (1920x1080, -14 LUFS)
    Ard,
    /// ZDF German public broadcaster 16:9 landscape (1920x1080, -14 LUFS)
    Zdf,
    /// NRK Norwegian public broadcaster 16:9 landscape (1920x1080, -14 LUFS)
    Nrk,
    /// SVT Swedish public broadcaster 16:9 landscape (1920x1080, -14 LUFS)
    Svt,
    /// DR Danish public broadcaster 16:9 landscape (1920x1080, -14 LUFS)
    Dr,
    /// CBC Canadian public broadcaster 16:9 landscape (1920x1080, -14 LUFS)
    Cbc,
    /// SBS Australian public broadcaster 16:9 landscape (1920x1080, -14 LUFS)
    Sbs,
    /// TF1 French broadcaster 16:9 landscape (1920x1080, -14 LUFS)
    Tf1,
    /// France.tv French public broadcaster 16:9 landscape (1920x1080, -14 LUFS)
    Francetv,
    /// Mediaset Italian broadcaster 16:9 landscape (1920x1080, -14 LUFS)
    Mediaset,
    /// Channel 4 UK broadcaster 16:9 landscape (1920x1080, -14 LUFS)
    Channel4,
    /// 10 Play Australian broadcaster 16:9 landscape (1920x1080, -14 LUFS)
    Tenplay,
    /// NOW TV UK streaming 16:9 landscape (1920x1080, -14 LUFS)
    Nowtv,
    /// SRF Swiss public broadcaster 16:9 landscape (1920x1080, -14 LUFS)
    Srf,
    /// Fubo US sports-first live TV 16:9 landscape (1920x1080, -14 LUFS)
    Fubo,
    /// Sling US vMVPD live TV 16:9 landscape (1920x1080, -14 LUFS)
    Sling,
    /// Philo US entertainment live TV 16:9 landscape (1920x1080, -14 LUFS)
    Philo,
    /// DirecTV US satellite/streaming 16:9 landscape (1920x1080, -14 LUFS)
    Directv,
    /// Xumo US FAST platform 16:9 landscape (1920x1080, -14 LUFS)
    Xumo,
    /// Vidgo US live TV 16:9 landscape (1920x1080, -14 LUFS)
    Vidgo,
    /// Frndly US family live TV 16:9 landscape (1920x1080, -14 LUFS)
    Frndly,
    /// BBC iPlayer UK public streamer 16:9 landscape (1920x1080, -14 LUFS)
    Iplayer,
    /// My5 UK Channel 5 streamer 16:9 landscape (1920x1080, -14 LUFS)
    My5,
    /// BritBox UK catalogue streamer 16:9 landscape (1920x1080, -14 LUFS)
    Britbox,
    /// Acorn TV UK niche streamer 16:9 landscape (1920x1080, -14 LUFS)
    Acorntv,
    /// Shudder US horror niche streamer 16:9 landscape (1920x1080, -14 LUFS)
    Shudder,
    /// Showtime US premium streamer 16:9 landscape (1920x1080, -14 LUFS)
    Showtime,
    /// Starz US premium streamer 16:9 landscape (1920x1080, -14 LUFS)
    Starz,
    /// Reddit video post 16:9 landscape (1920x1080, -14 LUFS)
    Reddit,
    /// Zillow listing tour 16:9 landscape (1920x1080, -14 LUFS — real-estate video)
    Zillow,
    /// eBay listing video 16:9 landscape (1920x1080, -14 LUFS)
    Ebay,
    /// Walmart marketplace video 16:9 landscape (1920x1080, -14 LUFS)
    Walmart,
    /// Poshmark listing video 1:1 square (1080x1080, -14 LUFS)
    Poshmark,
    /// Whatnot live-shopping 9:16 vertical (1080x1920, -14 LUFS)
    Whatnot,
    /// Kanopy library streamer 16:9 landscape (1920x1080, -14 LUFS)
    Kanopy,
    /// MUBI arthouse streamer 16:9 landscape (1920x1080, -14 LUFS)
    Mubi,
    /// Criterion Channel streamer 16:9 landscape (1920x1080, -14 LUFS)
    Criterion,
    /// CuriosityStream doc streamer 16:9 landscape (1920x1080, -14 LUFS)
    Curiositystream,
    /// MagellanTV doc streamer 16:9 landscape (1920x1080, -14 LUFS)
    Magellantv,
    /// Tinder video profile 9:16 vertical (1080x1920, -14 LUFS)
    Tinder,
    /// Bumble video profile 9:16 vertical (1080x1920, -14 LUFS)
    Bumble,
    /// Hinge video prompt 9:16 vertical (1080x1920, -14 LUFS)
    Hinge,
    /// Brightcove B2B video hosting 16:9 landscape (1920x1080, -14 LUFS)
    Brightcove,
    /// JW Player embed player video 16:9 landscape (1920x1080, -14 LUFS)
    Jwplayer,
    /// Kaltura enterprise video platform 16:9 landscape (1920x1080, -14 LUFS)
    Kaltura,
    /// SproutVideo business hosting 16:9 landscape (1920x1080, -14 LUFS)
    Sproutvideo,
    /// Vidyard sales/marketing video hosting 16:9 landscape (1920x1080, -14 LUFS)
    Vidyard,
    /// Uscreen OTT app builder 16:9 landscape (1920x1080, -14 LUFS)
    Uscreen,
    /// VdoCipher DRM hosting 16:9 landscape (1920x1080, -14 LUFS)
    Vdocipher,
    /// Mercari resale listing video 9:16 vertical (1080x1920, -14 LUFS)
    Mercari,
    /// Vinted resale listing video 9:16 vertical (1080x1920, -14 LUFS)
    Vinted,
    /// Depop resale listing video 9:16 vertical (1080x1920, -14 LUFS)
    Depop,
    /// Carousell resale listing video 9:16 vertical (1080x1920, -14 LUFS)
    Carousell,
    /// OLX classifieds listing video 9:16 vertical (1080x1920, -14 LUFS)
    Olx,
    /// Jellyfin media-server video 16:9 landscape (1920x1080, -14 LUFS)
    Jellyfin,
    /// Emby media-server video 16:9 landscape (1920x1080, -14 LUFS)
    Emby,
    /// OnlyFans creator post video 9:16 vertical (1080x1920, -14 LUFS)
    Onlyfans,
    /// Fansly creator post video 9:16 vertical (1080x1920, -14 LUFS)
    Fansly,
    /// pixiv FANBOX creator post video 9:16 vertical (1080x1920, -14 LUFS)
    Fanbox,
    /// Cameo fan-request video 9:16 vertical (1080x1920, -14 LUFS)
    Cameo,
    /// SubscribeStar member post video 9:16 vertical (1080x1920, -14 LUFS)
    Subscribestar,
    /// Ko-fi creator post video 16:9 landscape (1920x1080, -14 LUFS)
    #[value(name = "kofi")]
    Kofi,
    /// Buy Me a Coffee post video 16:9 landscape (1920x1080, -14 LUFS)
    Buymeacoffee,
    /// Buzzsprout video episode 16:9 landscape (1920x1080, -14 LUFS)
    Buzzsprout,
    /// Captivate video episode 16:9 landscape (1920x1080, -14 LUFS)
    Captivate,
    /// Transistor video episode 16:9 landscape (1920x1080, -14 LUFS)
    Transistor,
    /// RedCircle video episode 16:9 landscape (1920x1080, -14 LUFS)
    Redcircle,
    /// Sounder video episode 16:9 landscape (1920x1080, -14 LUFS)
    Sounder,
    /// Acast video episode 16:9 landscape (1920x1080, -14 LUFS)
    Acast,
    /// Spreaker video episode 16:9 landscape (1920x1080, -14 LUFS)
    Spreaker,
    /// BandLab video/visualizer upload 16:9 landscape (1920x1080, -14 LUFS)
    Bandlab,
    /// DistroKid music-video upload 16:9 landscape (1920x1080, -14 LUFS)
    Distrokid,
    /// TuneCore music-video upload 16:9 landscape (1920x1080, -14 LUFS)
    Tunecore,
    /// Amuse music-video upload 16:9 landscape (1920x1080, -14 LUFS)
    Amuse,
    /// CD Baby music-video upload 16:9 landscape (1920x1080, -14 LUFS)
    Cdbaby,
    /// Symphonic music-video upload 16:9 landscape (1920x1080, -14 LUFS)
    Symphonic,
    /// LANDR music-video upload 16:9 landscape (1920x1080, -14 LUFS)
    Landr,
    /// ShareChat short video 9:16 vertical (1080x1920, -14 LUFS)
    Sharechat,
    /// Chingari short video 9:16 vertical (1080x1920, -14 LUFS)
    Chingari,
    /// VMate short video 9:16 vertical (1080x1920, -14 LUFS)
    Vmate,
    /// Blim (Televisa) episode upload 16:9 landscape (1920x1080, -14 LUFS)
    Blim,
    /// Vix (TelevisaUnivision) episode upload 16:9 landscape (1920x1080, -14 LUFS)
    Vix,
    /// iROKOtv Nollywood upload 16:9 landscape (1920x1080, -14 LUFS)
    Irokotv,
    /// STARZPLAY (MENA) episode upload 16:9 landscape (1920x1080, -14 LUFS)
    Starzplay,
    /// Pear Video 梨视频 — Chinese short-form news video 16:9
    Pearvideo,
    /// Haokan 好看视频 — Baidu short video 16:9
    Haokan,
    /// Miaopai 秒拍 — Chinese short video 16:9
    Miaopai,
    /// AcFun — Chinese anime/video community 16:9
    Acfun,
    /// Toutiao 头条视频 — news-feed video 16:9
    Toutiao,
    /// Baijiahao 百家号 — Baidu content-platform video 16:9
    Baijiahao,
    /// Ifeng 凤凰视频 — Phoenix TV video portal 16:9
    Ifeng,
    /// Weishi 微视 — Tencent short video 9:16
    Weishi,
    /// Huoshan 火山小视频 — ByteDance short video 9:16
    Huoshan,
    /// Quanmin 全民小视频 — Baidu short video 9:16
    Quanmin,
    /// Meipai 美拍 — women's lifestyle short video 9:16
    Meipai,
    /// Migu 咪咕视频 — China Mobile streaming (sports/content) 16:9
    Migu,
    /// PPTV — sports/video streaming 16:9
    Pptv,
    /// LeTV 乐视视频 — OTT video portal 16:9
    Letv,
    /// Pinduoduo 拼多多 — social-commerce video 9:16
    Pdd,
    /// JD.com 京东 — e-commerce product video 9:16
    Jd,
    /// VIP 唯品会 — e-commerce video 9:16
    Vip,
    /// Kocowa — K-content US SVOD 16:9
    Kocowa,
    /// Rakuten TV — European TVOD/SVOD 16:9
    Rakuentv,
    /// iWantTFC — Filipino OTT 16:9
    Iwanttfc,
    /// Hoichoi — Bengali OTT 16:9
    Hoichoi,
    /// 17LIVE — JP/TW mobile live app 9:16
    #[value(name = "17live")]
    Live17,
    /// Pococha — JP mobile live streaming 9:16
    Pococha,
    /// Mirrativ — JP mobile game streaming 9:16
    Mirrativ,
    /// MX TakaTak — Indian short video 9:16
    Mxtakatak,
    /// Roposo — Indian short video 9:16
    Roposo,
    /// Boomplay — African music streaming 16:9
    Boomplay,
    /// Sohu 搜狐视频 — CN video portal 16:9
    Sohu,
    /// Pixelfed — Fediverse Instagram-style square posts 1:1
    Pixelfed,
    /// ArtStation — portfolio/video-host 16:9
    Artstation,
    /// Clapper — US TikTok-alt short-video 9:16
    Clapper,
    /// YouNow — live-stream social video 9:16
    Younow,
    /// Meesho — IN commerce video listings 9:16
    Meesho,
    /// Bulbul — IN commerce video listings 9:16
    Bulbul,
    /// Fanvue — creator-economy posts 9:16
    Fanvue,
    /// Temu — e-commerce product video 9:16
    Temu,
    /// Shein — fashion e-commerce product video 9:16
    Shein,
    /// AliExpress — e-commerce product video 9:16
    Aliexpress,
    /// Flipkart — IN e-commerce product video 9:16
    Flipkart,
    /// Zalando — EU fashion e-commerce product video 9:16
    Zalando,
    /// Coupang — KR e-commerce product video 9:16
    Coupang,
    /// MercadoLibre — LatAm e-commerce product video 9:16
    Mercadolibre,

    /// Truth Social video posts 16:9 landscape (1920x1080, -14 LUFS)
    Truthsocial,
    /// GETTR video posts 16:9 landscape (1920x1080, -14 LUFS)
    Gettr,
    /// Parler video posts 16:9 landscape (1920x1080, -14 LUFS)
    Parler,
    /// Locals creator community video 16:9 landscape (1920x1080, -14 LUFS)
    Locals,
    /// Utreon creator video host 16:9 landscape (1920x1080, -14 LUFS)
    Utreon,
    /// caffeine.tv live host 16:9 landscape (1920x1080, -14 LUFS)
    Caffeine,
    /// QQ video posts 16:9 landscape (1920x1080, -14 LUFS)
    Qq,
    /// NBA League Pass — basketball highlights 16:9
    Nba,
    /// NFL+ — football highlights 16:9
    Nfl,
    /// MLB.TV — baseball highlights 16:9
    Mlb,
    /// NHL — hockey highlights 16:9
    Nhl,
    /// FIFA+ — soccer highlights 16:9
    Fifa,
    /// UFC Fight Pass — MMA highlights 16:9
    Ufc,
    /// WWE — wrestling highlights 16:9
    Wwe,
    /// TSN — CA sports broadcaster highlights 16:9
    Tsn,
    /// Sportsnet — CA sports broadcaster highlights 16:9
    Sportsnet,
    /// beIN Sports — MENA/INTL sports broadcaster highlights 16:9
    Beinsports,
    /// Sky Sports — UK sports broadcaster highlights 16:9
    Skysports,
    /// TNT Sports (formerly BT Sport) — UK sports broadcaster highlights 16:9
    Tntsports,
    /// Fox Sports — US sports broadcaster highlights 16:9
    Foxsports,
    /// CBS Sports — US sports broadcaster highlights 16:9
    Cbssports,
    /// Eurosport — EU sports broadcaster highlights 16:9
    Eurosport,
    /// Kayo — AU sports streaming highlights 16:9
    Kayo,
    /// Optus Sport — AU soccer broadcaster highlights 16:9
    Optussport,
    /// SuperSport — ZA sports broadcaster highlights 16:9
    Supersport,
    /// Astro — MY sports broadcaster highlights 16:9
    Astro,
    /// Willow TV — US/CA cricket broadcaster highlights 16:9
    Willow,
    /// Premier Sports — UK sports broadcaster highlights 16:9
    Premier,
    /// La Liga highlights clip 16:9 landscape (1920x1080, -14 LUFS)
    Laliga,
    /// Bundesliga highlights clip 16:9 landscape (1920x1080, -14 LUFS)
    Bundesliga,
    /// Serie A highlights clip 16:9 landscape (1920x1080, -14 LUFS)
    Seriea,
    /// Ligue 1 highlights clip 16:9 landscape (1920x1080, -14 LUFS)
    Ligue1,
    /// MLS highlights clip 16:9 landscape (1920x1080, -14 LUFS)
    Mls,
    /// Champions League highlights clip 16:9 landscape (1920x1080, -14 LUFS)
    Championsleague,
    /// Mildom — JP live-streaming clip 16:9 landscape (1920x1080, -14 LUFS)
    Mildom,
    /// MTV highlight clip 16:9 landscape (1920x1080, -14 LUFS)
    Mtv,
    /// BET highlight clip 16:9 landscape (1920x1080, -14 LUFS)
    Bet,
    /// VH1 highlight clip 16:9 landscape (1920x1080, -14 LUFS)
    Vh1,
    /// Comedy Central highlight clip 16:9 landscape (1920x1080, -14 LUFS)
    Comedycentral,
    /// Nickelodeon highlight clip 16:9 landscape (1920x1080, -14 LUFS)
    Nickelodeon,
    /// Cartoon Network highlight clip 16:9 landscape (1920x1080, -14 LUFS)
    Cartoonnetwork,
    /// Adult Swim highlight clip 16:9 landscape (1920x1080, -14 LUFS)
    Adultswim,
    /// CNN news clip 16:9 landscape (1920x1080, -14 LUFS)
    Cnn,
    /// ABC news clip 16:9 landscape (1920x1080, -14 LUFS)
    Abc,
    /// NBC news clip 16:9 landscape (1920x1080, -14 LUFS)
    Nbc,
    /// CBS news clip 16:9 landscape (1920x1080, -14 LUFS)
    Cbs,
    /// Fox News clip 16:9 landscape (1920x1080, -14 LUFS)
    Foxnews,
    /// Al Jazeera news clip 16:9 landscape (1920x1080, -14 LUFS)
    Aljazeera,
    /// BBC News clip 16:9 landscape (1920x1080, -14 LUFS)
    Bbcnews,
    /// Tmall product video 9:16 portrait (1080x1920, -14 LUFS)
    Tmall,
    /// Noon product video 9:16 portrait (1080x1920, -14 LUFS)
    Noon,
    /// Nykaa product video 9:16 portrait (1080x1920, -14 LUFS)
    Nykaa,
    /// Daraz product video 9:16 portrait (1080x1920, -14 LUFS)
    Daraz,
    /// Jumia Africa product video 9:16 portrait (1080x1920, -14 LUFS)
    Jumia,
    /// TikTok Shop product video 9:16 portrait (1080x1920, -14 LUFS)
    Tiktokshop,
    /// Quikr India classified video 9:16 portrait (1080x1920, -14 LUFS)
    Quikr,
    /// Dubizzle MENA classifieds video 9:16 portrait (1080x1920, -14 LUFS)
    Dubizzle,
    /// Wallapop ES classifieds video 9:16 portrait (1080x1920, -14 LUFS)
    Wallapop,
    /// Subito IT classifieds video 9:16 portrait (1080x1920, -14 LUFS)
    Subito,
    /// Kleinanzeigen DE classifieds video 9:16 portrait (1080x1920, -14 LUFS)
    Kleinanzeigen,
    /// Blocket SE classifieds video 9:16 portrait (1080x1920, -14 LUFS)
    Blocket,
    /// Tradera SE auction video 9:16 portrait (1080x1920, -14 LUFS)
    Tradera,
    /// Leboncoin FR classifieds video 9:16 portrait (1080x1920, -14 LUFS)
    Leboncoin,
    /// Dropbox video preview link 16:9 landscape (1920x1080, -14 LUFS)
    Dropbox,
    /// Box video preview 16:9 landscape (1920x1080, -14 LUFS)
    Box,
    /// OneDrive video preview link 16:9 landscape (1920x1080, -14 LUFS)
    Onedrive,
    /// Google Drive video preview link 16:9 landscape (1920x1080, -14 LUFS)
    Gdrive,
    /// MEGA video preview link 16:9 landscape (1920x1080, -14 LUFS)
    Mega,
    /// WeTransfer video preview link 16:9 landscape (1920x1080, -14 LUFS)
    Wetransfer,
    /// SendAnywhere video link 16:9 landscape (1920x1080, -14 LUFS)
    Sendanywhere,
    /// Loom async video message link 16:9 landscape (1920x1080, -14 LUFS)
    Loom,
    /// Tella screen-recording share link 16:9 landscape (1920x1080, -14 LUFS)
    Tella,
    /// ScreenPal hosted video link 16:9 landscape (1920x1080, -14 LUFS)
    Screenpal,
    /// Cisco Vidcast share link 16:9 landscape (1920x1080, -14 LUFS)
    Vidcast,
    /// Microsoft Stream video link 16:9 landscape (1920x1080, -14 LUFS)
    Msstream,
    /// Panopto enterprise video link 16:9 landscape (1920x1080, -14 LUFS)
    Panopto,
    /// SharePoint video preview link 16:9 landscape (1920x1080, -14 LUFS)
    Sharepoint,
    /// Zoom cloud recordings / Zoom Events replays
    Zoom,
    /// Webex (Cisco) meeting recordings
    Webex,
    /// GoToMeeting / GoTo Webinar recordings
    Gotomeeting,
    /// BlueJeans meeting & event recordings
    Bluejeans,
    /// RingCentral meeting recordings
    Ringcentral,
    /// Hopin (RingCentral Events) session recordings
    Hopin,
    /// Airmeet virtual-event session recordings
    Airmeet,
    /// Shutterstock contributor stock-footage upload
    Shutterstock,
    /// Pond5 contributor stock-footage upload
    Pond5,
    /// Artgrid contributor stock-footage upload
    Artgrid,
    /// Storyblocks contributor stock-footage upload
    Storyblocks,
    /// Videvo contributor stock-footage upload
    Videvo,
    /// Motion Array contributor upload
    Motionarray,
    /// Dissolve contributor stock-footage upload
    Dissolve,
    /// Rightmove property-listing video (UK)
    Rightmove,
    /// Zoopla property-listing video (UK)
    Zoopla,
    /// Realtor.com property-listing video (US)
    Realtor,
    /// Redfin property-listing video (US)
    Redfin,
    /// Domain property-listing video (AU)
    Domain,
    /// ImmoScout24 property-listing video (DE)
    Immoscout,
    /// Idealista property-listing video (ES/IT/PT)
    Idealista,
    /// AutoTrader vehicle-listing video
    Autotrader,
    /// CarGurus vehicle-listing video
    Cargurus,
    /// Carvana vehicle-listing video
    Carvana,
    /// Carwow vehicle-listing video
    Carwow,
    /// mobile.de vehicle-listing video (DE)
    Mobilede,
    /// AutoScout24 vehicle-listing video (EU)
    Autoscout,
    /// Copart salvage-auction listing video
    Copart,
    /// Airbnb listing video
    Airbnb,
    /// Booking.com listing video
    Booking,
    /// Expedia listing video
    Expedia,
    /// Hotels.com listing video
    Hotels,
    /// Tripadvisor listing video
    Tripadvisor,
    /// Agoda listing video (APAC)
    Agoda,
    /// Vrbo listing video
    Vrbo,
    /// Uber Eats restaurant-listing video
    Ubereats,
    /// DoorDash restaurant-listing video
    Doordash,
    /// Deliveroo restaurant-listing video
    Deliveroo,
    /// Grubhub restaurant-listing video
    Grubhub,
    /// Swiggy restaurant-listing video (IN)
    Swiggy,
    /// Zomato restaurant-listing video (IN/global)
    Zomato,
    /// Meituan restaurant-listing video (CN)
    Meituan,
    /// Robinhood fintech promo video (US)
    Robinhood,
    /// eToro social-trading promo video (global)
    Etoro,
    /// Webull trading-platform promo video (US/CN)
    Webull,
    /// Coinbase crypto promo video (US/global)
    Coinbase,
    /// Binance crypto promo video (global)
    Binance,
    /// Kraken crypto promo video (US/EU)
    Kraken,
    /// Public.com investing promo video (US)
    Public,
    /// OkCupid dating profile video (global)
    Okcupid,
    /// Match.com dating profile video (global)
    Match,
    /// Grindr dating profile video (global)
    Grindr,
    /// eHarmony dating profile video (global)
    Eharmony,
    /// Zoosk dating profile video (global)
    Zoosk,
    /// Badoo dating profile video (EU/global)
    Badoo,
    /// Plenty of Fish dating profile video (US/global)
    Pof,
    /// Medium article-embedded video (global)
    Medium,
    /// Ghost publication video (self-hosted publishing)
    Ghost,
    /// WordPress post-embedded video (global CMS)
    Wordpress,
    /// Squarespace site-embedded video (global)
    Squarespace,
    /// Wix site-embedded video (global)
    Wix,
    /// Webflow site-embedded video (global)
    Webflow,
    /// Framer site/prototype video (global)
    Framer,
    /// Newgrounds — animation/game portal uploads 16:9
    Newgrounds,
    /// DeviantArt — art community video posts 16:9
    Deviantart,
    /// VSCO — creator community video posts 16:9
    Vsco,
    /// SmugMug — photo/video portfolio hosting 16:9
    Smugmug,
    /// Zenfolio — photographer hosting with video 16:9
    Zenfolio,
    /// 9Now — Australian Nine catch-up OTT 16:9
    #[value(name = "9now")]
    Ninenow,
    /// 7plus — Australian Seven OTT 16:9
    #[value(name = "7plus")]
    Sevenplus,
    /// Pluto TV FAST channel clips (global)
    Plutotv,
    /// Amazon Freevee AVOD clips (US)
    Freevee,
    /// fuboTV sports-streaming clips (US)
    Fubotv,
    /// Globo/Globoplay promo video (BR)
    Globo,
    /// PBS Kids clips/promos (US)
    Pbskids,
    /// Boomerang classic-cartoon clips (global)
    Boomerang,
    /// Cartoonito preschool-block clips (global)
    Cartoonito,
    /// DraftKings sportsbook promo video (US)
    Draftkings,
    /// FanDuel sportsbook promo video (US)
    Fanduel,
    /// bet365 sportsbook promo video (global)
    Bet365,
    /// William Hill sportsbook promo video (UK)
    Williamhill,
    /// Betfair betting-exchange promo video (UK)
    Betfair,
    /// Sky Bet promo video (UK)
    Skybet,
    /// Paddy Power promo video (IE/UK)
    Paddypower,
    /// PGA Tour golf highlights (global)
    Pga,
    /// ATP tennis tour highlights (global)
    Atp,
    /// WTA tennis tour highlights (global)
    Wta,
    /// ICC cricket highlights (global)
    Icc,
    /// Formula 1 highlights (global)
    F1,
    /// MotoGP motorcycle-racing highlights (global)
    Motogp,
    /// NASCAR stock-car highlights (US)
    Nascar,
    /// Orange TV telecom-OTT clips (France/Poland set-top)
    Orange,
    /// SFR telecom-OTT clips (France set-top)
    Sfr,
    /// Free/Freebox telecom-OTT clips (France set-top)
    Free,
    /// Proximus Pickx telecom-OTT clips (Belgium set-top)
    Proximus,
    /// Swisscom TV telecom-OTT clips (Switzerland set-top)
    Swisscom,
    /// Telstra TV telecom-OTT clips (Australia set-top)
    Telstra,
    /// KPN iTV telecom-OTT clips (Netherlands set-top)
    Kpn,
    /// Indeed job-board listing videos (employer-brand posts)
    Indeed,
    /// Glassdoor employer-profile video uploads
    Glassdoor,
    /// ZipRecruiter job-listing video posts
    Ziprecruiter,
    /// SEEK job-board listing clips (AU/NZ)
    Seek,
    /// Monster job-listing video uploads
    Monster,
    /// Naukri recruiter video posts (India job board)
    Naukri,
    /// Apna job-card video posts (India blue-collar)
    Apna,
    /// App Store app-preview videos (iOS listing)
    Appstore,
    /// Google Play app-listing promo videos
    Googleplay,
    /// TestFlight beta-marketing clips
    Testflight,
    /// APKPure store listing videos
    Apkpure,
    /// Samsung Galaxy Store listing videos
    Galaxystore,
    /// Huawei AppGallery listing videos
    Appgallery,
    /// F-Droid repo listing videos
    Fdroid,
    /// Epic Games Store trailer uploads
    Epic,
    /// GOG store listing videos
    Gog,
    /// Battle.net launcher feature videos
    Battlenet,
    /// Xbox Store game trailers
    Xbox,
    /// PlayStation Store game trailers
    Playstation,
    /// Nintendo eShop listing videos
    Nintendo,
    /// EA app store trailers
    Ea,
    /// OpenSea NFT listing videos
    Opensea,
    /// Rarible NFT listing videos
    Rarible,
    /// Foundation NFT listing videos
    Foundation,
    /// Zora NFT listing videos
    Zora,
    /// SuperRare NFT listing videos
    Superrare,
    /// MakersPlace NFT listing videos
    Makersplace,
    /// Objkt NFT listing videos
    Objkt,
    /// Google Ads / YouTube ad creatives
    Googleads,
    /// Meta (Facebook/Instagram) feed video ads
    Metaads,
    /// TikTok ad creatives
    Tiktokads,
    /// Snapchat ad creatives
    Snapads,
    /// Amazon sponsored-brand video ads
    Amazonads,
    /// Pinterest ad creatives
    Pinterestads,
    /// LinkedIn video ad creatives
    Linkedinads,
    /// Libsyn podcast host video uploads
    Libsyn,
    /// Megaphone podcast host video uploads
    Megaphone,
    /// Simplecast podcast host video uploads
    Simplecast,
    /// Fireside podcast host video uploads
    Fireside,
    /// Blubrry podcast host video uploads
    Blubrry,
    /// Audioboom podcast host video uploads
    Audioboom,
    /// Omny Studio podcast host video uploads
    Omny,
    /// Google Classroom lesson video posts
    Googleclassroom,
    /// Moodle LMS course video uploads
    Moodle,
    /// Blackboard Learn course video uploads
    Blackboard,
    /// Canvas LMS course video uploads
    Canvaslms,
    /// Schoology course video uploads
    Schoology,
    /// Seesaw classroom portfolio videos
    Seesaw,
    /// ClassDojo classroom story videos
    Classdojo,
    /// Echo360 lecture capture video uploads
    Echo360,
    /// Mediasite lecture capture video uploads
    Mediasite,
    /// YuJa enterprise video uploads
    Yuja,
    /// Warpwire education video uploads
    Warpwire,
    /// Ensemble Video education uploads
    Ensemblevideo,
    /// Edpuzzle interactive lesson videos
    Edpuzzle,
    /// PlayPosit interactive lesson videos
    Playposit,
    /// Frame.io review-and-approve video uploads
    Frameio,
    /// Wipster video review uploads
    Wipster,
    /// Filestage creative review uploads
    Filestage,
    /// Ziflow marketing asset review uploads
    Ziflow,
    /// iconik media-asset-manager uploads
    Iconik,
    /// Latakoo broadcast field-upload video
    Latakoo,
    /// Wiredrive creative-showcase video
    Wiredrive,
    /// Notion doc-embedded video
    Notion,
    /// Confluence wiki-embedded video
    Confluence,
    /// Coda doc-embedded video
    Coda,
    /// Miro board video
    Miro,
    /// Figma prototype/showcase video
    Figma,
    /// Canva design-platform video
    Canva,
    /// Internet Archive (archive.org) uploads
    Archiveorg,
    /// Zendesk help-article / ticket video
    Zendesk,
    /// Freshdesk support-portal video
    Freshdesk,
    /// Intercom messenger / article video
    Intercom,
    /// Help Scout docs video
    Helpscout,
    /// Zoho Desk help-center video
    Zohodesk,
    /// Kayako helpdesk video
    Kayako,
    /// Crisp chat / helpdesk video
    Crisp,
    /// UnitedMasters artist video uploads
    Unitedmasters,
    /// Anghami MENA streaming artist uploads
    Anghami,
    /// JioSaavn India streaming artist uploads
    Jiosaavn,
    /// Gaana India streaming artist uploads
    Gaana,
    /// Wynk India streaming artist uploads
    Wynk,
    /// NetEase Cloud Music artist uploads
    Netease,
    /// QQ Music artist uploads
    Qqmusic,
    /// Kugou Music artist uploads
    Kugou,
    /// Zenodo research-data uploads (video abstracts, data supplements
    /// with DOI archiving)
    Zenodo,
    /// Figshare research figure/poster video uploads
    Figshare,
    /// JoVE visualized-experiments video journal submissions
    Jove,
    /// SlideShare deck-attached video uploads
    Slideshare,
    /// SpeakerDeck talk/deck video uploads
    Speakerdeck,
    /// Instructables maker project video
    Instructables,
    /// Hackster.io hardware/maker project video
    Hackster,
    /// Thingiverse maker/design video
    Thingiverse,
}

#[derive(clap::Args, Debug)]
pub struct TrailArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Trail length in frames (2-16, echo mode)
    #[arg(long, default_value_t = 8)]
    pub frames: u32,
    /// echo = motion smear (tmix), light = bright-pixel persistence (lagfun)
    #[arg(long, value_enum, default_value_t = TrailMode::Echo)]
    pub mode: TrailMode,
    /// Bright-trail persistence 0.5-0.99 (light mode)
    #[arg(long, default_value_t = 0.95)]
    pub decay: f64,
    /// Apply the trail inside this window — comma list ok (echo mode)
    #[arg(long)]
    pub at: Option<String>,
    /// Window seconds (needs --at)
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum TrailMode {
    Echo,
    Light,
    /// tblend difference — only the moving pixels survive (motion ghost)
    Diff,
}

#[derive(clap::Args, Debug)]
pub struct GlitchArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Glitch intensity 0.5-20 (channel shift px + noise)
    #[arg(long, default_value_t = 3.0)]
    pub strength: f64,
    /// Engine: shift (default, rgb offset + noise) | planes — channel rotation
    /// swap (psychedelic false-color, no noise)
    #[arg(long, value_enum)]
    pub engine: Option<GlitchEngine>,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum GlitchEngine {
    /// rgbashift + noise — classic datamosh glitch
    Shift,
    /// shuffleplanes channel rotation — false-color acid look
    Planes,
    /// swapuv — U/V chroma swap (magenta↔green flip, weird-Terry look)
    Swapuv,
    /// stutter — shuffleframes: repeats every 4th frame → VHS-style stutter
    Stutter,
    /// pixels — shufflepixels block scatter (digital corruption bursts)
    Pixels,
    /// swaprect — swaps frame quadrants (surreal mirror-shuffle)
    Swaprect,
    /// random — frame-order scramble within a rolling cache (digital chaos;
    /// --strength scales the cache depth 2..200)
    Random,
}

#[derive(clap::Args, Debug)]
pub struct BarsArgs {
    #[arg(short, long)]
    pub output: PathBuf,
    /// Canvas WxH
    #[arg(long, default_value = "1920x1080")]
    pub size: String,
    /// Card length in seconds
    #[arg(long, default_value_t = 2.0)]
    pub dur: f64,
    /// HD bars (smptehdbars) instead of SD smptebars
    #[arg(long, default_value_t = true)]
    pub hd: bool,
    /// Test-card generator: hd (SMPTE HD) | sd (SMPTE SD) | pal100 | pal75 |
    /// rgb | yuv (wins over --hd when set)
    #[arg(long, value_enum)]
    pub kind: Option<BarKind>,
    /// Add a 1kHz tone bed
    #[arg(long, default_value_t = true)]
    pub tone: bool,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
pub enum BarKind {
    /// SMPTE HD bars (smptehdbars) — default card
    Hd,
    /// SMPTE SD bars (smptebars)
    Sd,
    /// PAL 100% bars (pal100bars)
    Pal100,
    /// PAL 75% bars (pal75bars)
    Pal75,
    /// RGB test pattern (rgbtestsrc)
    Rgb,
    /// YUV test pattern (yuvtestsrc)
    Yuv,
    /// Every RGB color sweep (allrgb) — encoder color-bleed QC card
    Allrgb,
    /// Every YUV color sweep (allyuv) — chroma subsample QC card
    Allyuv,
    /// mptestsrc encoder-torture pattern — cycling fine-detail/ringing
    /// torture zones; stress-test your codec preset before a master run
    Mptest,
    /// testsrc2 all-in-one calibration card — moving elements, color chips,
    /// circles, contrast ramps (the broadcast "please verify everything" card)
    Testsrc,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
pub enum ScopeMode {
    /// Color vectorscope
    Vector,
    /// Luma/RGB waveform monitor
    Wave,
    /// Temporal histogram — luma distribution rolling over time
    Hist,
    /// codecview motion vectors — macroblock arrows overlay (compression QC:
    /// panning shots should show coherent arrows; noise means jittery bits)
    Mvs,
    /// datascope — hex pixel values around --x/--y (full-frame readout: find
    /// the exact luma at the logo edge, verify a clipped highlight value)
    Data,
    /// qp — per-macroblock quantization overlay (compression QC: uniform
    /// blocks = clean encode, speckled blocks = starved bitrate)
    Qp,
    /// pixscope — magnified pixel-grid window at --x/--y
    Pix,
    /// osc — oscilloscope XY plot of the video signal (broadcast-style
    /// waveform XY; diagonal spread = luma range coverage)
    Osc,
    /// drift — drawgraph luma-drift curve in the corner (exposure-ramp QC:
    /// flat line = constant exposure, slope = gradual ramp/flicker source)
    Drift,
    /// loud — adrawgraph loudness-over-time curve (ebur128 momentary LUFS;
    /// podcast/voice QC: dips = quiet stretches, flat-top = clipping drive)
    Loud,
    /// cie — CIE 1931 chromaticity map (gamut QC: pixels plotted on the
    /// horseshoe diagram vs the Rec.709 triangle — out-of-gamut spills past)
    Cie,
    /// palette — frame's color palette swatch grid (GIF/8-bit QC: see the
    /// actual palette the encoder picked; palette-source review)
    Palette,
    /// graphmonitor — live filtergraph stats card (frames in/out, queue,
    /// pts) for encode-pipeline debugging
    Graph,
    /// safe — broadcast safe-area guides drawn full-frame (90% action-safe
    /// yellow, 80% title-safe red, center cross) — composition QC overlay
    Safe,
}

#[derive(clap::Args, Debug)]
pub struct ScopeArgs {
    pub input: PathBuf,
    /// Sample point X for --mode data (default: frame centre)
    #[arg(long)]
    pub x: Option<u32>,
    /// Sample point Y for --mode data (default: frame centre)
    #[arg(long)]
    pub y: Option<u32>,
    #[arg(short, long)]
    pub output: PathBuf,
    #[arg(long, value_enum, default_value_t = ScopeMode::Vector)]
    pub mode: ScopeMode,
    /// Scope box as a fraction of frame width (0.1-0.6)
    #[arg(long, default_value_t = 0.25)]
    pub size: f64,
    /// Corner: bottom-right | top-right | bottom-left | top-left
    #[arg(long, default_value = "bottom-right")]
    pub position: String,
    /// Only show inside window(s); comma list, 'end' = tail
    #[arg(long)]
    pub at: Option<String>,
    /// Window length in seconds (required with --at)
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(clap::Args, Debug)]
pub struct DesqueezeArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Anamorphic lens factor (1.33, 1.5, 1.8, 2.0)
    #[arg(long, default_value_t = 1.33)]
    pub factor: f64,
    /// Stretch axis: y (vertical, classic anamorphic) | x
    #[arg(long, default_value = "y")]
    pub axis: String,
}

#[derive(clap::Args, Debug)]
pub struct SolarizeArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Luma threshold 0-255; pixels above invert
    #[arg(long, default_value_t = 128)]
    pub threshold: u32,
    /// Only apply inside window(s); comma list, 'end' = tail
    #[arg(long)]
    pub at: Option<String>,
    /// Window length in seconds (required with --at)
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(clap::Args, Debug)]
pub struct PulseArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Breaths per second (zoom cycles)
    #[arg(long, default_value_t = 0.5)]
    pub rate: f64,
    /// Zoom amplitude 0.005-0.3
    #[arg(long, default_value_t = 0.05)]
    pub depth: f64,
    /// Only apply inside window(s); comma list, 'end' = tail
    #[arg(long)]
    pub at: Option<String>,
    /// Window length in seconds (required with --at)
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(clap::Args, Debug)]
pub struct DeflickerArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Temporal averaging window in frames (3-129)
    #[arg(long, default_value_t = 5)]
    pub size: u32,
    /// Engine: am (default, arithmetic-mean window) | tmide — temporal midway
    /// histogram equalization: stronger for timelapse/strobe-source flicker
    #[arg(long, value_enum)]
    pub engine: Option<DeflickerEngine>,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum DeflickerEngine {
    /// deflicker — local arithmetic-mean window
    Am,
    /// tmidequalizer — temporal histogram midpoint (harder flicker flattening)
    Tmide,
}

#[derive(clap::Args, Debug)]
pub struct EmbossArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Mix with source 0-1 (1 = full relief)
    #[arg(long, default_value_t = 1.0)]
    pub amount: f64,
    /// Only apply inside window(s); comma list, 'end' = tail
    #[arg(long)]
    pub at: Option<String>,
    /// Window length in seconds (required with --at)
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(clap::Args, Debug)]
pub struct TiltArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Sharp middle band as fraction of height (0.1-0.45)
    #[arg(long, default_value_t = 0.3)]
    pub band: f64,
    /// Blur strength for the outer strips (sigma 1-40)
    #[arg(long, default_value_t = 8.0)]
    pub blur: f64,
    /// Only apply inside window(s); comma list, 'end' = tail
    #[arg(long)]
    pub at: Option<String>,
    /// Window length in seconds (required with --at)
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(clap::Args, Debug)]
pub struct SwayArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Sway cycles per second
    #[arg(long, default_value_t = 0.4)]
    pub rate: f64,
    /// Max pixel drift (2-80)
    #[arg(long, default_value_t = 10)]
    pub px: u32,
    /// Only apply inside window(s); comma list, 'end' = tail
    #[arg(long)]
    pub at: Option<String>,
    /// Window length in seconds (required with --at)
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(clap::Args, Debug)]
pub struct RackArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Focus cycles per second
    #[arg(long, default_value_t = 0.25)]
    pub rate: f64,
    /// Max blur sigma (2-40)
    #[arg(long, default_value_t = 12.0)]
    pub blur: f64,
}

#[derive(clap::Args, Debug)]
pub struct OutlineArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Edge strength 0-1 (lower = inker)
    #[arg(long, default_value_t = 0.5)]
    pub strength: f64,
    /// Only apply inside window(s); comma list, 'end' = tail
    #[arg(long)]
    pub at: Option<String>,
    /// Window length in seconds (required with --at)
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(clap::Args, Debug)]
pub struct NightArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Grain level 0-30
    #[arg(long, default_value_t = 8.0)]
    pub grain: f64,
    /// Only apply inside window(s); comma list, 'end' = tail
    #[arg(long)]
    pub at: Option<String>,
    /// Window length in seconds (required with --at)
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(clap::Args, Debug)]
pub struct SnowArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Speckle density 5-200
    #[arg(long, default_value_t = 40)]
    pub density: u32,
    /// Fall speed px/sec
    #[arg(long, default_value_t = 60)]
    pub speed: u32,
    /// Only apply inside window(s); comma list, 'end' = tail
    #[arg(long)]
    pub at: Option<String>,
    /// Window length in seconds (required with --at)
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(clap::Args, Debug)]
pub struct ImpactArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Moment of the hit in seconds (default 0.5)
    #[arg(long)]
    pub at: Option<f64>,
    /// Shake amplitude 4-80 px
    #[arg(long, default_value_t = 20)]
    pub amp: u32,
    /// White flash seconds 0.02-0.5
    #[arg(long, default_value_t = 0.08)]
    pub flash: f64,
}

#[derive(clap::Args, Debug)]
pub struct WaveArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Horizontal shift 1-60 px
    #[arg(long, default_value_t = 8)]
    pub amp: u32,
    /// Undulation cycles/sec 0.1-10
    #[arg(long, default_value_t = 1.5)]
    pub speed: f64,
    /// Only apply inside window(s); comma list, 'end' = tail
    #[arg(long)]
    pub at: Option<String>,
    /// Window length in seconds (required with --at)
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(clap::Args, Debug)]
pub struct SpinArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Swing amplitude in degrees 0.5-45
    #[arg(long, default_value_t = 8.0)]
    pub deg: f64,
    /// Swings per second 0.05-10
    #[arg(long, default_value_t = 0.7)]
    pub rate: f64,
    /// Only apply inside window(s); comma list, 'end' = tail
    #[arg(long)]
    pub at: Option<String>,
    /// Window length in seconds (required with --at)
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(clap::Args, Debug)]
pub struct ThumpArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Moment of the drop in seconds (default 0.5)
    #[arg(long)]
    pub at: Option<f64>,
    /// Thump frequency 20-120 Hz
    #[arg(long, default_value_t = 55)]
    pub freq: u32,
    /// Thump level 0.05-1.0
    #[arg(long, default_value_t = 0.5)]
    pub gain: f64,
    /// Thump tail seconds
    #[arg(long, default_value_t = 0.6)]
    pub dur: f64,
}

#[derive(clap::Args, Debug)]
pub struct RiserArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Moment the rise lands on, seconds (default 0.5)
    #[arg(long)]
    pub at: Option<f64>,
    /// Rise length 0.2-10 s
    #[arg(long, default_value_t = 1.0)]
    pub dur: f64,
    /// Riser level 0.05-1.0
    #[arg(long, default_value_t = 0.4)]
    pub gain: f64,
}

#[derive(clap::Args, Debug)]
pub struct WhooshArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Moment the swell lands on, seconds (default 0.5)
    #[arg(long)]
    pub at: Option<f64>,
    /// Swell length 0.2-10 s
    #[arg(long, default_value_t = 1.0)]
    pub dur: f64,
    /// Swell level 0.05-1.0
    #[arg(long, default_value_t = 0.4)]
    pub gain: f64,
}

#[derive(clap::Args, Debug)]
pub struct EqualizeArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Equalization strength 0.05-1.0
    #[arg(long, default_value_t = 0.2)]
    pub strength: f64,
    /// Saturation-preserving intensity 0.05-1.0
    #[arg(long, default_value_t = 0.21)]
    pub intensity: f64,
    /// Only equalize inside window(s); comma list, 'end' = tail
    #[arg(long)]
    pub at: Option<String>,
    /// Window length per --at, seconds
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(clap::Args, Debug)]
pub struct DeesserArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Essing strength 0.05-1.0
    #[arg(long, default_value_t = 0.5)]
    pub amount: f64,
    /// Sibilance band position 0.2-0.9 (0.5 ≈ 4-8kHz)
    #[arg(long, default_value_t = 0.5)]
    pub freq: f64,
    /// Only de-ess inside window(s); comma list, 'end' = tail
    #[arg(long)]
    pub at: Option<String>,
    /// Window length per --at, seconds
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(clap::Args, Debug)]
pub struct DeclipArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Repair engine: clip (blown-out peaks, default) | click (vinyl pops,
    /// mouth clicks, digital dropouts)
    #[arg(long, value_enum)]
    pub engine: Option<DeclipEngine>,
    /// Analysis window in ms (10-100; bigger = smoother repair on music)
    #[arg(long, default_value_t = 55.0)]
    pub window: f64,
    /// Clip threshold fraction 1-100 (lower rescues harsher clipping)
    #[arg(long, default_value_t = 10.0)]
    pub threshold: f64,
    /// Use overlap-save instead of overlap-add (slightly better on noise)
    #[arg(long)]
    pub overlap_save: bool,
    /// Repair only inside this window — comma list for several (needs --dur)
    #[arg(long)]
    pub at: Option<String>,
    /// Window length in seconds (default: to the end)
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(clap::Args, Debug)]
pub struct DebandArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Smoothing strength 0.05-1.0
    #[arg(long, default_value_t = 0.5)]
    pub strength: f64,
    /// Neighborhood radius 4-32 px
    #[arg(long, default_value_t = 16)]
    pub radius: i32,
    /// Only de-band inside window(s); comma list, 'end' = tail
    #[arg(long)]
    pub at: Option<String>,
    /// Window length per --at, seconds
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(clap::Args, Debug)]
pub struct DedupArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Changed-pixel fraction needed to keep a frame 0.01-1.0
    #[arg(long, default_value_t = 0.33)]
    pub frac: f64,
}

#[derive(clap::Args, Debug)]
pub struct RepairArgs {
    /// Clip with bad/glitched frames
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Reference take to borrow a clean frame from
    #[arg(long = "ref")]
    pub ref_file: PathBuf,
    /// Start of the damaged stretch (seconds or mm:ss; 'end' = last frame)
    #[arg(long)]
    pub at: String,
    /// Length of the damaged stretch in seconds (default: one frame)
    #[arg(long)]
    pub dur: Option<f64>,
    /// Which frame of --ref to paste in (seconds; default: same as --at)
    #[arg(long)]
    pub ref_at: Option<String>,
}

#[derive(clap::Args, Debug)]
pub struct IrisArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Circle center X as % of width
    #[arg(long, default_value_t = 50.0)]
    pub x: f64,
    /// Circle center Y as % of height
    #[arg(long, default_value_t = 50.0)]
    pub y: f64,
    /// Circle radius as % of width 2-100
    #[arg(long, default_value_t = 30)]
    pub radius: u32,
    /// Only apply inside window(s); comma list, 'end' = tail
    #[arg(long)]
    pub at: Option<String>,
    /// Window length in seconds (required with --at)
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(clap::Args, Debug)]
pub struct BurstArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Smear opacity 0.05-0.9
    #[arg(long, default_value_t = 0.35)]
    pub strength: f64,
}

#[derive(clap::Args, Debug)]
pub struct PickArgs {
    pub input: PathBuf,
    /// Timestamp to sample (default: midpoint)
    #[arg(long)]
    pub at: Option<f64>,
}

#[derive(clap::Args, Debug)]
pub struct DiffArgs {
    /// First clip
    pub a: PathBuf,
    /// Second clip
    pub b: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Show second clip beside the diff
    #[arg(long)]
    pub side: bool,
    /// Diff mode: blend (amplified difference, default) | mask (only the
    /// pixels that changed — maskedthreshold change mask)
    #[arg(long, value_enum)]
    pub mode: Option<DiffMode>,
    /// Change threshold 0..1 for --mode mask (default 0.1)
    #[arg(long)]
    pub threshold: Option<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum DiffMode {
    /// Amplified difference view (blend=difference + gamma boost)
    Blend,
    /// Bare change mask: only pixels whose diff exceeds --threshold
    Mask,
}

#[derive(clap::Args, Debug)]
pub struct SelectiveArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Color to keep (name or #hex)
    #[arg(long)]
    pub color: String,
    /// Match tolerance 0.01-0.7
    #[arg(long, default_value_t = 0.4)]
    pub similarity: f64,
    /// Edge softness 0-1 (feather the kept color into the gray field)
    #[arg(long, default_value_t = 0.0)]
    pub blend: f64,
    /// Hold engine — chroma (YUV chroma-space hold) tracks saturated
    /// hues better than the RGB colorhold on uneven subjects
    #[arg(long, value_enum)]
    pub engine: Option<SelectiveEngine>,
    /// Only apply inside window(s); comma list, 'end' = tail
    #[arg(long)]
    pub at: Option<String>,
    /// Window length in seconds (required with --at)
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub enum SelectiveEngine {
    /// colorhold — RGB-space keep-color (default)
    Color,
    /// chromahold — YUV chroma-space keep-color
    Chroma,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum KeyMode {
    /// Chroma key — remove --color (green/blue screen)
    Color,
    /// Luma key — remove a brightness band around --threshold
    Luma,
    /// External matte — copy --mask's luma into the alpha channel
    /// (alphamerge; outputs prores 4444 with alpha)
    Matte,
    /// Chroma key — chromakey, the YUV-domain broadcast keyer: handles
    /// gradient/wrinkled screens better than RGB colorkey on real footage
    Chroma,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum ChromaEdge {
    /// Wrap chroma around the shifted edge (cleanest for corrections)
    Wrap,
    /// Smear edge chroma outward
    Smear,
}

#[derive(clap::Args, Debug)]
pub struct DeblockArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Detection strength 0.05-0.95 (0.5 balanced; heavy blocking needs 0.8+)
    #[arg(long, default_value_t = 0.5)]
    pub strength: f64,
    /// Only apply inside window(s); comma list, 'end' = tail
    #[arg(long)]
    pub at: Option<String>,
    /// Window length in seconds (required with --at)
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(clap::Args, Debug)]
pub struct ChromashiftArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Horizontal chroma shift in px -255..255 (tape halo is usually 1-8)
    #[arg(long, allow_hyphen_values = true, default_value_t = 0)]
    pub x: i32,
    /// Vertical chroma shift in px -255..255
    #[arg(long, allow_hyphen_values = true, default_value_t = 0)]
    pub y: i32,
    /// Edge handling: wrap (default) | smear
    #[arg(long, value_enum, default_value_t = ChromaEdge::Wrap)]
    pub edge: ChromaEdge,
    /// Only apply inside window(s); comma list, 'end' = tail
    #[arg(long)]
    pub at: Option<String>,
    /// Window length in seconds (required with --at)
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(clap::Args, Debug)]
pub struct TmedianArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Frames of temporal history each side (1-127; bigger removes longer
    /// intrusions, drops 2*radius output frames)
    #[arg(long, default_value_t = 15)]
    pub radius: u32,
    /// Percentile 0-1 (0.5 = median; lower also darkens, higher brightens)
    #[arg(long, default_value_t = 0.5)]
    pub percentile: f64,
    /// Only apply inside window(s); comma list, 'end' = tail
    #[arg(long)]
    pub at: Option<String>,
    /// Window length in seconds (required with --at)
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(clap::Args, Debug)]
pub struct StereoArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Packed 3D input format (default sbsl — side-by-side left first):
    /// sbsl/sbsr/sbs2l/sbs2r/abl/abr/ab2l/ab2r/tbl/tbr/tb2l/tb2r/al/ar/irl/irr/icl/icr
    #[arg(long = "in", default_value = "sbsl")]
    pub in_format: String,
    /// Output format (default arcd — anaglyph red/cyan dubois):
    /// all of the above + anaglyphs arcg/arch/arcc/arcd/arbg/agmg/agmh/agmc/agmd
    #[arg(long = "out", default_value = "arcd")]
    pub out_format: String,
}

#[derive(clap::Args, Debug)]
pub struct StackArgs {
    /// Locked-off inputs of the same scene (3+, same WxH — conform first)
    #[arg(required = true)]
    pub inputs: Vec<PathBuf>,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Percentile 0-1 across inputs (0.5 = median; lower darkens, higher brightens)
    #[arg(long, default_value_t = 0.5)]
    pub percentile: f64,
    /// Combine mode: median (default, object removal), max (star/light
    /// trails), min (noise floor / darkest composite), mean (weighted
    /// average — noise stacking / double exposure)
    #[arg(long, value_enum, default_value_t = StackMode::Median)]
    pub mode: StackMode,
    /// Per-input weights for --mode mean (comma list, count must match
    /// inputs; auto-normalized). Default: equal weights.
    #[arg(long)]
    pub weights: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum StackMode {
    #[default]
    Median,
    Max,
    Min,
    Mean,
}

#[derive(clap::Args, Debug)]
pub struct AmplifyArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Magnification factor 1-50 (default 8: gentle; 20+ = microscope motion)
    #[arg(long, default_value_t = 8.0)]
    pub amount: f64,
    /// Frames of history the diff accumulates over (1-63; more = smoother)
    #[arg(long, default_value_t = 3)]
    pub radius: u32,
    /// Pixel diffs below this get magnified (0-255; higher = bigger motion included)
    #[arg(long, default_value_t = 30)]
    pub threshold: u32,
    /// Only apply inside window(s); comma list, 'end' = tail
    #[arg(long)]
    pub at: Option<String>,
    /// Window length in seconds (required with --at)
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(clap::Args, Debug)]
pub struct CartoonArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Posterize levels 2-16 (fewer = chunkier color blocks)
    #[arg(long, default_value_t = 6)]
    pub levels: u32,
    /// Only apply inside window(s); comma list, 'end' = tail
    #[arg(long)]
    pub at: Option<String>,
    /// Window length in seconds (required with --at)
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
pub enum HeatPreset {
    Magma,
    Inferno,
    Plasma,
    Viridis,
    Turbo,
    Cividis,
    Range1,
    Range2,
    Shadows,
    Highlights,
}

#[derive(clap::Args, Debug)]
pub struct HeatArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    #[arg(long, value_enum, default_value_t = HeatPreset::Inferno)]
    pub preset: HeatPreset,
    /// Blend strength 0-1
    #[arg(long, default_value_t = 1.0)]
    pub opacity: f64,
    /// Only apply inside window(s); comma list, 'end' = tail
    #[arg(long)]
    pub at: Option<String>,
    /// Window length in seconds (required with --at)
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(clap::Args, Debug)]
pub struct KaleidoArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Only apply inside window(s); comma list, 'end' = tail
    #[arg(long)]
    pub at: Option<String>,
    /// Window length in seconds (required with --at)
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum MirrorAxis {
    /// Left half mirrored onto the right
    X,
    /// Top half mirrored onto the bottom
    Y,
}

#[derive(clap::Args, Debug)]
pub struct ShearArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Horizontal shear -2..2 (rows slant; 0.3 ≈ strong italic)
    #[arg(long, allow_hyphen_values = true, default_value_t = 0.0)]
    pub x: f64,
    /// Vertical shear -2..2 (columns slant)
    #[arg(long, allow_hyphen_values = true, default_value_t = 0.0)]
    pub y: f64,
    /// Edge fill color for the vacated corners (default black)
    #[arg(long, default_value = "black")]
    pub fill: String,
    /// Interpolation: bilinear (default, smoother) | nearest (retro staircase)
    #[arg(long, default_value = "bilinear")]
    pub interp: String,
    /// Timestamp(s) to start the slant — comma list allowed; `end` = tail
    #[arg(long)]
    pub at: Option<String>,
    /// Seconds the slant lasts per --at point (default: to end)
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(clap::Args, Debug)]
pub struct MirrorArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Mirror axis: x = left→right, y = top→bottom
    #[arg(long, value_enum, default_value_t = MirrorAxis::X)]
    pub axis: MirrorAxis,
    /// Timestamp(s) to start mirroring — comma list allowed; `end` = tail
    #[arg(long)]
    pub at: Option<String>,
    /// Seconds the mirror lasts per --at point (default: to end)
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(clap::Args, Debug)]
pub struct PixArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Pixel block divisor 2-64 (bigger = chunkier)
    #[arg(long, default_value_t = 8.0)]
    pub strength: f64,
    /// Timestamp(s) to start pixelating — comma list allowed; `end` = tail
    #[arg(long)]
    pub at: Option<String>,
    /// Seconds the pixelation lasts per --at point (default: to end)
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum FlipAxis {
    /// Horizontal flip (unmirror selfie footage)
    X,
    /// Vertical flip
    Y,
}

#[derive(clap::Args, Debug)]
pub struct FlipArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Flip axis: x = horizontal (unmirror), y = vertical
    #[arg(long, value_enum, default_value_t = FlipAxis::X)]
    pub axis: FlipAxis,
    /// Timestamp(s) to start flipping — comma list allowed; `end` = tail
    #[arg(long)]
    pub at: Option<String>,
    /// Seconds the flip lasts per --at point (default: to end)
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(clap::Args, Debug)]
pub struct PosterArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Palette colors to keep (2-64, lower = more posterized)
    #[arg(long, default_value_t = 8)]
    pub levels: u32,
    /// Timestamp(s) to start posterizing — comma list allowed; `end` = tail
    #[arg(long)]
    pub at: Option<String>,
    /// Seconds the poster look lasts per --at point (default: to end)
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(clap::Args, Debug)]
pub struct DuotoneArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Color for the dark end (name or RRGGBB)
    #[arg(long, default_value = "001a33")]
    pub shadow: String,
    /// Color for the bright end (name or RRGGBB)
    #[arg(long, default_value = "ffd699")]
    pub highlight: String,
    /// Timestamp(s) to start the duotone — comma list allowed; `end` = tail
    #[arg(long)]
    pub at: Option<String>,
    /// Seconds the duotone lasts per --at point (default: to end)
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(clap::Args, Debug)]
pub struct GlowArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Bloom radius (gblur sigma 0.5-40)
    #[arg(long, default_value_t = 6.0)]
    pub strength: f64,
    /// Timestamp(s) to start the glow — comma list allowed; `end` = tail
    #[arg(long)]
    pub at: Option<String>,
    /// Seconds the glow lasts per --at point (default: to end)
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(clap::Args, Debug)]
pub struct VhsArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Tape damage intensity 0-3 (noise + chroma shift + scanlines)
    #[arg(long, default_value_t = 1.0)]
    pub strength: f64,
    /// Timestamp(s) to start the VHS look — comma list allowed; `end` = tail
    #[arg(long)]
    pub at: Option<String>,
    /// Seconds the VHS look lasts per --at point (default: to end)
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(clap::Args, Debug)]
pub struct MotionBlurArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Frames blended together 2-8 (2 = true shutter smear, more = ghost trail)
    #[arg(long, default_value_t = 2)]
    pub frames: u32,
    /// Timestamp(s) to start the blur — comma list allowed; `end` = tail
    #[arg(long)]
    pub at: Option<String>,
    /// Seconds the blur lasts per --at point (default: to end)
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(clap::Args, Debug)]
pub struct StrobeArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Flashes per second
    #[arg(long, default_value_t = 4.0)]
    pub rate: f64,
    /// Fraction of each period the flash is on (0.05-0.8)
    #[arg(long, default_value_t = 0.25)]
    pub duty: f64,
    /// Flash color (name or hex)
    #[arg(long, default_value = "white")]
    pub color: String,
    /// Only flash inside window(s); comma list, 'end' = tail
    #[arg(long)]
    pub at: Option<String>,
    /// Window length in seconds (required with --at)
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
pub enum EdgeMode {
    /// White wireframe lines on black
    Wires,
    /// Neon colored outlines over the picture
    Colormix,
}

#[derive(clap::Args, Debug)]
pub struct EdgeArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    #[arg(long, value_enum, default_value_t = EdgeMode::Colormix)]
    pub mode: EdgeMode,
    /// Detector: edgedetect (default, honors --mode) or a classic kernel —
    /// sobel | kirsch | roberts | prewitt (cruder, crunchier look)
    #[arg(long, value_enum)]
    pub engine: Option<EdgeEngine>,
    /// Edge low threshold 0-1
    #[arg(long, default_value_t = 0.2)]
    pub low: f64,
    /// Edge high threshold 0-1
    #[arg(long, default_value_t = 0.4)]
    pub high: f64,
    /// Only apply inside window(s); comma list, 'end' = tail
    #[arg(long)]
    pub at: Option<String>,
    /// Window length in seconds (required with --at)
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
pub enum EdgeEngine {
    Edgedetect,
    Sobel,
    Kirsch,
    Roberts,
    Prewitt,
    /// Canny-style linking: weak edges that touch strong ones survive,
    /// isolated specks drop (hysteresis) — clean connected line art
    Link,
}

#[derive(clap::Args, Debug)]
pub struct GenArgs {
    /// -o target (e.g. bg.mp4) — generative sources take no input file
    #[arg(short, long)]
    pub output: PathBuf,
    /// Pattern: mandelbrot | gradients | life (cellular automaton) | sierpinski | hald (identity LUT image — author your own `grade --lut` in an editor) | noise | tone | sweep | silence (audio beds / speaker-test chirp / silent padding)
    #[arg(long, default_value = "gradients")]
    pub pattern: String,
    /// Frame size WxH
    #[arg(long, default_value = "1920x1080")]
    pub size: String,
    /// Frame rate
    #[arg(long, default_value_t = 30.0)]
    pub fps: f64,
    /// Duration seconds
    #[arg(long, default_value_t = 5.0)]
    pub dur: f64,
    /// Random seed for gradients (reproducible palettes)
    #[arg(long, allow_hyphen_values = true)]
    pub seed: Option<i64>,
    /// Gradient colors, up to 8 (comma list: name or 0xRRGGBB)
    #[arg(long)]
    pub colors: Option<String>,
    /// Tone frequency Hz for --pattern tone (default 440)
    #[arg(long)]
    pub freq: Option<f64>,
    /// With --pattern noise: noise colour white|pink|brown|blue|violet|velvet (default pink)
    #[arg(long)]
    pub color: Option<String>,
    /// Gradient drift speed 0.001-1
    #[arg(long, default_value_t = 0.01)]
    pub speed: f64,
    /// Mandelbrot terminal zoom depth (end_scale; smaller = deeper)
    #[arg(long, default_value_t = 0.05)]
    pub zoom: f64,
    /// Life rule 0-255 (110 = classic glider-friendly)
    #[arg(long, default_value_t = 110)]
    pub rule: i64,
    /// HALD cube level for --pattern hald (3..12; 8 = standard 512x512)
    #[arg(long, default_value_t = 8)]
    pub level: u32,
}

#[derive(clap::Args, Debug)]
pub struct PerspectiveArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Quad corners in the source, px: "x0,y0,x1,y1,x2,y2,x3,y3" (TL,TR,BL,BR)
    #[arg(long, required = true)]
    pub points: String,
    /// Interpolation kernel: linear | cubic
    #[arg(long, default_value = "linear")]
    pub interp: String,
}

#[derive(clap::Args, Debug)]
pub struct LensArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Radial correction: negative = fisheye look, positive = defish
    #[arg(long, default_value_t = -0.2, allow_negative_numbers = true)]
    pub k1: f64,
    /// Secondary radial term
    #[arg(long, default_value_t = -0.05, allow_negative_numbers = true)]
    pub k2: f64,
    /// Only apply inside window(s); comma list, 'end' = tail
    #[arg(long)]
    pub at: Option<String>,
    /// Window length in seconds (required with --at)
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(clap::Args, Debug)]
pub struct SpeedArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Playback factor: 2 = twice as fast, 0.5 = slow-mo
    #[arg(long, required_unless_present_any = ["ramp", "fit"])]
    pub factor: Option<f64>,
    /// Retime to exactly SEC seconds (auto-picks the factor — a 90s take
    /// --fit 15 becomes 6x). Within the same 0.25..8 factor range.
    #[arg(long)]
    pub fit: Option<f64>,
    /// Linear speed ramp FROM,TO across the input (or --at/--dur window), e.g. 0.5,3
    #[arg(long)]
    pub ramp: Option<String>,
    /// Apply the factor only inside this window — comma list for several windows (needs --dur)
    #[arg(long)]
    pub at: Option<String>,
    /// Window length in seconds (default: to the end)
    #[arg(long)]
    pub dur: Option<f64>,
    /// Blend interpolated frames for smooth slow-mo (needs factor < 1)
    #[arg(long)]
    pub interp: bool,
}

#[derive(clap::Args, Debug)]
pub struct RotateArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Degrees clockwise: 90, 180, 270
    #[arg(long, default_value_t = 90)]
    pub deg: u32,
    /// Mirror instead of rotating: h or v
    #[arg(long, value_enum)]
    pub flip: Option<FlipMode>,
    /// Free rotation in degrees — dutch tilt (overrides --deg)
    #[arg(long)]
    pub angle: Option<f64>,
    /// Tilt only inside this window — comma list for several (needs --dur)
    #[arg(long)]
    pub at: Option<String>,
    /// Window length for --at (seconds; default = to the end)
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(clap::Args, Debug)]
pub struct DelogoArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Logo box left edge (px) — or use --regions for several
    #[arg(long)]
    pub x: Option<u32>,
    /// Logo box top edge (px) — or use --regions for several
    #[arg(long)]
    pub y: Option<u32>,
    /// Logo box width (px) — or use --regions for several
    #[arg(long)]
    pub w: Option<u32>,
    /// Logo box height (px) — or use --regions for several
    #[arg(long)]
    pub h: Option<u32>,
    /// Comma list of logo boxes `x:y:w:h` — covers several logos/spots in one pass
    #[arg(long)]
    pub regions: Option<String>,
    /// Blur the box only inside this window — comma list for several spots (needs --dur)
    #[arg(long)]
    pub at: Option<String>,
    /// Window length in seconds (default: to the end)
    #[arg(long)]
    pub dur: Option<f64>,
    /// Precise removal from a drawn mask image (white = remove): removelogo
    /// inpaints only where your bitmap says — better than box --soft when the
    /// logo isn't rectangular. Mask must be the video's size
    #[arg(long)]
    pub image: Option<PathBuf>,
    /// Auto-locate the logo from a reference bitmap (find_rect hunts it in
    /// the first 15s, then removes the found box) — no coordinates needed
    #[arg(long)]
    pub find: Option<PathBuf>,
    /// Follow a MOVING watermark: find_rect+cover_rect re-detects the ref
    /// every frame and blurs wherever it lands (needs --find; no --at window)
    #[arg(long)]
    pub track: bool,
    /// Feathered removal via removelogo mask instead of the hard delogo box
    #[arg(long)]
    pub soft: bool,
    /// Mask shape: box (default) or circle (ellipse via edge-interpolated mask)
    #[arg(long, value_enum, default_value_t = DelogoShape::Box)]
    pub shape: DelogoShape,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum DelogoShape {
    Box,
    Circle,
}

#[derive(clap::Args, Debug)]
pub struct SubsArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Subtitle stream index (0 = first)
    #[arg(long, default_value_t = 0)]
    pub stream: u32,
    /// Burn this subtitle file into the video instead of extracting (--file subs.srt)
    #[arg(long)]
    pub burn: Option<PathBuf>,
    /// With --burn omitted: burn the input's own subtitle stream N (multi-track files)
    #[arg(long)]
    pub burn_si: Option<u32>,
    /// With --burn: opaque plate behind each line (semi-black box style)
    #[arg(long = "box")]
    pub burn_box: bool,
    /// With --burn: line alignment left|center|right (default center)
    #[arg(long)]
    pub align: Option<String>,
    /// Rewrite cue text case: upper|lower|title (with --burn/--convert)
    #[arg(long, value_enum)]
    pub case: Option<TextCase>,
    /// Shift every cue of an .srt by ±N seconds (input = .srt, output = .srt)
    #[arg(long, allow_hyphen_values = true)]
    pub shift: Option<f64>,
    /// Decode a subtitle file in this charset (gbk/big5/sjis/latin1...) instead of UTF-8
    #[arg(long)]
    pub encoding: Option<String>,
    /// Burned subtitle font size (default 18)
    #[arg(long)]
    pub size: Option<f64>,
    /// Burned subtitle color (RRGGBB hex, default white)
    #[arg(long)]
    pub color: Option<String>,
    /// Burned subtitles on top instead of bottom
    #[arg(long)]
    pub top: bool,
    /// Burned subtitle font family name (default Sans)
    #[arg(long)]
    pub font: Option<String>,
    /// Mux this subtitle file into the video as selectable soft subs (.mp4/.mkv)
    #[arg(long)]
    pub mux: Option<PathBuf>,
    /// Language tag on the muxed subtitle stream (eng|spa|zho|…)
    #[arg(long)]
    pub lang: Option<String>,
    /// Push burned captions into the social-safe zone (bigger MarginV)
    #[arg(long)]
    pub safe: bool,
    /// With --burn: exact MarginV in px (overrides --safe's computed margin)
    #[arg(long)]
    pub margin: Option<u32>,
    /// With --burn: keep only cues overlapping [FROM,TO) — window start
    #[arg(long)]
    pub from: Option<String>,
    /// Window end (h:mm:ss or seconds; needs --from)
    #[arg(long)]
    pub to: Option<String>,
    /// Burned subtitle outline width in px (default 1)
    #[arg(long)]
    pub outline: Option<f64>,
    /// Burned subtitle drop-shadow depth 0-8 px (default 0)
    #[arg(long)]
    pub shadow: Option<f64>,
    /// Merge another .srt into the input .srt (dual-language; cues sorted by start)
    #[arg(long)]
    pub merge: Option<PathBuf>,
    /// Keep only cues whose text contains this string (case-insensitive) —
    /// locate every "um"/phrase spoken, then cut or caption around them
    #[arg(long)]
    pub find: Option<String>,
    /// Extract EVERY subtitle stream to stem_0.srt, stem_1.srt … (batch)
    #[arg(long)]
    pub all: bool,
    /// Rescale every cue time by this factor — 25→23.976 fps drift ≈ 0.959
    #[arg(long)]
    pub rate: Option<f64>,
    /// Frame rate for frame-based subtitle formats (.sub MicroDVD input
    /// declares times in frames — a {1}{1}fps header line wins, else this)
    #[arg(long)]
    pub fps: Option<f64>,
    /// Convert between subtitle formats (.srt ↔ .vtt) — input is the cue file
    #[arg(long)]
    pub convert: bool,
    /// Sort cues by start time and renumber (broken exports write cues
    /// out of order — players then show them late or not at all)
    #[arg(long)]
    pub sort: bool,
    /// Clamp each cue's end to the next cue's start (overlapping-cue
    /// repair for bad subs; pairs with --sort when times are scrambled)
    #[arg(long)]
    pub fix_overlaps: bool,
    /// Drop exact duplicate cues — same start/end/text (buggy exports
    /// repeat every cue; players flash the line twice)
    #[arg(long)]
    pub dedupe: bool,
    /// Drop a cue whose text repeats the previous kept cue — auto-transcript
    /// repeat-line artifacts (Whisper prints the same sentence twice across
    /// different timings; text compared case/space-insensitive; extras: dropped)
    #[arg(long)]
    pub dedupe_text: bool,
    /// Stretch each over-CPS cue until it reads at N chars/sec (auto-fix for
    /// the --cps gate — extends the cue's end, capped at the next cue's
    /// start; extras: stretched)
    #[arg(long, value_name = "CPS")]
    pub fix_cps: Option<f64>,
    /// Split a cue with more than N text lines into multiple cues (text
    /// split at line boundaries, duration divided evenly) — the auto-fix
    /// for the --max-lines gate (extras: lines_split)
    #[arg(long)]
    pub fix_lines: Option<u32>,
    /// Report unique speaker labels in the transcript (extras:
    /// speakers[] + speaker_count) — the cast list; pairs with
    /// --strip-speakers which removes them
    #[arg(long)]
    pub speakers: bool,
    /// Transcript stats report (extras: cues/words/chars/span_secs/
    /// median_dur_secs — readability & density QC without burning)
    #[arg(long)]
    pub stats: bool,
    /// Readability gate: report cues faster than N chars/sec
    /// (Netflix-style caption-speed spec; extras: over_limit/worst_cps)
    #[arg(long)]
    pub cps: Option<f64>,
    /// Extend cues shorter than SEC up to SEC (flash-text repair — the
    /// extension caps at the next cue's start so it can't re-overlap)
    #[arg(long)]
    pub min_dur: Option<f64>,
    /// Enforce at least SEC seconds of silence between adjacent cues —
    /// trim the earlier cue's tail when the gap is shorter (broadcast
    /// spec is ~2 frames; extras: gapped)
    #[arg(long)]
    pub min_gap: Option<f64>,
    /// Merge adjacent cues separated by less than SEC — auto-transcript
    /// over-fragmentation repair (Whisper-style fragments join into
    /// sentence-length cues; text joined with a space, span covers the
    /// merged range; extras: joined)
    #[arg(long)]
    pub join: Option<f64>,
    /// Report cues with more than N text lines (broadcast spec is 2)
    #[arg(long)]
    pub max_lines: Option<usize>,
    /// Find/replace inside cue text — `OLD,NEW` (rename a character or
    /// fix a typo across the whole file; extras: replaced)
    #[arg(long)]
    pub replace: Option<String>,
    /// Strip speaker labels from cue text — `[NAME]` / `<NAME>` /
    /// `ALL-CAPS NAME:` prefixes (auto-generated transcripts; extras: stripped)
    #[arg(long)]
    pub strip_speakers: bool,
    /// Strip inline markup from cue text — `<i>`/`<b>`/`<font …>` tags and
    /// `{\…}` ASS override blocks (transcripts that burn literal markup;
    /// extras: tags_stripped)
    #[arg(long)]
    pub strip_tags: bool,
    /// Strip SDH/HI annotations from cue text — `[SOUND]`/`(door creaks)`
    /// spans and `♪`/`♫` marks (burnable dialogue-only transcript;
    /// extras: sdh_stripped, empty cues drop)
    #[arg(long)]
    pub strip_sdh: bool,
    /// Strip emoji/pictograph characters from cue text — broadcast caption
    /// paths (608/708) and station ingest reject them, consumer decoders
    /// render tofu; burnable broadcast-safe transcript (extras:
    /// emotes_stripped, empty cues drop)
    #[arg(long)]
    pub strip_emotes: bool,
    /// Wrap every cue-text line in U+202B..U+202C RTL marks — Arabic/Hebrew
    /// captions render mirrored punctuation in players without them
    /// (MENA subtitle delivery; extras: rtl_wrapped)
    #[arg(long)]
    pub rtl: bool,
    /// Keep only cues overlapping window `F,T` (`end` ok for T), clip the
    /// edges, and re-time the result to start at 0 — the `extract --audio
    /// --from/--to` counterpart for transcripts (grab the subtitle chunk
    /// for a cut segment; extras: clipped)
    #[arg(long)]
    pub clip: Option<String>,
    /// Drop cues overlapping window `F,T` and re-time the tail left by
    /// (T-F) — the `cut --drop` counterpart for transcripts (excise a
    /// segment AND its subtitles in one pass; `end` ok for T; a cue
    /// spanning the whole cut keeps its head; extras: excised)
    #[arg(long)]
    pub drop: Option<String>,
    /// Rewrap cue text at N chars per line (portrait-phone captions;
    /// extras: rewrapped)
    #[arg(long)]
    pub wrap: Option<usize>,
    /// Append another .srt file's cues, shifted to start where this
    /// file's cues end — join subtitle files after `concat` joins the
    /// matching clips (extras: appended, offset)
    #[arg(long)]
    pub append: Option<PathBuf>,
    /// Split cues at SECONDS into one .srt per segment (`-o part.srt` →
    /// part_0.srt/part_1.srt/...; comma list = several cuts) — the
    /// `split --at` counterpart for transcripts: cues are re-timed so
    /// each part starts at 0, a cue spanning a cut keeps its head in the
    /// earlier part
    #[arg(long)]
    pub split: Option<String>,
    /// Two-point resync `O1,O2,N1,N2` — remap cue times linearly so old
    /// timestamps O1,O2 land on new N1,N2 (offset + drift in one pass;
    /// retiming subs exported for a different cut/frame-rate — the
    /// `--shift`+`--rate` combined transform)
    #[arg(long)]
    pub resync: Option<String>,
    /// Re-seat cue N (1-based, the input file's numbering) to start at T —
    /// nudge one mis-timed cue without resyncing the file (duration kept)
    #[arg(long = "move")]
    pub cue_move: Option<String>,
    /// Snap every cue's start/end to the nearest frame boundary at --fps
    /// (default 25) — frame-accurate subtitles for broadcast/QC handoffs;
    /// cues shorter than half a frame grow to one frame (extras: snapped)
    #[arg(long)]
    pub snap: bool,
}

#[derive(clap::Args, Debug)]
pub struct ThumbArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Frame timestamp (default: 10% into the clip)
    #[arg(long)]
    pub at: Option<String>,
    /// Exact frame index instead of a timestamp
    #[arg(long)]
    pub frame: Option<u64>,
    /// Grab N evenly-spaced stills instead of one (out_01.jpg … out_NN.jpg)
    #[arg(long)]
    pub count: Option<u32>,
    /// Bound the --count spread window (default: whole clip; `end` ok on --to)
    #[arg(long)]
    pub from: Option<String>,
    #[arg(long)]
    pub to: Option<String>,
    /// Grab a still at every scene change (thumbnail candidates)
    #[arg(long)]
    pub scenes: bool,
    /// Let ffmpeg pick the most representative frame (thumbnail filter:
    /// averages each batch — a clean, typical still from shaky footage)
    #[arg(long)]
    pub best: bool,
    /// Scale the still to this width (height follows aspect)
    #[arg(long)]
    pub width: Option<u32>,
}

#[derive(clap::Args, Debug)]
pub struct MetaArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Title tag
    #[arg(long)]
    pub title: Option<String>,
    /// Artist / author tag
    #[arg(long)]
    pub artist: Option<String>,
    /// Comment / description tag
    #[arg(long)]
    pub comment: Option<String>,
    /// Album / collection tag
    #[arg(long)]
    pub album: Option<String>,
    /// Genre tag
    #[arg(long)]
    pub genre: Option<String>,
    /// Date / year tag
    #[arg(long)]
    pub date: Option<String>,
    /// Track number tag
    #[arg(long)]
    pub track: Option<String>,
    /// Disc number tag (N or N/total) for multi-disc releases
    #[arg(long)]
    pub disc: Option<String>,
    /// Composer / songwriter tag (music metadata)
    #[arg(long)]
    pub composer: Option<String>,
    /// BPM tempo tag (DJ mixes, beat packs) — writes `bpm` for
    /// mp3/mkv/flac plus the iTunes `tmpo` atom so mp4/m4a/mov land it too
    #[arg(long)]
    pub bpm: Option<u32>,
    /// iTunes compilation flag (cpil atom — "Various Artists" albums in
    /// Apple Music/Books libraries; mp4-family)
    #[arg(long)]
    pub compilation: bool,
    /// Comma list of search keywords (iTunes keyw atom — Apple apps index
    /// them for search; lands on mp4/m4a/mov)
    #[arg(long)]
    pub keywords: Option<String>,
    /// Recorder/camera manufacturer tag (lands on .mov — dailies &
    /// footage ingest identity; mp4-family drops it)
    #[arg(long)]
    pub make: Option<String>,
    /// Recorder/camera model tag (lands on .mov — pairs with --make)
    #[arg(long)]
    pub model: Option<String>,
    /// Embed unsynced lyrics from a .lrc/.txt file — LRC timestamps
    /// are stripped so players show plain lines
    #[arg(long)]
    pub lyrics: Option<PathBuf>,
    /// Creation-time tag (ISO 8601 like 2026-09-24T10:00:00Z, or `auto` to
    /// stamp the input file's own mtime) — archive dailies, shoot-day stamping
    #[arg(long)]
    pub creation_time: Option<String>,
    /// Location tag (ISO 6709 like "+31.23+121.47/") — travel/daily vlog
    /// geo-stamp, lands in mp4 too
    #[arg(long)]
    pub location: Option<String>,
    /// Copyright / license tag (release metadata; lands in mp4 too)
    #[arg(long)]
    pub copyright: Option<String>,
    /// Album-artist tag (compilations, DJ mixes) — verified in m4a/mp4
    #[arg(long)]
    pub album_artist: Option<String>,
    /// Show / podcast title tag (TV deliverables, podcast feeds)
    #[arg(long)]
    pub show: Option<String>,
    /// Season number tag — pairs with --show/--episode
    #[arg(long)]
    pub season: Option<String>,
    /// Episode id tag (podcast episode number / TV episode code)
    #[arg(long)]
    pub episode: Option<String>,
    /// Network / broadcaster tag
    #[arg(long)]
    pub network: Option<String>,
    /// iTunes media kind (music|musicvideo|tvshow|movie|audiobook → stik
    /// atom value) — podcast/music app library sorting
    #[arg(long)]
    pub media_type: Option<String>,
    /// Gapless-playback flag (pgap atom) — continuous albums, live splits,
    /// DJ mixes shouldn't gap between tracks
    #[arg(long)]
    pub gapless: bool,
    /// Long description — podcast episode notes / audiobook blurb
    #[arg(long)]
    pub description: Option<String>,
    /// Short synopsis (iTunes stores both; stores show this in listings)
    #[arg(long)]
    pub synopsis: Option<String>,
    /// iTunes hd_video atom — mark the file as HD for store listings
    #[arg(long)]
    pub hd: bool,
    /// Fix the display rotation flag (0/90/180/270) without re-encoding
    #[arg(long)]
    pub rotate: Option<u32>,
    /// Strip ALL container metadata (privacy clean before publishing)
    #[arg(long)]
    pub clear: bool,
    /// Copy metadata + chapters from this file onto the output
    #[arg(long)]
    pub copy: Option<PathBuf>,
    /// Language tag for audio tracks in order, comma list
    /// (`--lang-audio eng,jpn` — players show language in the track
    /// menu; ISO-639 codes)
    #[arg(long)]
    pub lang_audio: Option<String>,
    /// Language tag for subtitle tracks in order, comma list
    /// (`--lang-subs eng,fra` — audience captions labelled for the player)
    #[arg(long)]
    pub lang_subs: Option<String>,
    /// Display title for audio tracks in order, comma list
    /// (`--title-audio "Program,Commentary"` — players show the name
    /// instead of "Track 1"; blank slots skip)
    #[arg(long)]
    pub title_audio: Option<String>,
    /// Display title for subtitle tracks in order, comma list
    /// (`--title-subs "English,Français"` — label each caption track)
    #[arg(long)]
    pub title_subs: Option<String>,
    /// Display title for video tracks in order, comma list
    /// (`--title-video "Main,Angle-2"` — multi-cam files label each angle)
    #[arg(long)]
    pub title_video: Option<String>,
    /// ISRC recording code (`USRC17607839` — the international standard
    /// ID streaming services/distributors match royalties on)
    #[arg(long)]
    pub isrc: Option<String>,
    /// License / rights text (Creative Commons URL, rights statement —
    /// lands on mp3/flac/mkv/ogg; mp4-family containers drop it)
    #[arg(long)]
    pub license: Option<String>,
    /// Publisher / label name (record label, imprint, or publishing
    /// house — lands on mp3/flac/mkv/ogg; mp4-family drops it)
    #[arg(long)]
    pub publisher: Option<String>,
}

#[derive(clap::Args, Debug)]
pub struct MusicArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Music / trending-audio file
    #[arg(long)]
    pub track: PathBuf,
    /// Linear gain on the bed before ducking (0.05–1)
    #[arg(long, default_value_t = 0.18)]
    pub gain: f64,
    /// Duck the bed when speech is present
    #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
    pub duck: bool,
    /// Fade the music bed in/out over N seconds (0 = cut)
    #[arg(long, default_value_t = 0.0)]
    pub fade: f64,
    /// Bed enters at this time — music kicks in after the intro (h:mm:ss or seconds)
    #[arg(long)]
    pub at: Option<String>,
    /// Bed stops after this many seconds (default: plays to the end)
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(clap::Args, Debug)]
pub struct ReplaceArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Replacement audio file (lav mic, clean voice, music)
    #[arg(long, required_unless_present = "video")]
    pub audio: Option<PathBuf>,
    /// Swap the PICTURE instead: keep this video's audio, show this file's
    /// frames (music video / retake — audio is the master clock)
    #[arg(long)]
    pub video: Option<PathBuf>,
    /// Shift the new audio in seconds: positive delays, negative trims its start
    #[arg(long, allow_hyphen_values = true, default_value_t = 0.0)]
    pub audio_offset: f64,
    /// Fade the new audio in/out over N seconds (0 = hard start/end)
    #[arg(long, default_value_t = 0.0)]
    pub fade: f64,
    /// Keep the original track under the new one at this linear gain (0–1)
    #[arg(long, default_value_t = 0.0)]
    pub mix: f64,
    /// With --mix: sidechain-duck the original under the new audio (voice-over)
    #[arg(long)]
    pub duck: bool,
    /// Loop the replacement audio if it is shorter than the video
    #[arg(long = "loop")]
    pub loop_track: bool,
    /// Replace only inside this window: original audio keeps playing outside —
    /// comma list lays the new track across several windows (needs --dur)
    #[arg(long)]
    pub at: Option<String>,
    /// Window length (default: to the end of the new audio)
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(clap::Args, Debug)]
pub struct SlideshowArgs {
    /// Still images, in order
    pub inputs: Vec<PathBuf>,
    /// Read still paths from a manifest file — one path per line, '#'
    /// comments allowed, relative paths resolve against the list's
    /// directory; curated order beyond --sort's name/mtime
    #[arg(long)]
    pub list: Option<PathBuf>,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Seconds each image stays on screen
    #[arg(long, default_value_t = 3.0)]
    pub per: f64,
    /// Total montage length in seconds — overrides --per (per = the value
    /// that lands the reel on exactly this runtime, fade included)
    #[arg(long)]
    pub dur: Option<f64>,
    /// Crossfade seconds between images (0 = hard cuts)
    #[arg(long, default_value_t = 0.6)]
    pub fade: f64,
    /// Canvas WxH (even numbers)
    #[arg(long, default_value = "1920x1080")]
    pub size: String,
    /// Music bed under the slideshow (faded out at the end)
    #[arg(long)]
    pub audio: Option<PathBuf>,
    /// Transition between stills (needs --fade > 0)
    #[arg(long, value_enum, default_value_t = XfadeTransition::Fade)]
    pub transition: XfadeTransition,
    /// Per-still motion (kenburns = slow push-in / pull-out via zoompan)
    #[arg(long, value_enum, default_value_t = SlideMotion::None)]
    pub motion: SlideMotion,
    /// Output frame rate
    #[arg(long, default_value_t = 30.0)]
    pub fps: f64,
    /// Linear gain on the music bed 0..=4 (with --audio; default 1.0 —
    /// drop to ~0.5 under narration)
    #[arg(long)]
    pub volume: Option<f64>,
    /// Letterbox color behind stills (name or RRGGBB; default black)
    #[arg(long)]
    pub bg: Option<String>,
    /// End the montage exactly when the --audio bed ends — solves --per
    /// from the music's length so the last still lands on the song's outro
    #[arg(long)]
    pub fit: bool,
    /// Shuffle the stills deterministically: same seed = same order
    /// (photo-dump montages, reroll the order without re-listing files)
    #[arg(long)]
    pub shuffle: Option<u64>,
    /// Sort the stills: name (path order) or mtime (shoot-time order for
    /// a camera-dump folder; overrides arg order, loses to --shuffle)
    #[arg(long, value_enum)]
    pub sort: Option<SlideSort>,
    /// Per-still bottom captions, comma list in the final slide order
    /// (after --sort/--shuffle — an empty entry skips that still):
    /// travel-photo labels, portfolio credits, event recaps
    #[arg(long)]
    pub titles: Option<String>,
    /// Music-bed tail fade seconds (default 0.8 — longer lets the song
    /// ring out under the last still instead of cutting abruptly)
    #[arg(long)]
    pub audio_fade: Option<f64>,
    /// Start the music bed T seconds in — skip the intro, use the chorus
    /// (--fit measures the montage against the bed's remaining length)
    #[arg(long)]
    pub audio_offset: Option<f64>,
    /// Loop the music bed when it's shorter than the montage
    /// (-stream_loop -1 — short jingle under a long slideshow)
    #[arg(long)]
    pub audio_loop: bool,
    /// Music-bed head fade-in seconds (eases the song in instead of
    /// starting cold — pairs with --audio-fade's tail fade)
    #[arg(long)]
    pub audio_fade_in: Option<f64>,
}

#[derive(clap::Args, Debug)]
pub struct KeyArgs {
    /// Foreground footage with the color to remove
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Background image or video (sized to the foreground canvas);
    /// not needed for --mode matte
    #[arg(long)]
    pub bg: Option<PathBuf>,
    /// Grayscale matte clip for --mode matte (luma becomes the alpha)
    #[arg(long)]
    pub mask: Option<PathBuf>,
    /// Hex color to remove, e.g. 0x00ff00 or 00ff00
    #[arg(long, default_value = "0x00ff00")]
    pub color: String,
    /// Similarity threshold 0..1
    #[arg(long, default_value_t = 0.3)]
    pub similarity: f64,
    /// Edge blend 0..1
    #[arg(long, default_value_t = 0.05)]
    pub blend: f64,
    /// Remove color spill ringing on the keyed edges
    #[arg(long)]
    pub despill: bool,
    /// Key mode: color (green/blue screen, default) | luma (bright/dark
    /// background — whiteboard scans, bright sky, no green screen needed)
    #[arg(long, value_enum)]
    pub mode: Option<KeyMode>,
    /// Luma pivot to key out for --mode luma (0-1; ~0.1 = dark bg, ~0.9 = bright sky)
    #[arg(long)]
    pub threshold: Option<f64>,
    /// Key only inside this window — comma list for several windows (needs --dur)
    #[arg(long)]
    pub at: Option<String>,
    /// Window length for --at (seconds; default = to the end)
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum XfadeTransition {
    Fade,
    Wipeleft,
    Wiperight,
    Wipeup,
    Wipedown,
    Slideleft,
    Slideright,
    Slideup,
    Slidedown,
    Dissolve,
    Radial,
    Circleopen,
    Circleclose,
    Circlecrop,
    Rectcrop,
    Distance,
    Fadeblack,
    Fadewhite,
    Fadegrays,
    Smoothleft,
    Smoothright,
    Smoothup,
    Smoothdown,
    Vertopen,
    Vertclose,
    Horzopen,
    Horzclose,
    Pixelize,
    Diagtl,
    Diagtr,
    Diagbl,
    Diagbr,
    Hlslice,
    Hrslice,
    Vuslice,
    Vdslice,
    Hblur,
    Wipetl,
    Wipetr,
    Wipebl,
    Wipebr,
    Squeezeh,
    Squeezev,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum SlideMotion {
    None,
    Kenburns,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum SlideSort {
    /// Path-name order
    Name,
    /// File mtime — shoot-time order for a camera dump
    Mtime,
}

impl XfadeTransition {
    pub fn xfade_name(self) -> &'static str {
        match self {
            Self::Fade => "fade",
            Self::Wipeleft => "wipeleft",
            Self::Wiperight => "wiperight",
            Self::Wipeup => "wipeup",
            Self::Wipedown => "wipedown",
            Self::Slideleft => "slideleft",
            Self::Slideright => "slideright",
            Self::Slideup => "slideup",
            Self::Slidedown => "slidedown",
            Self::Dissolve => "dissolve",
            Self::Radial => "radial",
            Self::Circleopen => "circleopen",
            Self::Circleclose => "circleclose",
            Self::Circlecrop => "circlecrop",
            Self::Rectcrop => "rectcrop",
            Self::Distance => "distance",
            Self::Fadeblack => "fadeblack",
            Self::Fadewhite => "fadewhite",
            Self::Fadegrays => "fadegrays",
            Self::Smoothleft => "smoothleft",
            Self::Smoothright => "smoothright",
            Self::Smoothup => "smoothup",
            Self::Smoothdown => "smoothdown",
            Self::Vertopen => "vertopen",
            Self::Vertclose => "vertclose",
            Self::Horzopen => "horzopen",
            Self::Horzclose => "horzclose",
            Self::Pixelize => "pixelize",
            Self::Diagtl => "diagtl",
            Self::Diagtr => "diagtr",
            Self::Diagbl => "diagbl",
            Self::Diagbr => "diagbr",
            Self::Hlslice => "hlslice",
            Self::Hrslice => "hrslice",
            Self::Vuslice => "vuslice",
            Self::Vdslice => "vdslice",
            Self::Hblur => "hblur",
            Self::Wipetl => "wipetl",
            Self::Wipetr => "wipetr",
            Self::Wipebl => "wipebl",
            Self::Wipebr => "wipebr",
            Self::Squeezeh => "squeezeh",
            Self::Squeezev => "squeezev",
        }
    }
}

#[derive(clap::Args, Debug)]
pub struct JumpcutArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Silence threshold in dBFS (more negative = quieter counts as speech)
    #[arg(long, default_value_t = -30.0, allow_hyphen_values = true)]
    pub threshold: f64,
    /// Minimum silence length to cut, seconds
    #[arg(long, default_value_t = 0.3)]
    pub min_duration: f64,
    /// Keep this much silence on each side of a cut, seconds
    #[arg(long, default_value_t = 0.05)]
    pub pad: f64,
}

#[derive(clap::Args, Debug)]
pub struct RoughArgs {
    pub input: PathBuf,
    /// Assemble keeps here. Omit to only list speech islands as JSON.
    #[arg(short, long)]
    pub output: Option<PathBuf>,
    /// Silence threshold in dBFS
    #[arg(long, default_value_t = -30.0, allow_hyphen_values = true)]
    pub threshold: f64,
    /// Minimum silence length to split on, seconds (longer than jumpcut: keep breaths)
    #[arg(long, default_value_t = 0.5)]
    pub min_duration: f64,
    /// Keep this much silence on each side of a keep, seconds
    #[arg(long, default_value_t = 0.12)]
    pub pad: f64,
    /// Stream-copy keeps (fast, keyframe-sloppy). Default encodes only the keep windows.
    #[arg(long)]
    pub copy: bool,
    /// Merge keeps whose gap is smaller than N seconds (less jarring jump cuts)
    #[arg(long, default_value_t = 0.0)]
    pub merge: f64,
    /// Also split keeps at scene changes (cut detection, threshold 0.4)
    #[arg(long)]
    pub by_scene: bool,
}

#[derive(clap::Args, Debug)]
pub struct CoverArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Timestamp to grab; default 0 — `end` ok, comma list = one cover per time
    #[arg(long)]
    pub at: Option<String>,
    /// Ambient: fill the pad with a blurred copy of the frame instead of black
    #[arg(long)]
    pub blur: bool,
    /// Canvas WxH (default 1080x1920; 1280x720 for YouTube thumbs)
    #[arg(long)]
    pub size: Option<String>,
}

#[derive(clap::Args, Debug)]
pub struct FadeArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Fade-in seconds (0 = none)
    #[arg(long = "in", default_value_t = 0.25)]
    pub fade_in: f64,
    /// Fade-out seconds (0 = none)
    #[arg(long = "out", default_value_t = 0.25)]
    pub fade_out: f64,
    /// Fade to this color (default black; e.g. white)
    #[arg(long)]
    pub color: Option<String>,
    /// Dip to the color at this time — scene-change transition (half out, half back);
    /// comma list dips at several points (e.g. every chapter mark)
    #[arg(long)]
    pub dip: Option<String>,
    /// Dip length in seconds (default 0.8 — needs --dip)
    #[arg(long)]
    pub dur: Option<f64>,
    /// Audio fade curve shape (default linear)
    #[arg(long, value_enum)]
    pub curve: Option<FadeCurve>,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum FadeCurve {
    Tri,
    Qsin,
    Esin,
    Hsin,
    Log,
    Qua,
    Cub,
    Exp,
}

#[derive(clap::Args, Debug)]
pub struct TitleArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Hook text (newlines allowed)
    #[arg(long, required_unless_present = "file")]
    pub text: Option<String>,
    /// Read the hook text from a file instead of --text
    #[arg(long)]
    pub file: Option<PathBuf>,
    /// Auto word-wrap the text at N chars per line
    #[arg(long)]
    pub wrap: Option<u32>,
    /// Seconds the title stays on screen
    #[arg(long, default_value_t = 1.0)]
    pub duration: f64,
    /// Show the title at this time instead of the start — comma list flashes it at several marks
    #[arg(long)]
    pub at: Option<String>,
    #[arg(long)]
    pub font: Option<String>,
    /// Tile the text N times diagonally at 50% alpha (text draft watermark)
    #[arg(long, default_value_t = 0)]
    pub tile: u32,
    /// Text size multiplier (default 1.0)
    #[arg(long, default_value_t = 1.0)]
    pub size: f64,
    /// Text color as RRGGBB hex (default ffffff)
    #[arg(long)]
    pub color: Option<String>,
    /// center (default), top, or bottom
    #[arg(long, default_value = "center")]
    pub position: String,
    /// Fade the title in/out over this many seconds (0 = cut)
    #[arg(long, default_value_t = 0.0)]
    pub fade: f64,
    /// Stroke color around the text (RRGGBB hex)
    #[arg(long)]
    pub outline: Option<String>,
    /// Soft drop shadow under the title card (blur radius in px)
    #[arg(long)]
    pub shadow: Option<u32>,
    /// Filled card behind the text: RRGGBB or RRGGBBAA hex / color name
    #[arg(long)]
    pub box_color: Option<String>,
    /// Multi-line text alignment inside the title card
    #[arg(long, value_enum)]
    pub align: Option<crate::raster::TextAlign>,
    /// Draw the title at this % opacity (0-100 — ghost/watermark titles)
    #[arg(long)]
    pub opacity: Option<f64>,
    /// Pixels from the edge for corner positions (overrides the percent inset)
    #[arg(long)]
    pub margin: Option<u32>,
}

#[derive(clap::Args, Debug)]
pub struct SolidArgs {
    #[arg(short, long)]
    pub output: PathBuf,
    /// Fill color: name or RRGGBB/0xRRGGBB (default black)
    #[arg(long, default_value = "black")]
    pub color: String,
    /// Frame size WxH (default 1920x1080)
    #[arg(long, default_value = "1920x1080")]
    pub size: String,
    /// Length in seconds
    #[arg(long, default_value_t = 5.0)]
    pub dur: f64,
    /// Also add a silent stereo track (default true for edit-friendly files)
    #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
    pub audio: bool,
    /// Animated gradient RRGGBB:RRGGBB instead of a flat color
    #[arg(long)]
    pub gradient: Option<String>,
    /// Centered text on the card (end-cards, section titles)
    #[arg(long)]
    pub text: Option<String>,
    /// Text color RRGGBB (default white)
    #[arg(long)]
    pub text_color: Option<String>,
    /// Font .ttf for --text
    #[arg(long)]
    pub font: Option<String>,
    /// Fade the card in from / out to black over N seconds each side
    #[arg(long)]
    pub fade: Option<f64>,
    /// Word-wrap --text at N columns (≥4)
    #[arg(long)]
    pub wrap: Option<u32>,
    /// Animated film-grain on the card (0-100 strength)
    #[arg(long)]
    pub noise: Option<u32>,
    /// Card frame rate (default 30; 24 for filmic grain)
    #[arg(long)]
    pub fps: Option<f64>,
    /// Per-line alignment of --text on the card (with --wrap)
    #[arg(long, value_enum)]
    pub align: Option<crate::raster::TextAlign>,
}

#[derive(clap::Args, Debug)]
pub struct LoopArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Play this many times (2–12)
    #[arg(long, default_value_t = 2)]
    pub times: u32,
    /// Repeat until the output is this long in seconds (overrides --times)
    #[arg(long)]
    pub until: Option<f64>,
    /// Loop only this section, keep the rest once (needs --to or loops to end)
    #[arg(long)]
    pub from: Option<String>,
    /// Section end (needs --from)
    #[arg(long)]
    pub to: Option<String>,
    /// Crossfade seconds at every loop joint (seamless GIF-style loops;
    /// re-encodes instead of stream-copying)
    #[arg(long)]
    pub fade: Option<f64>,
}

#[derive(clap::Args, Debug)]
pub struct StabilizeArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Search radius on x (pixels)
    #[arg(long, default_value_t = 16)]
    pub rx: u32,
    /// Search radius on y (pixels)
    #[arg(long, default_value_t = 16)]
    pub ry: u32,
    /// How to fill the frame edge exposed by stabilization
    #[arg(long, value_enum)]
    pub edge: Option<StabilizeEdge>,
    /// Engine: deshake (default, single-pass) | vidstab (two-pass vid.stab —
    /// professional-grade; analyzes then transforms, steadier on real shake)
    #[arg(long, value_enum)]
    pub engine: Option<StabilizeEngine>,
    /// vidstab smoothing window in frames (default 15 = 2*N+1 past/future
    /// frames averaged; larger = smoother but slower to adapt to pans)
    #[arg(long)]
    pub smoothing: Option<u32>,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq)]
pub enum StabilizeEngine {
    /// deshake — single-pass block matching, fast
    Deshake,
    /// vidstab — vid.stab two-pass (detect transform then correct); the
    /// stabilizer pro NLEs wrap, steadier on handheld walking shots
    Vidstab,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
pub enum StabilizeEdge {
    /// Black border
    Blank,
    /// Keep original frame at the edge (less motion fill)
    Original,
    /// Clamp edge pixels
    Clamped,
    /// Mirror the edge (default)
    Mirror,
}

#[derive(clap::Args, Debug)]
pub struct ReverseArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
}

#[derive(clap::Args, Debug)]
pub struct GradeArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    #[arg(long, default_value_t = 1.12, allow_hyphen_values = true)]
    pub contrast: f64,
    #[arg(long, default_value_t = 1.18, allow_hyphen_values = true)]
    pub saturation: f64,
    #[arg(long, default_value_t = 0.02, allow_hyphen_values = true)]
    pub brightness: f64,
    /// EV stops (like a camera dial): -3 darkens .. +3 opens up underexposed
    /// footage — real exposure compensation, not a brightness slide
    #[arg(long, allow_hyphen_values = true)]
    pub exposure: Option<f64>,
    /// Apply a 3D LUT file (.cube etc) after the slider correction
    #[arg(long)]
    pub lut: Option<PathBuf>,
    /// Mid-tone gamma (default 1.0; >1 lifts mids like log-ish open-up)
    #[arg(long, default_value_t = 1.0)]
    pub gamma: f64,
    /// Film-grain amount in luma units (0 = off; 4–10 reads as film)
    #[arg(long, default_value_t = 0.0)]
    pub grain: f64,
    /// Warmth: positive warms (red cast), negative cools (blue), -1..1
    #[arg(long, default_value_t = 0.0, allow_hyphen_values = true)]
    pub warm: f64,
    /// One-click look applied before the sliders: cinematic, vivid, vintage, soft
    #[arg(long, value_enum)]
    pub preset: Option<GradePreset>,
    /// Rotate the hue by N degrees (-180..180): white-balance rescue or color FX
    #[arg(long, default_value_t = 0.0, allow_hyphen_values = true)]
    pub hue: f64,
    /// Skin warmth -1..1: selectivecolor on the reds channel only — warms
    /// faces without touching the rest of the grade
    #[arg(long, default_value_t = 0.0, allow_hyphen_values = true)]
    pub skin: f64,
    /// White balance by Kelvin (1000-40000): 2700 tungsten, 5500 daylight,
    /// 9000 cool — direct dial when --warm's -1..1 slide isn't enough
    #[arg(long)]
    pub kelvin: Option<f64>,
    /// Split-tone strength -1..1: teal shadows + orange highlights (the
    /// blockbuster grade); negative flips to warm shadows / cool highlights
    #[arg(long, allow_hyphen_values = true)]
    pub split: Option<f64>,
    /// Shadow-zone colour spots R,B (-1..1 each — pull a cast out of the
    /// shadows without cooling the highlights: mixed-lighting WB repair)
    #[arg(long, allow_hyphen_values = true)]
    pub shadows: Option<String>,
    /// Highlight-zone colour spots R,B (-1..1 each — rh/bh of the same
    /// two-zone repair; pairs with --shadows)
    #[arg(long, allow_hyphen_values = true)]
    pub highlights: Option<String>,
    /// Freeform master curve points "x/y x/y" (curves master): S-curve
    /// "0/0 0.25/0.18 0.75/0.82 1/1", matte "0/0.08 1/0.92"
    #[arg(long)]
    pub curve: Option<String>,
    /// Smarter saturation -1..1 (vibrance: boosts muted colors while
    /// protecting already-saturated skin — safer than --saturation on faces)
    #[arg(long, default_value_t = 0.0, allow_hyphen_values = true)]
    pub vibrance: f64,
    /// Borrow a reference clip's colour histogram (midequalizer) — camera
    /// matching / "grade it like that film". Second input, scaled to fit
    #[arg(long)]
    pub match_: Option<PathBuf>,
    /// Channel mixer — 9 gains rr,rg,rb,gr,gg,gb,br,bg,bb (0..2 each;
    /// identity = 1,0,0,0,1,0,0,0,1). Swap channels, custom orange/teal
    #[arg(long)]
    pub mix: Option<String>,
    /// Borrow the chroma (U/V) of another clip — mergeplanes: your luma,
    /// their color grade. Incompatible with --match / HALD --lut / --at
    #[arg(long, value_name = "REF")]
    pub color_from: Option<PathBuf>,
    /// Colour wash over the frame (colorize) — mood veil that keeps luma
    #[arg(long)]
    pub wash: Option<String>,
    /// Wash intensity 0..1 (default 0.5: saturation scales, lightness lifts)
    #[arg(long, default_value_t = 0.5)]
    pub wash_amount: f64,
    /// Grade only from this time — dream sequences, flashbacks; comma list for several windows (needs --dur)
    #[arg(long)]
    pub at: Option<String>,
    /// ..for this many seconds (default: to the end)
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum GradePreset {
    Cinematic,
    Vivid,
    Vintage,
    Soft,
    Sepia,
    /// Orange-and-teal blockbuster look
    Teal,
    /// High-contrast black & white
    Noir,
    /// Bleach bypass: crushed desat + hard contrast
    Bleach,
    /// Cyberpunk cyan shadows + magenta highlights
    Neon,
    /// Cross-processed film look (curves preset)
    Crossprocess,
    /// Hard S-curve contrast (curves preset)
    Strongcontrast,
    /// Gentle straight-line contrast stretch (curves preset)
    Linearcontrast,
}

#[derive(clap::Args, Debug)]
pub struct ZoomArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// 1.25 = 25% punch-in on the center
    #[arg(long, default_value_t = 1.25)]
    pub factor: f64,
    /// Punch only inside this window — comma list for several windows (needs --dur)
    #[arg(long)]
    pub at: Option<String>,
    /// Window length in seconds (default: to the end)
    #[arg(long)]
    pub dur: Option<f64>,
    /// Animate the punch over the window (kenburns = smooth push)
    #[arg(long, value_enum)]
    pub motion: Option<SlideMotion>,
    /// Zoom OUT instead of in: starts at --factor and settles to 1x (reveal shot)
    #[arg(long)]
    pub out: bool,
    /// Punch target point X,Y in % of frame (default 50,50 = dead center;
    /// 25,50 zooms the left third)
    #[arg(long)]
    pub center: Option<String>,
}

#[derive(clap::Args, Debug)]
pub struct PitchArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Semitones: +4 chipmunk-ish, -3 deeper (duration preserved)
    #[arg(long, allow_hyphen_values = true)]
    pub semitones: f64,
    /// Shift pitch only inside this window — comma list for several (needs --dur)
    #[arg(long)]
    pub at: Option<String>,
    /// Window length in seconds (default: to the end)
    #[arg(long)]
    pub dur: Option<f64>,
    /// Preserve formants (natural voice pitch, not chipmunk) — needs librubberband
    #[arg(long)]
    pub formant: bool,
}

#[derive(clap::Args, Debug)]
pub struct BleepArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Window start(s) — comma list for several bleeps (`end` ok per entry)
    #[arg(long)]
    pub at: String,
    /// Window length in seconds
    #[arg(long)]
    pub dur: f64,
    /// Beep frequency in Hz
    #[arg(long, default_value_t = 1000.0)]
    pub freq: f64,
    /// Beep level vs original mix (0..1)
    #[arg(long, default_value_t = 0.5)]
    pub level: f64,
}

#[derive(clap::Args, Debug)]
pub struct ReverbArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Room size preset
    #[arg(long, value_enum, default_value_t = ReverbSize::Room)]
    pub size: ReverbSize,
    /// Wet tail amount (0..0.9; 0.3 ≈ subtle room). With --ir it maps to
    /// afir wet gain 0..9 — 0.3 ≈ a live room
    #[arg(long, default_value_t = 0.3)]
    pub wet: f64,
    /// Convolution reverb: impulse-response WAV from an IR pack
    /// (cathedral/hall/plate — real spaces, not synthetic echo taps)
    #[arg(long)]
    pub ir: Option<PathBuf>,
    /// Ring the tail past the end by this many seconds (default: the IR's
    /// own length, so the last note still blooms)
    #[arg(long)]
    pub tail: Option<f64>,
    /// Start the reverb only here (echo on the hook)
    #[arg(long)]
    pub at: Option<String>,
    /// Stop the reverb after this many seconds (needs --at)
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum ReverbSize {
    #[default]
    Room,
    Hall,
    Cave,
}

#[derive(clap::Args, Debug)]
pub struct EqArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Bass gain dB (-20..20; 4-8 warms a voice)
    #[arg(long, allow_hyphen_values = true, default_value_t = 0.0)]
    pub bass: f64,
    /// Treble gain dB (-20..20)
    #[arg(long, allow_hyphen_values = true, default_value_t = 0.0)]
    pub treble: f64,
    /// Presence shelf at 3 kHz dB (voice clarity)
    #[arg(long, allow_hyphen_values = true, default_value_t = 0.0)]
    pub presence: f64,
    /// One-shot curve: voice|podcast|bright|bass (flags still apply on top)
    #[arg(long, value_enum)]
    pub preset: Option<EqPreset>,
    /// Tilt the whole spectrum -10..10 dB (+ warms bass / − brightens)
    #[arg(long, allow_hyphen_values = true)]
    pub tilt: Option<f64>,
    /// Parametric band FREQ:GAIN[:WIDTH_OCT], repeatable
    /// (e.g. --band 800:-3 --band 5200:2:0.7)
    #[arg(long)]
    pub band: Vec<String>,
    /// Freehand EQ curve through F,G points (freq Hz, gain dB):
    /// "80,0;3000,-6;8000,4" — points interpolate (firequalizer)
    #[arg(long)]
    pub curve: Option<String>,
    /// Classic graphic EQ: up to 18 comma dB gains (65Hz..20kHz sliders)
    /// e.g. --graphic "0,0,-6,-6,-3,0,0,0,2,2"
    #[arg(long)]
    pub graphic: Option<String>,
    /// Standard de-emphasis curve: riaa (vinyl rips) / cd / fm50 / fm75
    /// (broadcast captures) — undoes the recording's pre-emphasis HF boost
    #[arg(long, value_enum)]
    pub deemph: Option<DeemphType>,
    /// Shelf filter SIDE:FREQ:GAIN, repeatable — low:200:-3 cuts a rumble
    /// shelf under 200Hz, high:8000:4 adds an air shelf over 8kHz
    #[arg(long)]
    pub shelf: Vec<String>,
    /// Notch out a frequency FREQ[:WIDTH_HZ], repeatable — resonance/ring
    /// removal beyond dehum's fixed mains list (e.g. --notch 1250)
    #[arg(long)]
    pub notch: Vec<String>,
    /// Brick-wall bandpass LO,HI Hz — keeps only that band (telephone FX,
    /// speech-band isolation 300,3400); FFT-domain, zero phase smear
    #[arg(long)]
    pub brickwall: Option<String>,
    /// Butterworth resonant lowpass FREQ[:Q] — 3dB point at FREQ, Q
    /// resonance width (default 0.707); synth-style filter sweeps, HF
    /// hiss roll-off sharper than a shelf
    #[arg(long)]
    pub lowpass: Option<String>,
    /// Butterworth resonant highpass FREQ[:Q] — cuts everything under
    /// FREQ (mic rumble/handling below ~80, plosive cleanup)
    #[arg(long)]
    pub highpass: Option<String>,
    /// Butterworth bandpass FREQ[:WIDTH_HZ] — keeps FREQ±W (default
    /// FREQ/2); isolates a band without the brickwall's FFT frame echo
    #[arg(long)]
    pub bandpass: Option<String>,
    /// Linear-phase FIR for the resonant filters (sinc+afir): flat
    /// passband, ~60dB stopband, zero phase smear — mastering-safe cuts.
    /// Applies to --lowpass/--highpass/--bandpass; can't take --at
    #[arg(long)]
    pub linear: bool,
    /// Sub-bass cut FREQ Hz (2-200) — mic-stand rumble, wind, handling
    /// noise under the voice band; order-10 highpass
    #[arg(long)]
    pub subcut: Option<f64>,
    /// Ultrasonic cut FREQ Hz (20000-192000) — hiss/pilot-tone cleanup
    /// above the hearing band on high-sample-rate masters
    #[arg(long)]
    pub supercut: Option<f64>,
    /// Razor band isolation FREQ[:Q] — order-10 asuperpass band pass:
    /// keep only the band around FREQ (isolate a whine, whistle or tone)
    #[arg(long)]
    pub superpass: Option<String>,
    /// Razor band kill FREQ Hz — order-10 asuperstop band stop centered
    /// on FREQ (~50dB deeper than --notch's two-pole reject)
    #[arg(long)]
    pub superstop: Option<f64>,
    /// Phase rotator FREQ:WIDTH — two-pole allpass: symmetrizes lopsided
    /// vocal waveforms for free headroom (tone untouched)
    #[arg(long)]
    pub allpass: Option<String>,
    /// Apply the EQ only from here (bass boost on the drop)
    #[arg(long)]
    pub at: Option<String>,
    /// Stop after this many seconds (needs --at)
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(clap::Args, Debug)]
pub struct ChannelArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// dualmono: copy ch0 onto all channels | mono: fold to one | swap: L/R flip
    #[arg(long, value_enum, default_value_t = ChannelMode::Dualmono)]
    pub mode: ChannelMode,
    /// With --mode invert: which side flips polarity: left|right|both (default both)
    #[arg(long)]
    pub side: Option<String>,
    /// With --mode pan/bal: stereo position -1 (full left) .. 1 (full right)
    #[arg(long, allow_hyphen_values = true)]
    pub pan: Option<f64>,
    /// With --mode ambience: side-channel keep ratio 0..1 (room/reverb cut)
    #[arg(long)]
    pub amount: Option<f64>,
    /// With --mode bands: crossover frequencies Hz comma list (default "300,3000")
    #[arg(long)]
    pub freqs: Option<String>,
    /// With --mode sync: cm the --side mic sat closer to the source (0-100)
    #[arg(long)]
    pub cm: Option<f64>,
    /// With --mode merge: second track interleaved after this one's channels
    #[arg(long)]
    pub with: Option<PathBuf>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum DeemphType {
    /// RIAA vinyl curve — record-rip correction
    Riaa,
    /// Compact Disc 50/15µs curve
    Cd,
    /// FM broadcast 50µs (EU/Oceania captures)
    Fm50,
    /// FM broadcast 75µs (Americas captures)
    Fm75,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum ChannelMode {
    Dualmono,
    Mono,
    Swap,
    Invert,
    /// 5.1 surround → stereo fold-down (dialogue keeps center gain)
    Mix51,
    /// Stereo widen for flat camera audio (extrastereo)
    Widen,
    /// Stereo pan --pan -1..1 (push the mix to one ear)
    Pan,
    /// Split stereo into two mono files <stem>_L.wav / <stem>_R.wav (host/guest mics)
    Split,
    /// Cut side-channel room/ambience (stereotools slev) — drier voice
    Ambience,
    /// Keep only the mid (center) channel — isolate centered voice/dry-mix
    Mid,
    /// Keep only the side (difference) channel — room tone / ambience capture
    Side,
    /// Haas-effect stereo widening (short L/R delays) — mono-safe width;
    /// --amount 0..1 scales side gain 0.5..3.0
    Haas,
    /// Upmix stereo to 5.1 surround (soundfield transform + derived LFE —
    /// TV / set-top / cinema delivery)
    Surround,
    /// Stereo base via stereotools: --pan -1 (mono-fold) .. 0 (unchanged) .. 1 (wide)
    Base,
    /// Decode mid/side-recorded stereo back to L/R (stereotools ms>lr)
    Ms,
    /// Balance correction for lopsided stereo (tape drift, mismatched mics):
    /// --pan -1..1 (positive pushes the image toward the right channel)
    Bal,
    /// Frequency-band stems via acrossover: writes <stem>_band1..N.wav
    /// (low/mid/high splits for remixes); --freqs comma crossover Hz
    Bands,
    /// Delay one side by mic distance to fix two-mic comb-filtering on one
    /// source: --side left|right (default right) delayed by --cm (34cm ≈ 1ms)
    Sync,
    /// earwax — headphone-oriented stereo widening (crossfeed delay); makes
    /// podcasts/videos feel less "inside the skull" on earbuds
    Earwax,
    /// Merge --with FILE into one multichannel track (amerge): two mono mics
    /// → stereo (host L / guest R), two stereo stems → quad; input 0 lands
    /// on the first channels
    Merge,
    /// stereowiden — dedicated M/S widener (delay+feedback+crossfeed): wider
    /// stereo image on mono-safe terms; --amount 0..1 scales crossfeed 0.05..0.8
    Stereowiden,
}

#[derive(clap::Args, Debug)]
pub struct CutsilArgs {
    /// Audio file (or video with a single narration track)
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Silence threshold in dB (e.g. -45)
    #[arg(long, allow_hyphen_values = true, default_value_t = -45.0)]
    pub thresh: f64,
}

#[derive(clap::Args, Debug)]
pub struct AutocropArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Expand the detected crop box by N px on every side
    #[arg(long, default_value_t = 0)]
    pub buffer: i64,
}

#[derive(clap::Args, Debug)]
pub struct SheetArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    #[arg(long, default_value_t = 4)]
    pub cols: u32,
    #[arg(long, default_value_t = 3)]
    pub rows: u32,
    /// Tile width px (height follows aspect, forced even)
    #[arg(long, default_value_t = 320)]
    pub tile: u32,
    /// Gap between tiles in px (default 6)
    #[arg(long)]
    pub pad: Option<u32>,
    /// Outer margin in px (default = --pad)
    #[arg(long)]
    pub margin: Option<u32>,
    /// Stamp each tile's source timestamp under it (review sheets)
    #[arg(long)]
    pub time: bool,
    /// Big label rendered above the grid (header row, e.g. project name)
    #[arg(long)]
    pub title: Option<String>,
    /// Sample tiles only from this time on (h:mm:ss or seconds)
    #[arg(long)]
    pub from: Option<String>,
    /// ..up to this time (default: input end)
    #[arg(long)]
    pub to: Option<String>,
}

#[derive(clap::Args, Debug)]
pub struct SpriteArgs {
    pub input: PathBuf,
    /// Output stem — writes <stem>-1.jpg..-N.jpg plus <stem>.vtt
    #[arg(short, long)]
    pub output: PathBuf,
    /// WebVTT path (default: <output>.vtt)
    #[arg(long)]
    pub vtt: Option<PathBuf>,
    /// Seconds between thumbnails
    #[arg(long, default_value_t = 10.0)]
    pub every: f64,
    /// Bound the thumbnail window (default: whole clip; `end` ok on --to)
    #[arg(long)]
    pub from: Option<String>,
    #[arg(long)]
    pub to: Option<String>,
    /// Thumbnail width px (height follows aspect, forced even)
    #[arg(long, default_value_t = 160)]
    pub width: u32,
    /// Tiles per sheet axis (default 10x10 = 100 thumbs/sheet)
    #[arg(long, default_value_t = 10)]
    pub cols: u32,
    #[arg(long, default_value_t = 10)]
    pub rows: u32,
}

#[derive(clap::Args, Debug)]
pub struct TempoArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Speed factor 0.5..=8 (pitch preserved)
    #[arg(long, default_value_t = 1.5)]
    pub factor: f64,
    /// Retempo only this window — comma list for several windows (needs --dur)
    #[arg(long)]
    pub at: Option<String>,
    /// Window length in seconds (default: to the end)
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(clap::Args, Debug)]
pub struct VocalArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// karaoke = drop the center (vocals); isolate = keep only the center
    #[arg(long, value_enum, default_value_t = VocalMode::Karaoke)]
    pub mode: VocalMode,
    /// Effect amount 0..1 (default 1.0 = full cancel / full center)
    #[arg(long)]
    pub amount: Option<f64>,
    /// Apply only inside this window — comma list for several (needs --dur)
    #[arg(long)]
    pub at: Option<String>,
    /// Window length in seconds (default: to the end)
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum VocalMode {
    Karaoke,
    Isolate,
}

/// Bitstream filter applied during a stream copy — codec-level header
/// and payload surgery (annexb for mp4→ts, adts captures → .m4a)
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum RemuxBsf {
    /// H.264 avcC → Annex B startcodes (mp4/mov destined for .ts)
    Annexb,
    /// HEVC hvcC → Annex B startcodes (mp4/mov destined for .ts)
    HevcAnnexb,
    /// AAC ADTS → mp4-style header (internet-radio/DAB+ .aac → .m4a)
    Adts,
    /// Strip the fake Xing/VBR header off constant-rate MP3s
    Mp3Hdr,
    /// E-AC3 → core AC-3 (SPDIF-era amps can't decode the extension)
    Eac3Core,
    /// DTS → core substream (extracts the legacy track from dts-hd)
    DcaCore,
    /// MJPEG frames → JPEG payloads (camera still-capture streams)
    MjpegJpg,
    /// Re-stamp subtitle cue durations (heals overlapped/huge cue ends)
    FixSubs,
    /// Redundant PPS per keyframe (stream-recoverability broadcast spec)
    RedundantPps,
    /// Rebuild codec extradata from frames (heals files missing avcC)
    ExtractExtra,
}

#[derive(clap::Args, Debug)]
pub struct RemuxArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Keep only audio streams — rip the track to m4a/mp3/ogg/wav
    #[arg(long)]
    pub audio: bool,
    /// Keep only the video — repack muted (no re-encode)
    #[arg(long)]
    pub video: bool,
    /// Fix the display aspect ratio without re-encoding (16:9, 9:16, 1:1, ...)
    #[arg(long)]
    pub aspect: Option<String>,
    /// Fragmented MP4 output (frag_keyframe+empty_moov+default_base_moof —
    /// stream-friendly container for HLS/DASH/live pipelines)
    #[arg(long)]
    pub frag: bool,
    /// Drop subtitle and data streams in the repack (clean deliverable —
    /// mkv with embedded subs → bare mp4)
    #[arg(long)]
    pub no_subs: bool,
    /// Drop video streams in the repack (audio deliverable that keeps its
    /// cover art/subtitles/attachments — unlike --audio which rips ONLY
    /// the audio track)
    #[arg(long)]
    pub no_video: bool,
    /// Drop audio streams in the repack (silent deliverable that keeps
    /// picture/subtitles/cover — screencast library, muted B-roll
    /// master; unlike --video which rips ONLY the video track)
    #[arg(long)]
    pub no_audio: bool,
    /// Drop attachment streams in the repack (embedded fonts/payloads —
    /// slim down a file whose subtitles keep their styling via
    /// installed-system fonts; --no-cover only removes attached_pic
    /// cover art, this removes the rest)
    #[arg(long)]
    pub no_attachments: bool,
    /// Drop data streams in the repack — telemetry/timed-metadata tracks
    /// (GoPro gpmd, camera private data; `probe.has_data` tells you there's
    /// something to strip; conflicts with --keep/--drop/--program)
    #[arg(long)]
    pub no_data: bool,
    /// Constant mux rate on .ts/.m2ts targets (-muxrate R) — broadcast
    /// transport-stream spec pads the mux to a fixed bitrate (10M style
    /// values ok); mpegts outputs only
    #[arg(long)]
    pub muxrate: Option<String>,
    /// Override the major_brand atom on .mp4/.mov targets (-brand, e.g.
    /// mp42) — device ingest that rejects the default isom brand (Smart
    /// TVs, car units); mp4/mov muxers only
    #[arg(long)]
    pub brand: Option<String>,
    /// Strip embedded container chapters in the repack — clean audio
    /// deliverable/clip for players that show a broken TOC
    #[arg(long)]
    pub no_chapters: bool,
    /// Lossless trim: start the repack at SEC (keyframe-accurate seek —
    /// repackage just a segment without re-encoding)
    #[arg(long)]
    pub from: Option<f64>,
    /// ..end the repack at SEC (input-timeline position; needs the
    /// segment to land on or before a keyframe boundary)
    #[arg(long)]
    pub to: Option<f64>,
    /// Keep only audio tracks tagged with this language (ISO-639: eng,
    /// jpn…) — multi-language releases, dub extraction; with --audio =
    /// rip just that language's track; fails when no track matches
    #[arg(long)]
    pub lang: Option<String>,
    /// Keep only subtitle tracks tagged with these languages (comma list
    /// like --lang; multi-language releases — keep just the captions your
    /// audience reads; fails when no track matches)
    #[arg(long)]
    pub sub_lang: Option<String>,
    /// Make audio track N (0-based among audio streams) the default on
    /// multi-track files — players pick this one first (fixes the wrong
    /// language playing on a multi-language release)
    #[arg(long)]
    pub default_audio: Option<usize>,
    /// Attach this image as cover art on the repack (audio rips get feed
    /// art, video files get a thumbnail poster — attached_pic stream)
    #[arg(long)]
    pub cover: Option<PathBuf>,
    /// Embed container chapters from a YouTube-format list ('mm:ss title'
    /// per line — same file `chapter --yt` exports) on the repack
    #[arg(long)]
    pub chapters: Option<PathBuf>,
    /// Container title tag on the repack (music-library metadata without
    /// a re-encode)
    #[arg(long)]
    pub title: Option<String>,
    /// Container artist tag on the repack
    #[arg(long)]
    pub artist: Option<String>,
    /// Container album tag on the repack
    #[arg(long)]
    pub album: Option<String>,
    /// Container genre tag on the repack
    #[arg(long)]
    pub genre: Option<String>,
    /// Container comment tag on the repack
    #[arg(long)]
    pub comment: Option<String>,
    /// Container date tag on the repack
    #[arg(long)]
    pub date: Option<String>,
    /// Strip ALL container metadata on the repack (privacy wipe before
    /// upload — camera/GPS/app tags; combine with --title etc to retag)
    #[arg(long)]
    pub strip_meta: bool,
    /// Drop attached_pic (cover art) streams on the repack — slim a tagged
    /// audio book/m4a back to bare tracks; conflicts with --cover
    #[arg(long)]
    pub no_cover: bool,
    /// CENC AES-CTR encrypt the repacked essence (mp4/mov only — DRM prep
    /// for ClearKey/Widevine/PlayReady workflows)
    #[arg(long)]
    pub encrypt: bool,
    /// 32-hex content key for --encrypt (random when omitted)
    #[arg(long)]
    pub key: Option<String>,
    /// 32-hex key identifier for --encrypt (random when omitted)
    #[arg(long)]
    pub kid: Option<String>,
    /// Shift the audio track by SEC seconds against the video — lip-sync
    /// repair without re-encoding (positive delays audio, negative
    /// advances it; requires the video stream to be kept)
    #[arg(long, allow_negative_numbers = true)]
    pub audio_delay: Option<f64>,
    /// Shift the VIDEO track by SEC against the audio — the other half of
    /// lip-sync repair (capture cards that lag the picture; positive
    /// delays video, negative advances it)
    #[arg(long, allow_negative_numbers = true)]
    pub video_delay: Option<f64>,
    /// Retag the video stream's codec tag (`hvc1` makes HEVC mp4s play on
    /// Apple/QuickTime/Safari without re-encoding) — mp4/mov only
    #[arg(long)]
    pub tag: Option<String>,
    /// Attach a file into the repack as a binary attachment (matroska/webm
    /// only — subtitle fonts, licence PDFs); repeatable
    #[arg(long)]
    pub attach: Vec<PathBuf>,
    /// Write a start timecode into the repack (HH:MM:SS[:FF]) — mov/mp4 get
    /// a real tmcd track, mkv a TIMECODE tag; dailies matching a slate
    #[arg(long)]
    pub timecode: Option<String>,
    /// Make subtitle track N the player default (multi-language sub
    /// deliverables — same idea as --default-audio)
    #[arg(long)]
    pub default_sub: Option<usize>,
    /// Make video track N (per-type index) the player default —
    /// multi-angle files pick the hero angle players start on
    #[arg(long)]
    pub default_video: Option<usize>,
    /// Retime the whole container by a factor — re-stamp timestamps without
    /// re-encoding (1.042 = PAL 25→24 pull-down, 0.96 = film→PAL speed-up
    /// for broadcast; audio pulls with the picture)
    #[arg(long)]
    pub itsscale: Option<f64>,
    /// Shift every output timestamp by SEC — repairs negative/odd start
    /// times on capture files (players that can't seek negative ts)
    #[arg(long)]
    pub offset: Option<f64>,
    /// Keep + reorder audio tracks by per-type index (comma list: `1,0`
    /// swaps the first two tracks, unlisted tracks are dropped) — players
    /// that only play track 0 need the program mix first
    #[arg(long)]
    pub audio_order: Option<String>,
    /// Keep + reorder subtitle tracks by per-type index (comma list like
    /// --audio-order) — put the audience's captions first on multi-sub
    /// releases; unlisted tracks are dropped
    #[arg(long)]
    pub sub_order: Option<String>,
    /// Keep + reorder video tracks by per-type index (comma list like
    /// --audio-order) — multi-angle/multi-cam files pick the hero angle
    /// or reorder angles; unlisted video tracks are dropped
    #[arg(long)]
    pub video_order: Option<String>,
    /// Mark subtitle track N as FORCED (film-style forced-only captions —
    /// players auto-show them for the audience's language without a
    /// manual pick; pairs with --default-sub)
    #[arg(long)]
    pub forced_sub: Option<usize>,
    /// Flag subtitle track N as SDH/hearing-impaired (accessibility spec —
    /// players label it "SDH"; mkv/webm only, mp4 drops the flag silently)
    #[arg(long)]
    pub sdh: Option<usize>,
    /// Flag audio track N as a commentary track (director's commentary —
    /// mkv/webm only, mp4 drops the flag silently)
    #[arg(long)]
    pub commentary: Option<usize>,
    /// Flag audio track N as a visual-impaired/audio-description track
    /// (the 4.4-correct AD flag — -disposition +visual_impaired; mkv/webm
    /// only, mp4 drops the flag silently)
    #[arg(long)]
    pub audio_desc: Option<usize>,
    /// Flag audio track N as a dub track (dubbed-language track for
    /// multi-language files — mkv/webm only)
    #[arg(long)]
    pub dub: Option<usize>,
    /// Flag audio track N as the ORIGINAL-language track (the flip side
    /// of --dub — media servers pick original vs dub by it; mkv/webm only)
    #[arg(long)]
    pub original: Option<usize>,
    /// Keep ONLY the listed absolute stream indices (comma list,
    /// e.g. `0,3` keeps video 0 + audio 3 — the escape hatch when
    /// --audio-order/--sub-order/--lang can't express the pick; unlisted
    /// streams drop)
    #[arg(long)]
    pub keep: Option<String>,
    /// Drop ONLY these absolute stream indices, comma list — inverse of
    /// --keep (pull one commentary track / one language out, keep the rest)
    #[arg(long)]
    pub drop: Option<String>,
    /// Decrypt a CENC-encrypted input while repacking — 32-hex AES-CTR
    /// key (ClearKey receipt files; pair with --encrypt to rotate keys)
    #[arg(long)]
    pub decrypt: Option<String>,
    /// Copy input timestamps verbatim (-copyts — capture pipelines that
    /// need wall-clock pts preserved instead of re-zeroed)
    #[arg(long)]
    pub copy_ts: bool,
    /// Regenerate missing/broken timestamps on the way in (-fflags
    /// +genpts) — repair for camera/truncated files whose pts holes make
    /// them seek badly or probe at zero duration
    #[arg(long)]
    pub genpts: bool,

    /// Codec bitstream repair during the copy, repeatable —
    /// annexb|hevc-annexb|adts|mp3-hdr|eac3-core|dca-core|mjpeg-jpg|
    /// fix-subs|redundant-pps|extract-extra (several same-stream
    /// filters comma-join, e.g. --bsf annexb --bsf redundant-pps)
    #[arg(long, value_enum)]
    pub bsf: Vec<RemuxBsf>,

    /// Video track timescale on .mp4/.m4v/.mov outputs
    /// (-video_track_timescale) — ingest specs that pin the movie
    /// clock (600 for old QuickTime, 90000/30000 for broadcast)
    #[arg(long)]
    pub timescale: Option<u32>,

    /// Keep only program N's streams from a multi-service transport
    /// stream (program number from `probe.programs[]` — the whole
    /// service at once; conflicts with the stream-pick flags)
    #[arg(long)]
    pub program: Option<u32>,
    /// Service name in the .ts SDT table (-metadata service_name —
    /// the channel label broadcast/IPTV ingest reads; .ts/.m2ts only)
    #[arg(long)]
    pub service_name: Option<String>,
    /// Service provider in the .ts SDT table (-metadata
    /// service_provider — the network label; .ts/.m2ts only)
    #[arg(long)]
    pub provider: Option<String>,
    /// Service ID in the .ts PAT (-mpegts_service_id — the program
    /// number the service lists under; .ts/.m2ts only)
    #[arg(long)]
    pub service_id: Option<u32>,
    /// Transport stream ID (-mpegts_transport_stream_id — multiplex
    /// identity for DVB ingest; .ts/.m2ts only)
    #[arg(long)]
    pub tsid: Option<u32>,
    /// Original network ID (-mpegts_original_network_id — network
    /// identity for DVB ingest; .ts/.m2ts only)
    #[arg(long)]
    pub network_id: Option<u32>,
    /// First elementary-stream PID (-mpegts_start_pid 32-8186 —
    /// DVB/IPTV ingest PID plans allocate channels by PID; .ts/.m2ts only)
    #[arg(long)]
    pub start_pid: Option<u32>,
    /// First PMT PID (-mpegts_pmt_start_pid 32-8186 — the PID plan's
    /// program-map table slot; .ts/.m2ts only)
    #[arg(long)]
    pub pmt_pid: Option<u32>,
    /// Reemit PAT/PMT with every packet (-mpegts_flags resend_headers —
    /// join-in-progress playback on mid-stream captures; .ts/.m2ts only)
    #[arg(long)]
    pub resend_headers: bool,
    /// LATM/LOAS-encapsulate AAC audio in the transport stream
    /// (-mpegts_flags latm — DVB/ATSC broadcast spec variant; .ts only)
    #[arg(long)]
    pub latm: bool,
    /// Blu-ray m2ts packet mode (-mpegts_m2ts_mode — 192-byte packets +
    /// BD PID plan: video 0x1011, audio 0x1100; .ts/.m2ts targets only)
    #[arg(long)]
    pub m2ts: bool,
    /// CMAF-interoperable fragmented mp4 (-movflags +cmaf — one chunk
    /// pack playable as both HLS fMP4 and DASH; mp4/mov targets only)
    #[arg(long)]
    pub cmaf: bool,
    /// Write all metadata as mdta atom keys (-movflags +use_metadata_tags
    /// — HandBrake-style custom tags the stock udta writer can't express;
    /// mp4/mov targets only)
    #[arg(long)]
    pub mdta: bool,
    /// Skip the mfra trailer on fragmented mp4 (-movflags +skip_trailer —
    /// live ingest pipelines that never seek back don't need the random-
    /// access trailer; needs --frag; mp4/mov targets only)
    #[arg(long)]
    pub skip_trailer: bool,
    /// Write the Smooth Streaming prologue (-movflags +isml — piif/uuid
    /// boxes IIS Smooth Streaming ingest looks for; mp4/mov targets only)
    #[arg(long)]
    pub isml: bool,
    /// Add an RTP hint track per media stream (-movflags +rtphint —
    /// live555/Darwin Streaming ingest prep; mp4/mov targets only)
    #[arg(long)]
    pub rtphint: bool,
    /// -bitexact deterministic muxing — normalized encoder tag + headers:
    /// same input + same ffkit version → byte-identical output (archival
    /// reproducibility / dedupe QC)
    #[arg(long)]
    pub bitexact: bool,
    /// Second container written in the same pass via the tee muxer (e.g.
    /// social .mp4 + broadcast .ts from one repack — the same streams land
    /// in both outputs; mp4/mov/ts/m2ts/flv for video inputs, mp3/wav/aac/
    /// flac/ogg/m4a for audio-only; mkv/webm can't be tee slaves)
    #[arg(long)]
    pub also: Option<PathBuf>,
    /// Force a colr atom into mp4-family masters (-movflags +write_colr —
    /// platform QC that requires the atom even when color metadata is
    /// unspecified; mp4/mov/m4a targets only)
    #[arg(long)]
    pub colr: bool,
    /// Write a Producer Reference Time box per fragment (-write_prft —
    /// LL-DASH/CMAF ingest latency measurement; needs --frag; mp4/mov
    /// targets only)
    #[arg(long)]
    pub prft: bool,
    /// Fragment granularity cap in seconds (-frag_duration — moof
    /// boundaries at least this often, keyframe-aligned; needs --frag;
    /// mp4/mov targets only)
    #[arg(long)]
    pub frag_duration: Option<f64>,
    /// Fragment size cap in bytes (-frag_size — close a fragment early
    /// when it exceeds N bytes; needs --frag; mp4/mov targets only)
    #[arg(long)]
    pub frag_size: Option<u64>,
    /// Matroska cluster granularity in milliseconds (-cluster_time_limit
    /// — tighter seek granularity on archive masters; mkv/webm targets
    /// only)
    #[arg(long)]
    pub cluster: Option<u32>,
    /// Matroska cluster byte cap (-cluster_size_limit — close a cluster
    /// early when it exceeds N bytes; mkv/webm targets only)
    #[arg(long = "cluster-size")]
    pub cluster_size: Option<u64>,
    /// Copy global metadata tags from FILE (-map_metadata — apply a
    /// tagged template's title/artist/comment keys to the repack)
    #[arg(long = "meta-from", value_name = "FILE")]
    pub meta_from: Option<PathBuf>,
    /// Copy chapter marks from FILE (-map_chapters — transplant the
    /// chaptered mix's marks onto the master)
    #[arg(long = "chapters-from", value_name = "FILE")]
    pub chapters_from: Option<PathBuf>,
    /// ID3v2 tag version 3 or 4 for .mp3 outputs (-id3v2_version — car
    /// stereos and old feature phones only read v2.3)
    #[arg(long = "id3v2", value_name = "VER")]
    pub id3v2: Option<u32>,
    /// Append a legacy ID3v1 tag on .mp3 outputs (-write_id3v1 —
    /// ancient devices; carries title/artist/album when metadata exists)
    #[arg(long)]
    pub id3v1: bool,
    /// Broadcast-WAV BEXT chunk on .wav outputs (-write_bext — loudness
    /// and originator fields for broadcast ingest)
    #[arg(long)]
    pub bext: bool,
    /// Peak-envelope chunk on .wav outputs (-write_peak on — the levl
    /// chunk DAWs and ingest QC read for fast waveform/loudness display)
    #[arg(long)]
    pub peak: bool,
    /// Force an RF64 header on .wav outputs even under 4GB (-rf64 always
    /// — strict broadcast specs that want RF64 from the first byte)
    #[arg(long)]
    pub rf64: bool,
    /// Delay the moov atom until the first fragment flushes (-movflags
    /// +delay_moov — live/simulcast ingest that needs stream headers
    /// before any media lands; needs --frag, mp4/mov only)
    #[arg(long)]
    pub delay_moov: bool,
    /// Write each moof as a separate atom per stream fragment
    /// (-movflags +separate_moof — CDN/origin ingest specs that want
    /// strict moof/mdat alternation; needs --frag, mp4/mov only)
    #[arg(long)]
    pub separate_moof: bool,
    /// Force a tmcd timecode data track (-write_tmcd 1 — broadcast
    /// 2-pop/dailies that must carry an explicit TC track; needs
    /// --timecode, mp4/mov only)
    #[arg(long)]
    pub tmcd: bool,
    /// Transport-stream lead-in buffer in seconds (-muxpreload —
    /// buffering headroom the mux keeps before data lands; broadcast
    /// ingest lip-sync specs, .ts/.m2ts only)
    #[arg(long)]
    pub mux_preload: Option<f64>,
    /// Delay every stream by SEC seconds on mux (-muxdelay — shifts
    /// start_time so audio/video land on the broadcast clock;
    /// .ts/.m2ts only)
    #[arg(long)]
    pub mux_delay: Option<f64>,
    /// Keep the moov index at the END of the file (skip +faststart —
    /// append-friendly capture outputs and editors that mux-track
    /// layout; mp4/mov/m4a only, conflicts --frag)
    #[arg(long)]
    pub no_faststart: bool,
    /// Omit the Xing/Info VBR header on .mp3 outputs (-write_xing 0 —
    /// some firmware/players misread duration with it, and fixed
    /// bitrate jobs don't need it)
    #[arg(long)]
    pub no_xing: bool,
    /// Embed a keyframe index in the FLV metadata (-flvflags
    /// add_keyframe_index — the onMetaData keyframes table Flash-era
    /// players and some RTMP ingest tools read for scrubbing)
    #[arg(long)]
    pub flv_index: bool,
}

#[derive(clap::Args, Debug)]
pub struct MemeArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Top caption text
    #[arg(long)]
    pub top: Option<String>,
    /// Bottom caption text
    #[arg(long)]
    pub bottom: Option<String>,
    #[arg(long)]
    pub font: Option<String>,
    /// Text size multiplier (default 1.0)
    #[arg(long, default_value_t = 1.0)]
    pub size: f64,
    /// Text color as RRGGBB hex (default ffffff)
    #[arg(long)]
    pub color: Option<String>,
    /// Classic meme outline thickness in px (0 = off). Black outline, white text.
    #[arg(long, default_value_t = 0)]
    pub outline: u32,
    /// Draw the meme text at this % opacity (0-100 — ghost/watermark text)
    #[arg(long)]
    pub opacity: Option<f64>,
    /// Show the text only inside this window — comma list for several spots (needs --dur)
    #[arg(long)]
    pub at: Option<String>,
    /// Window length in seconds (default: to the end)
    #[arg(long)]
    pub dur: Option<f64>,
    /// Text block placement: top (classic), center (mid-screen stacked),
    /// bottom (paired captions low — out of the UI zone)
    #[arg(long, value_enum)]
    pub position: Option<MemePos>,
    /// Word-wrap meme text at N columns (≥4)
    #[arg(long)]
    pub wrap: Option<u32>,
    /// Per-line alignment inside each meme card (with --wrap)
    #[arg(long, value_enum)]
    pub align: Option<crate::raster::TextAlign>,
    /// Fade each text card in/out over SEC seconds at the window edges (with --at/--dur)
    #[arg(long)]
    pub fade: Option<f64>,
}
#[derive(clap::ValueEnum, Clone, Copy, Debug)]
pub enum MemePos {
    Top,
    Center,
    Bottom,
}

#[derive(clap::Args, Debug)]
pub struct VoiceArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Gate threshold dB — quieter than this gets muted (default -45)
    #[arg(long, default_value_t = -45.0, allow_hyphen_values = true)]
    pub threshold: f64,
    /// Target integrated loudness LUFS (default -16 podcast)
    #[arg(long, default_value_t = -16.0, allow_hyphen_values = true)]
    pub lufs: f64,
    /// Polish only from this time on (h:mm:ss or seconds)
    #[arg(long)]
    pub at: Option<String>,
    /// ..for this many seconds (default: to the end)
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(clap::Args, Debug)]
pub struct DeinterlaceArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// frame = same rate progressive (default); field = double rate smoothest
    #[arg(long, value_enum, default_value_t = DeinterlaceMode::Frame)]
    pub mode: DeinterlaceMode,
    /// Field order: auto, tff (top-first), bff (bottom-first). Wrong = judder.
    #[arg(long, value_enum, default_value_t = FieldParity::Auto)]
    pub parity: FieldParity,
    /// Deinterlacer engine (default yadif; bwdif smoother motion, estdif
    /// edge-slope tracing for diagonal lines, kerndeint adaptive kernel)
    #[arg(long, value_enum)]
    pub engine: Option<DeintEngine>,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum DeclipEngine {
    /// adeclip — interpolate clipped peaks back
    Clip,
    /// adeclick — remove impulsive clicks and pops
    Click,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum SmoothEngine {
    /// smartblur — box blur that skips real edges (default beauty mode)
    Smartblur,
    /// bilateral — edge-aware Gaussian (skin texture kept, noise dropped)
    Bilateral,
    /// deflate — morphological: pulls bright peaks down (pore/texture smooth)
    Deflate,
    /// inflate — morphological: lifts dark valleys up
    Inflate,
    /// uspp — MPEG post-processor deblock+dering: rescues over-compressed
    /// rips (re-uploads, low-bitrate sources) without the soft look of blur
    Uspp,
    /// pp7 — lighter/faster postproc deblock (spp sibling): when uspp is too slow
    Pp7,
    /// yaepblur — edge-preserving smoothing (bilateral-class, keeps contours)
    Yaep,
    /// spp — simple postprocess DCT deblock (lighter than uspp/pp7)
    Spp,
    /// fspp — fast spp variant (oldest, blockiest sources)
    Fspp,
    /// sab — shape-adaptive blur: smoothes inside flat regions without
    /// crossing object edges (matte-style cleanup, skin)
    Sab,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum DeintEngine {
    Yadif,
    Bwdif,
    Estdif,
    Kerndeint,
    /// fieldmatch + decimate — inverse telecine: 29.97i film content back to
    /// 23.976p (anime/film transfers). Not a per-field deinterlacer.
    Fieldmatch,
    /// detelecine — deterministic inverse telecine for a known 3:2 cadence
    /// (pattern=23, no comb analysis: frame-exact when the cadence is clean)
    Detelecine,
    /// mcdeint — motion-compensated deinterlacer (archive-quality, slow)
    Mcdeint,
    /// w3fdif — Martin Weston three-field filter (sharp diagonal edges, SD archives)
    W3fdif,
    /// separatefields — split each field into its own frame: 25i/29.97i
    /// becomes 50p/59.94p (smooth slow-mo source, sports frame stepping)
    Separate,
    /// pullup — inverse 3:2 pulldown IVTC (telecined NTSC → progressive)
    Pullup,
    /// phase — field-phase reorder: swaps field order when a capture has
    /// wrong parity (jumpy interlaced playback, no real deinterlacing needed)
    Phase,
    /// field — extract the top field only (half-height progressive, no
    /// interpolation): the fastest possible deinterlace — preview/rough-cut
    /// quality when yadif is overkill
    Field,
}

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum FieldParity {
    #[default]
    Auto,
    /// Top field first (most DV/HDV)
    Tff,
    /// Bottom field first (some DV, PAL)
    Bff,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum DeinterlaceMode {
    Frame,
    Field,
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub enum DedustEngine {
    /// erosion/dilation morphology on the luma plane (default)
    Morpho,
    /// tlut2 temporal min/max — one-frame sparkles & dropouts
    Temporal,
}

#[derive(clap::Args, Debug)]
pub struct DedustArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Speck size in px (1-4, default 1)
    #[arg(long, default_value_t = 1)]
    pub size: u32,
    /// Remove bright specks (white dust on scans, hot pixels) — default
    #[arg(long, default_value_t = false)]
    pub light: bool,
    /// Remove dark specks (film-negative dust, dead pixels)
    #[arg(long, default_value_t = false)]
    pub dark: bool,
    /// Dedust engine — temporal kills one-frame sparkles (film dust,
    /// VHS dropouts) the morphology pass misses
    #[arg(long, value_enum)]
    pub engine: Option<DedustEngine>,
    /// Dedust only from this time (needs --dur)
    #[arg(long)]
    pub at: Option<String>,
    /// ..for this many seconds
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum SpillType {
    #[default]
    Green,
    Blue,
}

#[derive(clap::Args, Debug)]
pub struct DespillArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Screen colour being removed
    #[arg(long, value_enum, default_value_t = SpillType::Green)]
    pub kind: SpillType,
    /// Spillmap mix 0-1 (higher removes more spill, may desaturate edges)
    #[arg(long, default_value_t = 0.5)]
    pub mix: f64,
    /// Expand the spill map 0-1 (catch fringe pixels)
    #[arg(long, default_value_t = 0.0)]
    pub expand: f64,
    /// Despill only from this time (needs --dur)
    #[arg(long)]
    pub at: Option<String>,
    /// ..for this many seconds
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(clap::Args, Debug)]
pub struct ExtendArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Stretch edge pixels to fill a left border this wide (px)
    #[arg(long, default_value_t = 0)]
    pub left: u32,
    /// ..right border
    #[arg(long, default_value_t = 0)]
    pub right: u32,
    /// ..top border
    #[arg(long, default_value_t = 0)]
    pub top: u32,
    /// ..bottom border
    #[arg(long, default_value_t = 0)]
    pub bottom: u32,
    /// Fill mode: smear (default edge-stretch), mirror, fixed, reflect, wrap, fade
    #[arg(long, value_enum, default_value_t = ExtendMode::Smear)]
    pub mode: ExtendMode,
}

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum InterpMode {
    /// Motion-compensated interpolation (best quality, slowest)
    #[default]
    Mci,
    /// Cheap frame blending
    Blend,
    /// Duplicate frames (convert fps only, no smoothing)
    Dup,
}

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum ColorMatrix {
    /// Auto (reads the stream's colorspace tag — source side only)
    #[default]
    Auto,
    Bt709,
    /// BT.601 / SMPTE-170M — SD NTSC/PAL masters
    Bt601,
    Smpte240m,
    Bt2020,
    Fcc,
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub enum MatrixEngine {
    /// colormatrix — matrix coefficient conversion (fast, tag-safe)
    Colormatrix,
    /// colorspace — full conversion incl. primaries + transfer curve
    /// (gamut-aware; the bt2020 ↔ bt709 path)
    Colorspace,
}

#[derive(clap::Args, Debug)]
pub struct MatrixArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Source matrix (default: auto — read the stream tag)
    #[arg(long, value_enum, default_value_t = ColorMatrix::Auto)]
    pub from: ColorMatrix,
    /// Destination matrix — convert into this space
    #[arg(long, value_enum, default_value_t = ColorMatrix::Bt709)]
    pub to: ColorMatrix,
    /// Conversion engine — colorspace also maps primaries + transfer
    /// (bt2020 HDR ↔ 709 SDR gamut work), not just the matrix coeff
    #[arg(long, value_enum)]
    pub engine: Option<MatrixEngine>,
    /// Convert only from this time (needs --dur)
    #[arg(long)]
    pub at: Option<String>,
    /// ..for this many seconds
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(clap::Args, Debug)]
pub struct LegalizeArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Luma floor (default 16, broadcast-safe)
    #[arg(long, default_value_t = 16)]
    pub min: u32,
    /// Luma ceiling (default 235, broadcast-safe)
    #[arg(long, default_value_t = 235)]
    pub max: u32,
    /// Clamp only from this time (needs --dur)
    #[arg(long)]
    pub at: Option<String>,
    /// ..for this many seconds
    #[arg(long)]
    pub dur: Option<f64>,
    /// Damp photosensitive-epilepsy flash cuts (safe-delivery pass —
    /// `scan` reports the same flashes as flash_frames/badness)
    #[arg(long)]
    pub flash: bool,
    /// Flash detection strictness (default 1.0; lower = stricter)
    #[arg(long)]
    pub flash_threshold: Option<f64>,
}

#[derive(clap::Args, Debug)]
pub struct LevelsArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Input black point 0-1 (everything darker → output black; crush rescue
    /// for lifted web rips uses ~0.06)
    #[arg(long, default_value_t = 0.0)]
    pub in_min: f64,
    /// Input white point 0-1 (everything brighter → output white)
    #[arg(long, default_value_t = 1.0)]
    pub in_max: f64,
    /// Output black point 0-1 (raise for matte/film fade)
    #[arg(long, default_value_t = 0.0)]
    pub out_min: f64,
    /// Output white point 0-1
    #[arg(long, default_value_t = 1.0)]
    pub out_max: f64,
    /// Remap only from this time (needs --dur)
    #[arg(long)]
    pub at: Option<String>,
    /// ..for this many seconds
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(clap::Args, Debug)]
pub struct AberrateArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Fringe width in px (red goes left, blue right)
    #[arg(long, default_value_t = 3)]
    pub amount: i32,
    /// Aberrate only from this time — comma list works (needs --dur)
    #[arg(long)]
    pub at: Option<String>,
    /// ..for this many seconds
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(clap::Args, Debug)]
pub struct InterpArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Output frame rate (default 60; use --fps 120 for high-refresh delivery)
    #[arg(long)]
    pub fps: Option<f64>,
    /// Smooth slow-mo instead of upfps: factor 0-1 (0.5 = half speed at the
    /// source frame rate, motion-compensated in-betweens)
    #[arg(long)]
    pub slow: Option<f64>,
    /// Interpolation engine
    #[arg(long, value_enum, default_value_t = InterpMode::Mci)]
    pub mode: InterpMode,
    /// Interp filter: minterpolate (default, motion-compensated, slow+smooth)
    /// | framerate (scene-aware frame blending — ~10x faster, slight ghost)
    #[arg(long, value_enum)]
    pub engine: Option<InterpEngine>,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
pub enum InterpEngine {
    Minterpolate,
    Framerate,
}

#[derive(clap::Args, Debug)]
pub struct DejudderArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Pullup cycle length (default 4 for 3:2 telecine judder)
    #[arg(long, default_value_t = 4)]
    pub cycle: u32,
}

#[derive(clap::Args, Debug)]
pub struct TonemapArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Tone curve: hable (default filmic S-curve), reinhard, gamma, clip, linear
    #[arg(long, value_enum, default_value_t = TonemapAlgo::Hable)]
    pub algo: TonemapAlgo,
    /// HDR peak in nits the curve maps from (default 100 = SDR-normalized)
    #[arg(long, default_value_t = 100.0)]
    pub peak: f64,
}

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum TonemapAlgo {
    #[default]
    Hable,
    Reinhard,
    Gamma,
    Clip,
    Linear,
}

#[derive(clap::Args, Debug)]
pub struct TelecineArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Field pattern string — "23" = 3:2 NTSC pulldown (default)
    #[arg(long, default_value = "23")]
    pub pattern: String,
    /// First field emitted
    #[arg(long, value_enum, default_value_t = FieldParity::Tff)]
    pub field: FieldParity,
}

#[derive(clap::Args, Debug)]
pub struct PremultArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Straight → premultiplied, or back with unpremultiply
    #[arg(long, value_enum, default_value_t = PremultMode::Premultiply)]
    pub mode: PremultMode,
}

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum PremultMode {
    #[default]
    Premultiply,
    Unpremultiply,
}

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum ExtendMode {
    #[default]
    Smear,
    Mirror,
    Fixed,
    Reflect,
    Wrap,
    Fade,
}

#[derive(clap::Args, Debug)]
pub struct CrossfadeArgs {
    /// First audio file (its tail fades out)
    pub input: PathBuf,
    /// Second audio file (fades in under the tail)
    #[arg(long)]
    pub second: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Overlap seconds (default 2)
    #[arg(long, default_value_t = 2.0)]
    pub dur: f64,
}

#[derive(clap::Args, Debug)]
pub struct StripArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
}

#[derive(clap::Args, Debug)]
pub struct FramesArgs {
    pub input: PathBuf,
    /// Output path or template — `shots.png` writes `shots_001.png`…
    #[arg(short, long)]
    pub output: PathBuf,
    /// Seconds between stills (default 5)
    #[arg(long, default_value_t = 5.0)]
    pub every: f64,
    /// Optional width to scale stills to
    #[arg(long)]
    pub width: Option<u32>,
    /// Grab stills at these timestamps instead of an --every grid
    #[arg(long, value_delimiter = ',')]
    pub at: Vec<String>,
    /// N evenly-spaced stills across the clip (overrides --every)
    #[arg(long)]
    pub count: Option<u32>,
    /// Every Nth frame instead of a time grid — dataset/QC sampling
    /// (overrides --every; e.g. --nth 10 on 30fps = 3 stills/sec)
    #[arg(long)]
    pub nth: Option<u32>,
    /// Grab exactly frame N (0-based, decoded order — pinpoint a known-bad
    /// frame by index; select + vsync 0, comma list grabs several
    /// indices, overrides --every/--at/--count)
    #[arg(long)]
    pub number: Option<String>,
    /// Split every frame into a COLSxROWS tile sequence instead —
    /// breaks a contact-sheet/mosaic back into per-tile stills (untile)
    #[arg(long)]
    pub untile: Option<String>,
}

#[derive(clap::Args, Debug)]
pub struct InvertArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Invert only from this time — comma list for several windows (needs --dur)
    #[arg(long)]
    pub at: Option<String>,
    /// ..for this many seconds (default: to the end)
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(clap::Args, Debug)]
pub struct CountdownArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Count down from N (default 3: 3-2-1)
    #[arg(long, default_value_t = 3)]
    pub from: u32,
    /// Count down to this local wall-clock time (HH:MM or HH:MM:SS) —
    /// premiere/stream-start overlay that ends at a real time. 1s per
    /// count, max 10 min ahead; a passed time rolls to tomorrow
    #[arg(long)]
    pub target: Option<String>,
    /// Seconds each number stays (default 1)
    #[arg(long, default_value_t = 1.0)]
    pub each: f64,
    /// Optional text shown after the count (e.g. "GO!")
    #[arg(long)]
    pub go: Option<String>,
    /// Start the countdown at this time (default 0; `end` = last ~50ms)
    #[arg(long)]
    pub at: Option<String>,
    #[arg(long)]
    pub font: Option<String>,
    /// Text size multiplier (default 3 — big center numerals)
    #[arg(long, default_value_t = 3.0)]
    pub size: f64,
    /// Text color as RRGGBB hex (default ffffff)
    #[arg(long)]
    pub color: Option<String>,
    /// Beep 880Hz for 120ms at the start of each count
    #[arg(long)]
    pub beep: bool,
    /// Beep frequency in Hz (default 880)
    #[arg(long)]
    pub tone: Option<f64>,
    /// Digit format: `s` seconds (default), `mm:ss`, `h:mm:ss` — long countdowns
    #[arg(long, default_value = "s")]
    pub format: String,
    /// Label shown above the digits for the whole count ("STARTING SOON")
    #[arg(long)]
    pub text: Option<String>,
    /// Card plate behind the numerals (name or RRGGBB)
    #[arg(long)]
    pub bg: Option<String>,
    /// Digit placement (default center): top|bottom|corners|edges
    #[arg(long)]
    pub position: Option<String>,
    /// Draw at this % opacity (0-100 — ghost/watermark overlay)
    #[arg(long)]
    pub opacity: Option<f64>,
    /// Read the wall clock in UTC instead of local time (cross-timezone
    /// premieres — --target HH:MM compares against UTC)
    #[arg(long)]
    pub utc: bool,
}

#[derive(clap::Args, Debug)]
pub struct MixArgs {
    /// First source (video kept as-is if present)
    pub a: PathBuf,
    /// Second source (audio merged in)
    pub b: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Linear level for input A (default 1.0)
    #[arg(long, default_value_t = 1.0)]
    pub vol_a: f64,
    /// Linear level for input B (default 1.0)
    #[arg(long, default_value_t = 1.0)]
    pub vol_b: f64,
    /// Output runs until the LONGER input ends (default: first input's length)
    #[arg(long)]
    pub longest: bool,
    /// Loop B if it is shorter than the output span (short beds)
    #[arg(long = "loop")]
    pub loop_track: bool,
    /// Sidechain-duck B under A's voice (podcast music bed)
    #[arg(long)]
    pub duck: bool,
    /// Harder duck: sidechaingate mutes B while A speaks (vs compressor's
    /// smooth dip) — talk-show music bed
    #[arg(long)]
    pub gate: bool,
    /// Let amix normalize the sum (halves level for two hot tracks)
    #[arg(long)]
    pub normalize: bool,
    /// Bring B in only from this time — comma list for several entrances (needs --dur)
    #[arg(long)]
    pub at: Option<String>,
    /// Fade B in/out over N seconds at the window edges (needs --at)
    #[arg(long)]
    pub fade: Option<f64>,
    /// ..for this many seconds (default: to the end)
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(clap::Args, Debug)]
pub struct TimerArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Start the running timer at this time (default 0; `end` = last ~50ms)
    #[arg(long)]
    pub at: Option<String>,
    /// Stop showing after this many seconds (default: to the end)
    #[arg(long)]
    pub dur: Option<f64>,
    /// bottom-right (default), top-right, top-left, bottom-left, top, bottom, center
    #[arg(long, default_value = "bottom-right")]
    pub position: String,
    #[arg(long, default_value_t = 20)]
    pub margin: i32,
    /// Text size multiplier (default 0.6 — corner counter)
    #[arg(long, default_value_t = 0.6)]
    pub size: f64,
    /// Text color as RRGGBB hex (default ffffff)
    #[arg(long)]
    pub color: Option<String>,
    #[arg(long)]
    pub font: Option<String>,
    /// Display format: hms (auto) or ms (mm:ss.cc centiseconds)
    #[arg(long, value_enum, default_value_t = TimerFormat::Hms)]
    pub format: TimerFormat,
    /// Filled card behind the digits: RRGGBB hex or color name
    #[arg(long)]
    pub box_color: Option<String>,
    /// Readout starts at N seconds (up: counts from N; down: from N to 0)
    #[arg(long)]
    pub start: Option<f64>,
    /// Burn a timecode readout HH:MM:SS:FF starting here (dailies/review
    /// copies; `;` before FF marks drop-frame intent — display only)
    #[arg(long)]
    pub tc: Option<String>,
    /// Count DOWN to the window end instead of up from --at
    #[arg(long)]
    pub down: bool,
    /// Draw at this % opacity (0-100 — ghost/watermark overlay)
    #[arg(long)]
    pub opacity: Option<f64>,
    /// Wall-clock readout HH:MM:SS seeded from the system clock at encode
    /// start (event/sports overlays — not for elapsed timing)
    #[arg(long)]
    pub clock: bool,
    /// Show the local wall date YYYY-MM-DD (static — air-date/archive
    /// overlays); with --clock it prefixes the time readout
    #[arg(long)]
    pub date: bool,
    /// With --clock/--date: read UTC instead of the local timezone
    /// (broadcast/satellite overlays, multi-region simulcast slates)
    #[arg(long)]
    pub utc: bool,
}

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum TimerFormat {
    #[default]
    Hms,
    /// mm:ss.cc
    Ms,
}

#[derive(clap::Args, Debug)]
pub struct MuteArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Silence only inside this window instead of dropping the whole track —
    /// comma list for several spots (needs --dur; `end` ok)
    #[arg(long)]
    pub at: Option<String>,
    /// Window length in seconds (default: to the end)
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(clap::Args, Debug)]
pub struct LiveArgs {
    /// Source file — or with --list a concat manifest; not needed for --test
    pub input: Option<PathBuf>,
    /// Stream destination — rtmp://host/app/key, rtmps://, tcp://host:port
    /// (plain TCP streams raw FLV — handy for local ingest tests)
    #[arg(long)]
    pub to: String,
    /// Loop the clip forever (24/7 music streams, premiere replays)
    #[arg(long = "loop")]
    pub loop_: bool,
    /// Start streaming from T seconds into the source (skip a long event's
    /// dead head — replay archives mid-way; input-side seek, fast)
    #[arg(long)]
    pub start: Option<f64>,
    /// Video bitrate for the live encode (default 2500k — ingest-safe)
    #[arg(long)]
    pub vbitrate: Option<String>,
    /// Constant-quality encode 0-51 instead of -b:v (CRF — some ingests
    /// prefer a quality target over a rate cap; conflicts with
    /// --vbitrate/--maxrate/--bufsize)
    #[arg(long)]
    pub crf: Option<u32>,
    /// Audio bitrate (default 128k)
    #[arg(long)]
    pub abitrate: Option<String>,
    /// Delay pushed audio by SEC (adelay — capture cards whose audio
    /// leads the picture; the video side stays untouched)
    #[arg(long)]
    pub audio_delay: Option<f64>,
    /// Freeze the first frame + mute audio for SEC before the feed begins
    /// (tpad clone + adelay — ingest warmup while the stream health checks;
    /// refuses --slate/--card since those ARE the lead-in)
    #[arg(long)]
    pub hold: Option<f64>,
    /// Push audio channel count — 1 mono for speech/radio ingest specs
    /// that reject stereo (refuses --no-audio)
    #[arg(long)]
    pub channels: Option<u8>,
    /// Scale pushed-audio gain 0..=4 (quiet a loud BGM source without
    /// re-rendering it; refuses --no-audio)
    #[arg(long)]
    pub volume: Option<f64>,
    /// One-pass dynamic loudnorm on the pushed audio (broadcast −23 LUFS
    /// spec — normalize ingest loudness inline; refuses --no-audio)
    #[arg(long)]
    pub loudnorm: bool,
    /// Downscale before streaming, WxH (push a 4K master to a 720p ingest)
    #[arg(long)]
    pub scale: Option<String>,
    /// Output frame rate on the stream (60fps capture → 30fps ingest)
    #[arg(long)]
    pub fps: Option<u32>,
    /// Record a local archive copy while streaming (tee muxer — encode
    /// once, mux twice; .mp4/.mov/.mkv/.ts extension picks the container)
    #[arg(long)]
    pub record: Option<PathBuf>,
    /// Also push to this second ingest URL at the same time (multistream —
    /// YouTube + Twitch in one encode; combines with --record)
    #[arg(long)]
    pub restream: Option<String>,
    /// Stop the stream automatically after SEC seconds (premiere windows)
    #[arg(long)]
    pub until: Option<f64>,
    /// Input is a concat manifest text file (`file 'a.mp4'` lines) — a
    /// 24/7 rotation channel; combine with --loop for infinite rotation
    #[arg(long)]
    pub list: bool,
    /// Stream a generated SMPTE-style test card + 1kHz tone instead of a
    /// file — verify the stream key/latency before showtime
    #[arg(long)]
    pub test: bool,
    /// Hold this image as a "starting soon" card for --slate-dur seconds
    /// before the content begins (premiere/scheduled-start countdown card —
    /// normalized to the stream canvas, silent audio bed)
    #[arg(long)]
    pub slate: Option<PathBuf>,
    /// Seconds the --slate card stays up before switching to the content
    /// (default 10)
    #[arg(long)]
    pub slate_dur: Option<f64>,
    /// Burn this image in a corner of the live feed (channel bug — scaled
    /// to ~10% of stream width)
    #[arg(long)]
    pub overlay: Option<PathBuf>,
    /// --overlay corner: tl|tr|bl|br (default br)
    #[arg(long, value_enum)]
    pub overlay_position: Option<LogoPos>,
    /// --overlay opacity 0..=1 (default 1)
    #[arg(long)]
    pub overlay_opacity: Option<f64>,
    /// Peak video bitrate the encode may burst to, like `4500k`/`6M` —
    /// platform ingest cap (Twitch ≤6000k, YouTube ≤9000k at 1080p60);
    /// paired with --bufsize for a real CBR envelope
    #[arg(long)]
    pub maxrate: Option<String>,
    /// Rate-control buffer like `9000k`/`12M` (default 2x --maxrate when
    /// only --maxrate is given; the standard CBR pairing)
    #[arg(long)]
    pub bufsize: Option<String>,
    /// Hold this image as the whole video for an audio-only source
    /// (24/7 lofi-radio style: static card + music stream)
    #[arg(long)]
    pub card: Option<PathBuf>,
    /// GOP / keyframe interval in frames (platform ingest spec —
    /// YouTube wants a keyframe every 2s ≈ 60 frames at 30fps)
    #[arg(long)]
    pub gop: Option<u32>,
    /// x264 encode preset (default veryfast; go slower for quality,
    /// faster for weak machines)
    #[arg(long, value_enum)]
    pub preset: Option<X264Preset>,
    /// Video codec for the stream encode: h264 (default) or hevc —
    /// HEVC needs an MPEG-TS transport (`--to srt://`/`udp://`, contribution
    /// links), the FLV muxer rejects it
    #[arg(long, value_enum)]
    pub codec: Option<LiveCodec>,
    /// Burn an .srt/.ass/.vtt caption file into the live picture —
    /// live-captioned broadcasts without a captioning rig
    #[arg(long, value_name = "FILE")]
    pub subs: Option<PathBuf>,
    /// Vertical-stream preset: letterbox the feed onto a 1080x1920 canvas
    /// (TikTok/Reels/抖音 live — combines with --to any ingest)
    #[arg(long)]
    pub vertical: bool,
    /// Drop the video track and stream audio only — audio podcast/radio
    /// push from any source (a concert file goes out as an aac-only feed)
    #[arg(long)]
    pub audio_only: bool,
    /// Drop the audio track and stream video only — silent ambience/
    /// surveillance feeds (the ingest program supplies its own bed)
    #[arg(long)]
    pub no_audio: bool,
    /// Show/stream title written into the FLV/TS container metadata (and
    /// the --record archive) — ingest dashboards and players display it
    #[arg(long)]
    pub title: Option<String>,
    /// Abort the push if the ingest stalls longer than SEC seconds
    /// (-rw_timeout — a wedged RTMP endpoint stops eating the program
    /// feed; single-destination pushes only)
    #[arg(long = "rw-timeout")]
    pub rw_timeout: Option<f64>,
}

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum LiveCodec {
    /// H.264 — every ingest accepts it (default)
    #[default]
    H264,
    /// HEVC via libx265 — contribution links only; pair with `--to srt://`
    /// or `udp://` (MPEG-TS), the FLV muxer can't carry it
    Hevc,
}

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum X264Preset {
    Ultrafast,
    Superfast,
    #[default]
    Veryfast,
    Faster,
    Fast,
    Medium,
    Slow,
    Slower,
    Veryslow,
}

#[derive(clap::Args, Debug)]
pub struct HlsArgs {
    pub input: PathBuf,
    /// Playlist path (out.m3u8) or a directory (→ dir/index.m3u8 + seg_*.ts)
    #[arg(short, long)]
    pub output: PathBuf,
    /// Segment length in seconds (default 4)
    #[arg(long, default_value_t = 4.0)]
    pub seg: f64,
    /// Single .ts + byte-range playlist — one file to upload instead of hundreds
    #[arg(long)]
    pub single: bool,
    /// Stream-copy the essence (fast repack; needs h264/aac input)
    #[arg(long)]
    pub copy: bool,
    /// ABR ladder: comma list of heights (e.g. 1080,720,480) → variant
    /// playlists + master.m3u8 (requires a directory -o)
    #[arg(long, value_delimiter = ',')]
    pub ladder: Vec<u32>,
    /// Audio-only stream package (-vn; podcasts, voice-over HLS)
    #[arg(long)]
    pub audio_only: bool,
    /// Video-only stream package (-an; muted/preview renditions)
    #[arg(long)]
    pub video_only: bool,
    /// Fragmented MP4 segments (CMAF; plays on Safari/AirPlay, .m4s files)
    #[arg(long)]
    pub fmp4: bool,
    /// Also write poster.jpg next to the playlist (mid-video frame for the player)
    #[arg(long)]
    pub poster: bool,
    /// Poster frame time (sec or `end`; default mid-video) — needs --poster
    #[arg(long)]
    pub poster_at: Option<String>,
    /// AES-128 encrypt the segments (writes key.bin + key.info; paywalled/private streams)
    #[arg(long)]
    pub encrypt: bool,
    /// Use this 32-hex key instead of a random one (implies --encrypt)
    #[arg(long)]
    pub key: Option<String>,
    /// URI written into the playlist for the key (default key.bin; use a CDN/auth URL for real deployments)
    #[arg(long)]
    pub key_uri: Option<String>,
    /// Reload the key file after every segment (-hls_flags
    /// periodic_rekey — rotate key material on a live packager by
    /// rewriting key.info between segments; needs --encrypt/--key)
    #[arg(long)]
    pub rekey: bool,
    /// Pin the AES-128 IV (32 hex chars — deterministic encrypted
    /// packages for reproducible builds/dedup; needs --encrypt/--key)
    #[arg(long)]
    pub enc_iv: Option<String>,
    /// Custom variant stream map, e.g.
    /// "v:0,agroup:aud a:0,agroup:aud,default:yes a:1,agroup:aud" —
    /// multi-language audio renditions / hand-shaped ABR groups
    /// (-var_stream_map raw spec over the input's v:/a: streams;
    /// output -o must be a directory; conflicts --ladder/--subs/--single/--live/--program)
    #[arg(long)]
    pub var_map: Option<String>,
    /// Write each segment + playlist to a tmp file and rename when
    /// complete (-hls_flags temp_file — live readers/nginx never see a
    /// half-written .ts or m3u8 while the packager is still writing)
    #[arg(long)]
    pub temp: bool,
    /// Round each segment duration in the playlist to whole seconds
    /// (-hls_flags round_durations — old players and strict HLS
    /// validators that reject fractional EXTINF)
    #[arg(long)]
    pub round_durations: bool,
    /// Sliding-window live playlist: keeps only the newest --live-window
    /// segments (delete_segments + omit_endlist) — self-hosted live channel
    /// fed while the input is still being written
    #[arg(long)]
    pub live: bool,
    /// Segments kept in a --live playlist (default 6)
    #[arg(long)]
    pub live_window: Option<u32>,
    /// Master playlist filename inside a --ladder output dir
    /// (-master_pl_name, default master.m3u8 — multi-channel ABR dirs
    /// each get their own master)
    #[arg(long)]
    pub master: Option<String>,
    /// fMP4 init-segment filename (-hls_fmp4_init_filename, default
    /// init.mp4 — CDN pathing for fMP4 packs; needs --fmp4)
    #[arg(long)]
    pub init: Option<String>,
    /// Append to an existing playlist instead of truncating it
    /// (-hls_flags append_list — crash-resume / accumulate a long
    /// recording across runs)
    #[arg(long)]
    pub append: bool,
    /// Split at every --seg boundary even without a keyframe
    /// (-hls_flags split_by_time — legacy players that need exact-length
    /// segments regardless of GOP alignment)
    #[arg(long)]
    pub split_by_time: bool,
    /// First segment index (seg_NNN + MEDIA-SEQUENCE) — resume a numbered
    /// stream after a restart instead of starting over at 0
    #[arg(long)]
    pub start: Option<u32>,
    /// Derive the first segment index from the epoch clock (24/7 channels
    /// restarting mid-run stay continuous without tracking N)
    #[arg(long)]
    pub epoch: bool,
    /// Write EXT-X-PROGRAM-DATE-TIME on every segment (DVR/live-event
    /// archive — players can seek by wall-clock time)
    #[arg(long)]
    pub date: bool,
    /// Mark the first segment as a discontinuity (EXT-X-DISCONTINUITY —
    /// players must not assume timestamp continuity across a restart;
    /// pairs with --start/--epoch when re-opening a playlist)
    #[arg(long)]
    pub discontinuity: bool,
    /// Name segments by wall-clock time (seg_YYYYmmdd-HHMMSS.ts — archive
    /// recordings whose filenames say when they were captured; -strftime)
    #[arg(long)]
    pub time_names: bool,
    /// Organize --time-names segments into a wall-clock directory per day
    /// (-strftime_mkdir — a 24/7 archive stays browsable: seg_YYYYMMDD/
    /// HHMMSS.ts instead of thousands of flat files)
    #[arg(long)]
    pub time_dirs: bool,
    /// Append the ordinal index to --time-names clock filenames
    /// (second_level_segment_index — seg_YYYYMMDD-HHMMSS_000.ts: wall-clock
    /// names that also sort correctly; needs --time-names)
    #[arg(long)]
    pub seg_index: bool,
    /// Tag EXT-X-INDEPENDENT-SEGMENTS + force a keyframe at every segment
    /// boundary (seek/trick-play VOD; conflicts with --copy and --ladder)
    #[arg(long)]
    pub independent: bool,
    /// Tag EXT-X-I-FRAMES-ONLY on the playlist (-hls_flags iframes_only —
    /// players read it for trick-play scrub previews; pair with
    /// --independent so every segment boundary really is a keyframe)
    #[arg(long)]
    pub iframes: bool,
    /// URL prefix for every playlist segment entry (-hls_base_url — serve
    /// segments from a CDN or different host than the manifest)
    #[arg(long)]
    pub base_url: Option<String>,
    /// Segment filename prefix (letters/digits/-/_) — `v-seg_…`-style
    /// name-schemes so several packs share one directory; applies to every
    /// naming scheme (numbered, --time-names, --single, --ladder)
    #[arg(long)]
    pub name: Option<String>,
    /// Package only program N from a multi-service transport stream
    /// (program number from `probe.programs[]` — broadcast pickup
    /// ingest: one channel of a captured mux, incl. --ladder ABR)
    #[arg(long)]
    pub program: Option<u32>,
    /// Mux a text subtitle file as a WebVTT sidecar rendition (captioned
    /// HLS packs — master gains EXT-X-MEDIA:TYPE=SUBTITLES and every
    /// variant gets SUBTITLES="subtitle"; .srt/.vtt/.ass all decode to
    /// webvtt; conflicts with --single/--program)
    #[arg(long)]
    pub subs: Option<PathBuf>,
}

#[derive(clap::Args, Debug)]
pub struct DashArgs {
    pub input: PathBuf,
    /// Manifest path (out.mpd) or a directory (→ dir/manifest.mpd + segments)
    #[arg(short, long)]
    pub output: PathBuf,
    /// Segment length in seconds (default 4)
    #[arg(long, default_value_t = 4.0)]
    pub seg: f64,
    /// Stream-copy the essence (fast repack; needs h264/aac input)
    #[arg(long)]
    pub copy: bool,
    /// One file per representation with byte-range segments (-single_file;
    /// on-demand VOD: one upload instead of hundreds of chunk files)
    #[arg(long)]
    pub single: bool,
    /// WebM segments (vp9+opus) instead of ISOBMFF — open-codec pipelines
    #[arg(long)]
    pub webm: bool,
    /// Sliding manifest window: keep only the newest N segments listed
    /// (-window_size; live channels written while input is still growing)
    #[arg(long)]
    pub window: Option<u32>,
    /// Audio-only stream package (-vn; podcasts, voice-over DASH)
    #[arg(long)]
    pub audio_only: bool,
    /// Video-only stream package (-an; muted/preview renditions)
    #[arg(long)]
    pub video_only: bool,
    /// ABR ladder: comma list of heights (e.g. 1080,720,480) → N video
    /// Representations at tiered bitrates in one manifest
    /// (adaptive DASH — players switch rungs with bandwidth)
    #[arg(long, value_delimiter = ',')]
    pub ladder: Vec<u32>,
    /// Streaming mode: every frame becomes its own moof fragment
    /// (-streaming — low-latency DASH prep, players append fragments
    /// without waiting for whole segments)
    #[arg(long)]
    pub streaming: bool,
    /// Fragment every SEC inside each segment (-frag_duration +
    /// -frag_type duration — moof granularity between --streaming's
    /// per-frame and plain whole segments; trick-play prep)
    #[arg(long)]
    pub frag: Option<f64>,
    /// Global SIDX index box for the --single byte-range file
    /// (-global_sidx — HTTP range seeking in the packaged asset; mp4
    /// single-file only, conflicts with --streaming)
    #[arg(long)]
    pub sidx: bool,
    /// Segment filename prefix (letters/digits/-/_) — `v-init-…`/`v-seg-…`
    /// name-schemes a pack so several representation packs share one dir
    #[arg(long, value_name = "PREFIX")]
    pub name: Option<String>,
    /// Package only program N from a multi-service transport stream
    /// (program number from `probe.programs[]` — broadcast pickup
    /// ingest: one channel of a captured mux, incl. --ladder ABR)
    #[arg(long)]
    pub program: Option<u32>,
    /// UTCTiming clock URL for live manifests (-utc_timing_url —
    /// players sync wall-clock to compute the live edge; pair --window)
    #[arg(long)]
    pub utc: Option<String>,
    /// DVB-DASH broadcast profile (-mpd_profile dvb_dash — broadcast
    /// ingest that requires the DVB profile instead of plain MPEG-DASH)
    #[arg(long)]
    pub dvb: bool,
    /// Init-segment filename (-init_seg_name — a plain name gets
    /// -$RepresentationID$ appended so streams can't collide; values
    /// containing $ pass through as raw DASH templates)
    #[arg(long)]
    pub init: Option<String>,
    /// Media-segment filename (-media_seg_name — a plain name gets
    /// -$RepresentationID$-$Number%05d$ appended; $ = raw template)
    #[arg(long)]
    pub seg_name: Option<String>,
    /// Drop the <SegmentTimeline> element from the manifest
    /// (-use_timeline 0 — older/basic DASH players that choke on it
    /// get a plain SegmentTemplate index instead)
    #[arg(long)]
    pub no_timeline: bool,
    /// Raw -adaptation_sets spec for multi-rendition packs
    /// (`id=0,streams=0 id=1,streams=1 id=2,streams=2` — maps every
    /// video/audio stream (0:v then 0:a in file order) and groups
    /// them per spec, so several dubbed tracks land as separate
    /// AdaptationSets: multi-language ABR. `streams=v`/`streams=a`
    /// group whole types. Conflicts --ladder/--streaming/--video-only/
    /// --audio-only/--copy/--program/--webm)
    #[arg(long)]
    pub var_map: Option<String>,
}

#[derive(clap::Args, Debug)]
pub struct QaArgs {
    /// Reference (original) clip
    pub a: PathBuf,
    /// Processed clip to measure against A (auto-rescaled to match)
    pub b: PathBuf,
    /// psnr, ssim, msad, vif, or both (default both)
    #[arg(long, default_value = "both")]
    pub metric: String,
}

#[derive(clap::Args, Debug)]
pub struct ConformArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Fit inside WxH (even dims, no crop)
    #[arg(long)]
    pub size: Option<String>,
    /// Constant frame rate
    #[arg(long)]
    pub fps: Option<f64>,
    /// One-pass loudnorm to this I target (e.g. -14)
    #[arg(long, allow_hyphen_values = true)]
    pub lufs: Option<f64>,
    /// x264 quality for the video transcode (0..=51, default 18)
    #[arg(long)]
    pub crf: Option<u32>,
    /// Peak video bitrate the conform encode may burst to (4500k/6M —
    /// spec envelopes that cap rate, e.g. Twitch ≤6000k); pairs --bufsize
    #[arg(long)]
    pub maxrate: Option<String>,
    /// Rate-control buffer (defaults to 2x --maxrate — standard CBR pair)
    #[arg(long)]
    pub bufsize: Option<String>,
    /// Pad color for the letterbox: name or RRGGBB/0xRRGGBB (needs --size)
    #[arg(long)]
    pub pad: Option<String>,
    /// Fill the letterbox with a blurred copy of the video (needs --size)
    #[arg(long)]
    pub blur: bool,
    /// Letterbox anchor: top|bottom|left|right (default centered; needs --pad)
    #[arg(long)]
    pub anchor: Option<String>,
    /// Hold the last frame SEC more seconds (end-card hold; audio is
    /// padded with silence to match)
    #[arg(long)]
    pub hold: Option<f64>,
    /// Hold the first frame SEC seconds before playback (pre-roll)
    #[arg(long)]
    pub hold_start: Option<f64>,
    /// Normalize odd pixel dimensions down to even — phone/screen captures
    /// at odd px can't encode yuv420p x264, this is the one-flag fix
    #[arg(long)]
    pub even: bool,
    /// Resample audio to HZ (conform defaults to broadcast 48000;
    /// pass 44100 for podcast/CD deliverables, 96000 for masters)
    #[arg(long)]
    pub ar: Option<u32>,
    /// Audio channel count (1 = mono for voice/podcast masters,
    /// 2 = stereo — the default)
    #[arg(long)]
    pub channels: Option<u8>,
    /// Conform only program N of a multi-service transport stream
    /// (program number from `probe.programs[]` — broadcast pickup:
    /// pull one channel out of a captured mux into the spec pass)
    #[arg(long)]
    pub program: Option<u32>,
    /// H.264 encode profile for device-compat spec conform
    /// (baseline|main|high — car/kiosk players reject High)
    #[arg(long)]
    pub profile: Option<TranscodeProfile>,
    /// H.264 level cap (e.g. 3.0/4.1 — old decoders reject high levels;
    /// a level alone keeps the profile's features — pair with --profile)
    #[arg(long)]
    pub level: Option<String>,
    /// B-frames (mobile/baseline ingest wants 0 — decode order = display order)
    #[arg(long)]
    pub bf: Option<u32>,
    /// Rotate the picture 90|180|270° during the spec pass (transpose —
    /// portrait phone footage → landscape spec in one encode)
    #[arg(long)]
    pub rotate: Option<u32>,
    /// Pin the video track timescale (-video_track_timescale N — ingest
    /// specs that lock the mp4 clock; mp4/mov targets only)
    #[arg(long)]
    pub timescale: Option<u32>,
    /// Drop the audio track entirely (-an — muted spec packages / silent
    /// B-roll deliverables; conflicts with --ar/--channels/--lufs)
    #[arg(long)]
    pub no_audio: bool,
    /// Pixel aspect ratio N:D or N/D (setsar — anamorphic capture masters:
    /// DV/DVD/CIF sources reflagged widescreen without a re-scale)
    #[arg(long)]
    pub sar: Option<String>,
    /// Display aspect ratio N:D or N/D (setdar — recomputes SAR so the
    /// file *shows* the shape; applied after --sar when both are given)
    #[arg(long)]
    pub dar: Option<String>,
    /// Force a colr atom into mp4-family spec masters (-movflags +write_colr
    /// — platform QC that requires the atom even when color metadata is
    /// unspecified; mp4/mov/m4a targets only)
    #[arg(long)]
    pub colr: bool,
    /// Keyframe interval in frames on the spec-pass encode (-g N —
    /// broadcast ingest specs like "IDR at least every 2s")
    #[arg(long)]
    pub gop: Option<u32>,
}

#[derive(clap::Args, Debug)]
pub struct SyncArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Shift audio by ms: + delays audio, - pulls it earlier
    #[arg(long, allow_hyphen_values = true)]
    pub ms: f64,
}

#[derive(clap::Args, Debug)]
pub struct AlignArgs {
    /// Reference file — the one everything syncs TO (camera master)
    pub reference: PathBuf,
    /// File to align (external recorder take, second camera)
    pub target: PathBuf,
    #[arg(short, long, required_unless_present = "check")]
    pub output: Option<PathBuf>,
    /// Report the detected offset without rendering (sync QC)
    #[arg(long)]
    pub check: bool,
    /// Max shift to search, seconds either way (default 10)
    #[arg(long, default_value_t = 10.0)]
    pub max_lag: f64,
    /// Decode only this many seconds of each file for the correlation
    /// (long multicam takes correlate much faster)
    #[arg(long)]
    pub window: Option<f64>,
}

#[derive(clap::Args, Debug)]
pub struct ScrollArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Credit lines (newlines allowed); or use --file
    #[arg(long, required_unless_present = "file")]
    pub text: Option<String>,
    /// Text file holding the credit lines
    #[arg(long)]
    pub file: Option<PathBuf>,
    /// Roll starts at this time (default 0)
    #[arg(long)]
    pub at: Option<String>,
    /// Roll duration in seconds (default: to the end of the video)
    #[arg(long)]
    pub dur: Option<f64>,
    /// Text size multiplier (default 1 — smaller than title numerals)
    #[arg(long, default_value_t = 1.0)]
    pub size: f64,
    /// Text color (hex or a color name)
    #[arg(long)]
    pub color: Option<String>,
    /// Per-line alignment inside the credit block (left-justified credits)
    #[arg(long, value_enum)]
    pub align: Option<crate::raster::TextAlign>,
    /// Word-wrap credit lines at N columns (≥4)
    #[arg(long)]
    pub wrap: Option<u32>,
    #[arg(long)]
    pub font: Option<String>,
    /// Roll mode: up (end credits) | ticker (bottom news crawl)
    #[arg(long, value_enum, default_value_t = ScrollMode::Up)]
    pub mode: ScrollMode,
    /// Scroll speed in px/s — sets each window's pacing (conflicts --dur)
    #[arg(long)]
    pub speed: Option<f64>,
    /// Opaque bar behind ticker text (name or RRGGBB; ticker only)
    #[arg(long)]
    pub bg: Option<String>,
    /// Draw at this % opacity (0-100 — ghost/watermark overlay)
    #[arg(long)]
    pub opacity: Option<f64>,
}

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum ScrollMode {
    #[default]
    Up,
    /// Bottom news ticker sliding left across the frame
    Ticker,
}

#[derive(clap::Args, Debug)]
pub struct InsertArgs {
    /// Base video that receives the insert
    pub input: PathBuf,
    /// Clip spliced in whole (scaled to the base size)
    #[arg(long)]
    pub clip: PathBuf,
    /// Splice point in the base (h:mm:ss or seconds, `end` to append,
    /// `chapterN` = start of embedded chapter N as `chapter --list` shows it)
    #[arg(long)]
    pub at: String,
    /// xfade into and out of the insert (any xfade name) instead of a hard cut
    #[arg(long)]
    pub transition: Option<String>,
    /// Crossfade seconds at each splice joint (default 0.4)
    #[arg(long)]
    pub duration: Option<f64>,
    /// Splice only the first N seconds of the clip (default: whole clip)
    #[arg(long)]
    pub dur: Option<f64>,
    /// Scale the insert clip's audio 0..=4 (default 1.0; 0 mutes it)
    #[arg(long)]
    pub volume: Option<f64>,
    /// Overwrite the base span under the clip instead of shifting it later
    /// (patch a flub mid-video — output keeps the base duration; single --at)
    #[arg(long)]
    pub replace: bool,
    #[arg(short, long)]
    pub output: PathBuf,
}

#[derive(clap::Args, Debug)]
pub struct MulticamArgs {
    /// Camera A — the angle the edit starts on
    pub cam_a: PathBuf,
    /// Camera B
    pub cam_b: PathBuf,
    /// Switch to the other camera at each of these times (comma list)
    #[arg(long, value_delimiter = ',')]
    pub at: Vec<String>,
    /// Auto-sync camera B to A by audio cross-correlation first — no separate
    /// `align` pass when the two takes started at different wall times
    #[arg(long)]
    pub align: bool,
    /// Keep camera A's audio for the whole edit instead of cutting
    /// audio with the angle (interview standard)
    #[arg(long)]
    pub keep_audio: bool,
    /// xfade duration at each switch in seconds (default 0 = hard cut)
    #[arg(long)]
    pub transition: Option<f64>,
    #[arg(short, long)]
    pub output: PathBuf,
}

#[derive(clap::Args, Debug)]
pub struct ArtArgs {
    /// Audio/video file to attach the cover to
    pub input: PathBuf,
    /// Cover image (jpg/png); omit with --extract
    #[arg(long, required_unless_present = "extract")]
    pub image: Option<PathBuf>,
    /// Pull the embedded cover OUT to -o instead of attaching one
    #[arg(long)]
    pub extract: bool,
    #[arg(short, long)]
    pub output: PathBuf,
}

#[derive(clap::Args, Debug)]
pub struct SilenceArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: Option<PathBuf>,
    /// Insert silence at this position, seconds (default 0 = leading pad) —
    /// comma list pads several points at once
    #[arg(long)]
    pub at: Option<String>,
    /// Append the silence at the end instead (overrides --at)
    #[arg(long)]
    pub end: bool,
    /// Seconds of silence to insert
    #[arg(long)]
    pub dur: Option<f64>,
    /// Report-only: list silence ranges in extras (no -o needed)
    #[arg(long)]
    pub detect: bool,
    /// With --detect: noise floor dB (default -35)
    #[arg(long, allow_hyphen_values = true)]
    pub threshold: Option<f64>,
    /// With --detect: minimum gap seconds (default 0.4)
    #[arg(long)]
    pub min: Option<f64>,
}

#[derive(clap::Args, Debug)]
pub struct LevelerArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Compression threshold dB (default -18)
    #[arg(long, default_value_t = -18.0, allow_hyphen_values = true)]
    pub threshold: f64,
    /// Ratio N:1 (default 4)
    #[arg(long, default_value_t = 4.0)]
    pub ratio: f64,
    /// Attack ms (default 20)
    #[arg(long, default_value_t = 20.0)]
    pub attack: f64,
    /// Release ms (default 250)
    #[arg(long, default_value_t = 250.0)]
    pub release: f64,
    /// Makeup gain dB (default 6)
    #[arg(long, default_value_t = 6.0)]
    pub makeup: f64,
    /// One-click compression curve: voice|podcast|master — fills the knobs
    #[arg(long, value_enum)]
    pub preset: Option<LevelerPreset>,
    /// Engine: compressor (acompressor) or speechnorm — adaptive voice
    /// normalizer that also lifts quiet speech up, not just peaks down
    #[arg(long, value_enum)]
    pub engine: Option<LevelerEngine>,
    /// Level only inside this window — comma list for several (needs --dur)
    #[arg(long)]
    pub at: Option<String>,
    /// Window length in seconds (default: to the end)
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
pub enum LevelerPreset {
    Voice,
    Podcast,
    Master,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug, Default)]
pub enum LevelerEngine {
    /// acompressor — squash peaks above --threshold
    #[default]
    Compressor,
    /// speechnorm — adaptive normalize for speech (speechnorm filter)
    Speechnorm,
    /// mcompand — multiband compression preset (low/body/air bands)
    Mcompand,
    /// alimiter — lookahead brickwall limiter: --threshold is the ceiling dB,
    /// --makeup pushes the input into it (master-safe loudness)
    Limit,
    /// compand — single-band transfer-curve leveler (quiet lifted toward
    /// program level on one continuous knee — gentler than acompressor)
    Compand,
}

#[derive(clap::Args, Debug)]
pub struct GateArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Silence below this dB (default -40)
    #[arg(long, default_value_t = -40.0, allow_hyphen_values = true)]
    pub threshold: f64,
    /// Reduction ratio (default 8)
    #[arg(long, default_value_t = 8.0)]
    pub ratio: f64,
    /// Attack ms (default 10)
    #[arg(long, default_value_t = 10.0)]
    pub attack: f64,
    /// Release ms (default 100)
    #[arg(long, default_value_t = 100.0)]
    pub release: f64,
    /// Tuned settings instead of manual ones
    #[arg(long, value_enum)]
    pub preset: Option<GatePreset>,
    /// Gate only inside this window — comma list for several (needs --dur)
    #[arg(long)]
    pub at: Option<String>,
    /// Window length in seconds (default: to the end)
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
pub enum GatePreset {
    Voice,
    Podcast,
    Studio,
}

#[derive(clap::Args, Debug)]
pub struct WaveformArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// PNG size WxH (default 1920x540)
    #[arg(long, default_value = "1920x540")]
    pub size: String,
    /// Waveform colour RRGGBB hex or ffmpeg name (default ffffff)
    #[arg(long)]
    pub color: Option<String>,
    /// Amplitude scale: lin (default), log, sqrt, cbrt — log shows quiet detail
    #[arg(long)]
    pub scale: Option<String>,
    /// Peak-sample rendering instead of average (transient detail)
    #[arg(long)]
    pub peak: bool,
    /// Draw each channel on its own row (stereo split view)
    #[arg(long)]
    pub split: bool,
    /// Draw every sample pixel (denser wave) vs the default scale draw
    #[arg(long)]
    pub full: bool,
    /// Opaque background color under the wave (thumbnails: kills transparency)
    #[arg(long)]
    pub bg: Option<String>,
    /// Render only this slice (h:mm:ss or seconds)
    #[arg(long)]
    pub at: Option<String>,
    /// Slice length in seconds (default: to the end)
    #[arg(long)]
    pub dur: Option<f64>,
    /// Rotate the wave 90° (time runs top→bottom — vertical PNG h×w)
    #[arg(long)]
    pub vertical: bool,
}

#[derive(clap::Args, Debug)]
pub struct MeterArgs {
    /// Audio or video file to meter
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Meter video size WxH (default 640x480)
    #[arg(long, default_value = "640x480")]
    pub size: String,
    /// EBU meter scale 9..=18 (default 9 = -18..+9 LUFS window)
    #[arg(long, default_value_t = 9)]
    pub meter: u32,
    /// Meter only from this time (`end` = last ~50ms; QC one slice)
    #[arg(long)]
    pub at: Option<String>,
    /// ..for this many seconds (default: to the end)
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(clap::Args, Debug)]
pub struct SpectrogramArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// PNG size WxH (default 1920x1080)
    #[arg(long, default_value = "1920x1080")]
    pub size: String,
    /// Color scheme: magma|viridis|fire|rainbow|green|terrain (ffmpeg names)
    #[arg(long)]
    pub color: Option<String>,
    /// Display scale: lin|sqrt|cbrt|log|4thrt|5thrt (default log)
    #[arg(long)]
    pub scale: Option<String>,
    /// Drop the axis/scale legend strip
    #[arg(long)]
    pub no_legend: bool,
    /// One band per channel instead of a combined picture
    #[arg(long)]
    pub separate: bool,
    /// Render only this slice (h:mm:ss or seconds)
    #[arg(long)]
    pub at: Option<String>,
    /// Slice length in seconds (default: to the end)
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum MainsFreq {
    #[value(name = "50")]
    F50,
    #[value(name = "60")]
    F60,
}

#[derive(clap::Args, Debug)]
pub struct DehumArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Mains frequency: 50 (EU/Asia) or 60 (US) Hz
    #[arg(long, value_enum, default_value_t = MainsFreq::F60)]
    pub mains: MainsFreq,
    /// Custom fundamental hum in Hz (fan/transformer buzz; overrides --mains)
    #[arg(long)]
    pub freq: Option<u32>,
    /// Harmonics to notch beyond the fundamental (1..8)
    #[arg(long, default_value_t = 4)]
    pub harmonics: u32,
    /// Notch only inside this window — comma list for several (needs --dur)
    #[arg(long)]
    pub at: Option<String>,
    /// Window length in seconds (default: to the end)
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(clap::Args, Debug)]
pub struct UpscaleArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Linear upscale multiplier (2 = double width AND height; 1.05-4)
    #[arg(long, default_value_t = 2.0)]
    pub factor: f64,
    /// Unsharp amount 0-1 restores edge acuity after the resize
    #[arg(long, default_value_t = 0.3)]
    pub strength: f64,
    /// Scaler: spline (default, photographic) | xbr / 2xsai (pixel-art /
    /// retro game captures — crisp sprite edges, no ringing)
    #[arg(long, value_enum)]
    pub engine: Option<UpscaleEngine>,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
pub enum UpscaleEngine {
    Spline,
    Xbr,
    TwoXsai,
    /// hqx — hq2x/hq3x/hq4x pixel-art scaler (cleanest sprite/text upscale)
    Hqx,
    /// epx — EPX pixel scaler 2x/3x (classic emulation-style pixel upscale;
    /// softer diagonals than hqx)
    Epx,
}

#[derive(clap::Args, Debug)]
pub struct SmoothArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Smoothing engine: smartblur (default) | bilateral (edge-aware)
    #[arg(long, value_enum)]
    pub engine: Option<SmoothEngine>,
    /// Blur strength 0.1..1 (default 0.5 — skin/sky flattening)
    #[arg(long, default_value_t = 0.5)]
    pub strength: f64,
    /// Smooth only inside this window — comma list for several (needs --dur)
    #[arg(long)]
    pub at: Option<String>,
    /// ..for this many seconds (default: to the end)
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(clap::Args, Debug)]
pub struct WbArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Correction strength 0..1 (0 = off, 1 = full neutralization)
    #[arg(long, default_value_t = 1.0)]
    pub strength: f64,
    /// Per-channel vs linked normalization 0..1 (1 = white balance pick;
    /// 0 = contrast stretch only, keeps the scene's color grade)
    #[arg(long, default_value_t = 1.0)]
    pub independence: f64,
    /// Temporal smoothing in frames — correction eases instead of breathing
    /// on cuts/brightness pops (0 = per-frame)
    #[arg(long, default_value_t = 32)]
    pub smooth: u32,
    /// Engine: normalize (default, per-channel histogram stretch) | greyedge
    /// (grey-edge illuminant estimation — corrects a color cast while keeping
    /// saturation direction; gentler on graded footage)
    #[arg(long, value_enum)]
    pub engine: Option<WbEngine>,
    /// Timestamp(s) to start correcting — comma list allowed; `end` = tail
    #[arg(long)]
    pub at: Option<String>,
    /// Seconds the correction lasts per --at point (default: to end)
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq)]
pub enum WbEngine {
    /// normalize — per-channel histogram stretch (classic WB pick)
    Normalize,
    /// greyedge — grey-edge illuminant estimation (minknorm), softer on
    /// already-graded footage than histogram stretch
    Greyedge,
}

#[derive(clap::Args, Debug)]
pub struct V360Args {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Look direction: degrees left(-)/right(+) of the source center
    #[arg(long, allow_hyphen_values = true, default_value_t = 0.0)]
    pub yaw: f64,
    /// Look direction: degrees down(-)/up(+)
    #[arg(long, allow_hyphen_values = true, default_value_t = 0.0)]
    pub pitch: f64,
    /// Output horizontal field of view in degrees (default 90)
    #[arg(long, default_value_t = 90.0)]
    pub fov: f64,
    /// Input projection (default equirect; use for GoPro/Insta360 dual-fisheye,
    /// single fisheye, cubemap, YouTube EAC, half-equirect)
    #[arg(long = "in", value_enum, default_value_t = V360Projection::Equirect)]
    pub projection: V360Projection,
    /// Output canvas WxH (default: input width x 16:9 height)
    #[arg(long)]
    pub size: Option<String>,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug, Default)]
pub enum V360Projection {
    /// Equirectangular 360 video (default — most exports)
    #[default]
    Equirect,
    /// Single fisheye capture
    Fisheye,
    /// Dual fisheye (GoPro Max / Insta360 / Ricoh Theta raw)
    Dfisheye,
    /// Cubemap 3x2
    C3x2,
    /// Equi-angular cubemap (YouTube EAC)
    Eac,
    /// Facebook barrel format
    Barrel,
    /// Half equirectangular (180-degree VR)
    Hequirect,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug, Default)]
pub enum VDenoiseEngine {
    /// nlmeans — best quality, slowest (per-pixel patch search)
    #[default]
    Nlmeans,
    /// hqdn3d — fast 3D denoiser for previews / long clips
    Hqdn3d,
    /// atadenoise — temporal frame-averaging, best on static shots
    Atadenoise,
    /// vaguedenoiser — wavelet denoiser (strong spatial cut)
    Vaguedenoise,
    /// bm3d — state-of-the-art patch denoiser, slowest but cleanest
    /// (no timeline window: --at is rejected with this engine)
    Bm3d,
    /// dctdnoiz — DCT-domain denoiser (sharp, aggressive sigma)
    Dctdnoiz,
    /// owdenoise — overcomplete wavelet denoiser (very smooth result)
    Owdenoise,
    /// median — salt&pepper / hot-pixel removal (small radius, timeline ok)
    Median,
    /// chroma — chroma-only noise reduction (phone-sensor color speckle)
    Chroma,
    /// edge — nlmeans masked to flat areas only (maskedmerge over an
    /// edgedetect+gblur+negate mask): denoise without melting detail
    Edge,
    /// dotcrawl — dedot: removes dot-crawl + rainbow edges from composite /
    /// analog captures (VHS rips, capture-card footage)
    Dotcrawl,
    /// fftdnoiz — FFT-domain denoise (film grain); prev/next add temporal
    Fftdnoiz,
    /// removegrain — VLC/AviSynth per-plane grain remover (fast spatial
    /// median/blur modes; --strength maps to mode 2..11)
    Rg,
}

#[derive(clap::Args, Debug)]
pub struct ScanArgs {
    pub input: PathBuf,
    /// Minimum frozen stretch to report, seconds (default 1.0)
    #[arg(long)]
    pub freeze_min: Option<f64>,
    /// Minimum black stretch to report, seconds (default 0.3)
    #[arg(long)]
    pub black_min: Option<f64>,
    /// Pixel luma below this counts as black, 0-255 (default 32)
    #[arg(long)]
    pub thresh: Option<f64>,
    /// Frame counts as blurry below this normalized diff-entropy 0..1
    /// (default 0.45 — out-of-focus shots sit well under it)
    #[arg(long)]
    pub blur: Option<f64>,
    /// Also report hard scene-cut timestamps (scdet) — edit-point map for QC
    #[arg(long)]
    pub scenes: bool,
    /// Check whether REF contains this video (MPEG-7 signature match) —
    /// duplicate/re-upload detection for library QC; needs ~2s+ of footage
    #[arg(long, value_name = "REF")]
    pub dupe: Option<PathBuf>,
    /// OCR the frames for burned-in text (tesseract) — reports first hit,
    /// hit frame count, and top confidence. Needs libtesseract ffmpeg
    #[arg(long)]
    pub text: bool,
    /// VMAF motion score (motion_avg/motion_max) — bitrate-budget QC:
    /// static ≈0, busy action ≈7+. Slower leg
    #[arg(long)]
    pub motion: bool,
    /// Read embedded VITC timecode lines (broadcast master QC) — reports
    /// `vitc`/`vitc_tc`/`vitc_frames` when a code is found
    #[arg(long)]
    pub timecode: bool,
    /// Content bounding box (bbox filter, union over frames) — reports
    /// `content_detected`, `content_box` "x,y,w,h" and `content_fill`
    /// (0-1 of frame area). Unlike cropdetect it works on any uniform
    /// background, not just black
    #[arg(long)]
    pub bbox: bool,
    /// Dead-air QC: silent stretches at/below this dB lasting ≥1s become
    /// `deadair_secs`/`deadair_ranges` — podcast/talking-head pause map
    /// before publish (e.g. --deadair -35)
    #[arg(long, allow_negative_numbers = true)]
    pub deadair: Option<f64>,
    /// EBU R128 loudness QC: `loud_i`/`loud_lra`/`loud_tp` (platform spec
    /// checks — podcast −16, broadcast −23, social −14)
    #[arg(long)]
    pub loud: bool,
    /// Keyframe/GOP interval QC (packet flags — no decode): `keyframes`,
    /// `gop_max_sec`, `gop_avg_sec`, `gop_max_frames` — ingest specs like
    /// "keyframes every ≤2s" (YouTube live) verified from the packet map
    #[arg(long)]
    pub gop: bool,
    /// Also write <input>.framemd5 — decoded-frame checksums of every
    /// stream (the archive-ingest integrity manifest; `hash_file`,
    /// `hash_frames` in the report)
    #[arg(long)]
    pub hash: bool,
    /// Video-bitrate curve QC from the packet map (no decode):
    /// `bitrate_mean_mbps`, `bitrate_peak_mbps` (worst 0.5s window),
    /// `bitrate_spike_at` — platform peak-rate spec checks
    #[arg(long)]
    pub bitrate: bool,
    /// Real per-stream packet counts via -count_packets (a second
    /// ffprobe pass): `streams_counted` — nb_read_packets per stream.
    /// Declared `nb_frames` comes from container headers; counted is
    /// actual — a mismatch means a truncated/damaged file
    #[arg(long)]
    pub packets: bool,
    /// Decode-clean QC: a full decode pass at -v warning —
    /// `decode_errors` stderr warning/error count, `decodes_clean`
    /// verdict, `first_error` first issue line (corrupt-ingest gate)
    #[arg(long)]
    pub verify: bool,
}

#[derive(clap::Args, Debug)]
pub struct VdenoiseArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Denoise strength 0.5..=30 (default 4; heavier is slower and softer)
    #[arg(long, default_value_t = 4.0)]
    pub strength: f64,
    /// Engine: nlmeans (quality), hqdn3d (fast), atadenoise (static shots)
    #[arg(long, value_enum)]
    pub engine: Option<VDenoiseEngine>,
    /// Denoise only from this time on (h:mm:ss or seconds) — nlmeans is
    /// slow, so window it when only one scene is grainy — comma list for several (needs --dur)
    #[arg(long)]
    pub at: Option<String>,
    /// ..for this many seconds (default: to the end)
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(clap::Args, Debug)]
pub struct CropArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Explicit box x:y:w:h (overrides --aspect)
    #[arg(long)]
    pub region: Option<String>,
    /// Reframe to aspect W:H (e.g. 1:1, 9:16)
    #[arg(long)]
    pub aspect: Option<String>,
    /// --aspect anchor: center|top|bottom|left|right (keep faces in 9:16)
    #[arg(long, default_value = "center")]
    pub anchor: String,
}

#[derive(clap::Args, Debug)]
pub struct SharpenArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Sharpen only from this time — comma list for several windows (needs --dur)
    #[arg(long)]
    pub at: Option<String>,
    /// ..for this many seconds (default: to the end)
    #[arg(long)]
    pub dur: Option<f64>,
    /// luma unsharp amount (0.3–2)
    #[arg(long, default_value_t = 1.0)]
    pub amount: f64,
    /// Engine: unsharp (default) or cas (contrast-adaptive, no halos)
    #[arg(long, value_enum, default_value_t = SharpenEngine::Unsharp)]
    pub engine: SharpenEngine,
}

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum DisplaceEdge {
    /// smear edge pixels outward
    Smear,
    /// leave uncovered pixels black
    Blank,
    /// wrap around the opposite edge
    Wrap,
    /// mirror the picture back in
    #[default]
    Mirror,
}

#[derive(clap::Args, Debug)]
pub struct DisplaceArgs {
    /// Video to warp
    pub input: PathBuf,
    /// Clip whose luma drives the displacement map
    #[arg(long)]
    pub map: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Edge handling for pixels pushed off-frame (default mirror)
    #[arg(long, value_enum, default_value_t = DisplaceEdge::Mirror)]
    pub edge: DisplaceEdge,
    /// Displace only from this time — comma list ok (`end` ok, needs --dur)
    #[arg(long)]
    pub at: Option<String>,
    /// ..for this many seconds (default: to the end)
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(clap::Args, Debug)]
pub struct EqvizArgs {
    /// Audio or video file to EQ + visualize
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// EQ bands, one per audio channel pair: "f=200 w=100 g=10 t=h" is a +10dB
    /// low shelf at 200Hz (t=h high-shelf, t=l low-shelf, t=p peak). Two
    /// entries separated by " | " sets stereo — one entry applies to all.
    #[arg(long)]
    pub bands: Option<String>,
    /// Curve video size WxH (default 640x360)
    #[arg(long, default_value = "640x360")]
    pub size: String,
    /// Only from this time (`end` ok)
    #[arg(long)]
    pub at: Option<String>,
    /// ..for this many seconds (default: to the end)
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(clap::Args, Debug)]
pub struct VignetteArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// vignette angle in radians (smaller = stronger; default PI/4)
    #[arg(long, default_value_t = 0.785)]
    pub angle: f64,
    /// Vignette only from this time — comma list for several windows (needs --dur)
    #[arg(long)]
    pub at: Option<String>,
    /// ..for this many seconds (default: to the end)
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(clap::Args, Debug)]
pub struct BwArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Desaturate only from this time — comma list for several windows (needs --dur)
    #[arg(long)]
    pub at: Option<String>,
    /// ..for this many seconds (default: to the end)
    #[arg(long)]
    pub dur: Option<f64>,
    /// Saturation left 0..=1 (default 0 = full bw; 0.4 keeps muted color)
    #[arg(long)]
    pub strength: Option<f64>,
    /// Film channel weights "r,g,b" (e.g. 1.5,0.3,0.1 darkens blue skies like
    /// a red filter) — replaces the BT.601 luma mix
    #[arg(long)]
    pub weights: Option<String>,
    /// Hard threshold 0-1 instead of grayscale (xerox / high-contrast look)
    #[arg(long)]
    pub cut: Option<f64>,
}

#[derive(clap::Args, Debug)]
pub struct VolumeArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Gain in dB (negative = quieter)
    #[arg(long, allow_hyphen_values = true)]
    pub db: f64,
    /// Apply the gain only from this time on — comma list for several spots (needs --dur)
    #[arg(long)]
    pub at: Option<String>,
    /// Length of the gain window (default: to the end)
    #[arg(long)]
    pub dur: Option<f64>,
    /// Brickwall limiter after the gain, ceiling in dBTP (e.g. -1)
    #[arg(long, allow_hyphen_values = true)]
    pub limit: Option<f64>,
}

#[derive(clap::Args, Debug)]
pub struct ProgressArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Bar color (ffmpeg name or 0xRRGGBB)
    #[arg(long, default_value = "white")]
    pub color: String,
    /// Track color behind the fill (default: no track)
    #[arg(long)]
    pub bg: Option<String>,
    /// Bar thickness in pixels
    #[arg(long, default_value_t = 8)]
    pub height: u32,
    /// Which edge the bar rides on
    #[arg(long, value_enum, default_value_t = BarEdge::Bottom)]
    pub edge: BarEdge,
    /// Show the bar only from this time on — comma list for several runs (needs --dur)
    #[arg(long)]
    pub at: Option<String>,
    /// ..for this many seconds (default: to the end)
    #[arg(long)]
    pub dur: Option<f64>,
    /// Count down instead: bar starts full and depletes to zero ("time left" overlays)
    #[arg(long)]
    pub reverse: bool,
    /// Draw the bar at this % opacity (ghost progress)
    #[arg(long)]
    pub opacity: Option<f64>,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug, Default, PartialEq)]
pub enum BarEdge {
    #[default]
    Bottom,
    Top,
    Left,
    Right,
}

#[derive(clap::Args, Debug)]
pub struct BoomerangArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Repeat the forward-backward cycle N times total (default 1)
    #[arg(long, default_value_t = 1)]
    pub times: u32,
    /// Boomerang only this window — the rest of the clip plays straight;
    /// comma list boomerangs several spots (needs --dur)
    #[arg(long)]
    pub at: Option<String>,
    /// Window length in seconds (default: to the end)
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(clap::Args, Debug)]
pub struct FxArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Effect to apply
    #[arg(long, value_enum, required = true)]
    pub kind: FxKind,
    /// Effect depth 0..1 (default 0.5)
    #[arg(long, default_value_t = 0.5)]
    pub strength: f64,
    /// Effect only from this time — comma list for several windows (needs --dur)
    #[arg(long)]
    pub at: Option<String>,
    /// ..for this many seconds (default: to the end)
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
pub enum FxKind {
    Tremolo,
    Vibrato,
    Flanger,
    Phaser,
    Chorus,
    Echo,
    Lofi,
    Radio,
    /// Tape-style soft clip saturation (asoftclip) — warmth + loudness
    Saturate,
    /// Harmonic exciter — adds airy presence above ~7.5kHz (aexciter)
    Excite,
    /// One-knob bass boost around 110Hz (bass) — thin audio gets weight
    Bass,
    /// Next-room / underwater muffle (lowpass sweep)
    Muffled,
    /// Transient sharpening for dull recordings (crystalizer)
    Crystal,
    /// Sub-bass synthesis for drops/trap (asubboost, derived low octave)
    Sub,
    /// Headphone crossfeed — speakers-like imaging on cans (long-form comfort)
    Crossfeed,
    /// Left-right autopan sweep (apulsator)
    Autopan,
    /// Robot/Dalek voice — TRUE ring modulation (amultiply with a sine
    /// carrier; strength sweeps the carrier 25→500Hz)
    Ringmod,
    /// crush — acrusher bitcrusher (bit depth + sample-rate destruction, lo-fi digital)
    Crush,
    /// fshift — afreqshift frequency shifter: metallic alien/robot voice,
    /// strength sweeps the shift 50→2000Hz (not pitch-shift — harmonic
    /// relationships warp deliberately)
    Fshift,
    /// contrast — acontrast dynamics tilt: strength >0.5 expands punch,
    /// <0.5 compresses toward level
    Contrast,
    /// Auto-wah — asendcmd sweeps a resonant equalizer peak like a wah
    /// pedal (strength speeds the sweep 0.7→3Hz)
    Wah,
}

#[derive(clap::Args, Debug)]
pub struct ChapterArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Chapter as TIME|TITLE, repeatable (time: h:mm:ss or seconds)
    #[arg(long = "at", required_unless_present_any = ["auto", "import", "export", "yt", "cue", "lrc", "podcast", "vtt", "csv", "srt", "edl", "fcpxml", "list", "remove", "spread", "scenes"])]
    pub at: Vec<String>,
    /// Auto-place chapters after each silence >= N seconds (podcast segments)
    #[arg(long)]
    pub auto: Option<f64>,
    /// Auto-place chapters at scene cuts (scdet — lecture/talking-head
    /// chapters where silence gaps don't exist)
    #[arg(long)]
    pub scenes: bool,
    /// Generate N evenly-spaced marks instead of --at (uniform TOC for a
    /// long episode/lecture — pair with --titles or any export flag)
    #[arg(long)]
    pub spread: Option<u32>,
    /// Comma-separated titles for --spread N (must give exactly N names;
    /// default "Chapter 1..N")
    #[arg(long)]
    pub titles: Option<String>,
    /// Write the chapter marks as an ffmetadata text file at -o instead of
    /// embedding them (hand the marks to an editor/DAW)
    #[arg(long)]
    pub export: bool,
    /// Write marks in YouTube description format ("0:00 Intro") at -o —
    /// paste under the video for platform seek chapters
    #[arg(long)]
    pub yt: bool,
    /// Write marks as a .cue sheet at -o (audiobook/podcast players —
    /// TRACK/INDEX entries at mm:ss:ff precision)
    #[arg(long)]
    pub cue: bool,
    /// Write marks as Podcasting 2.0 chapters JSON at -o
    /// ({"chapters":[{"startTime":sec,"title":…}]} — podverse/
    /// podcastindex feeds)
    #[arg(long)]
    pub podcast: bool,
    /// Write marks as an LRC file at -o ([mm:ss.xx]title per line) —
    /// synced-lyrics format music players read for seekable track/verse marks
    #[arg(long)]
    pub lrc: bool,
    /// Write marks as a WebVTT chapter file at -o — drop it next to the
    /// video on a web player (<track kind="chapters">) for click-to-seek
    /// navigation on self-hosted/Vimeo-style embeds
    #[arg(long)]
    pub vtt: bool,
    /// Write marks as a CSV at -o (H:MM:SS.mmm,Title per line) —
    /// Resolve/Premiere marker import and spreadsheet round-trips
    /// (--import reads .csv back: header row + quoted titles ok)
    #[arg(long)]
    pub csv: bool,
    /// Write marks as an .srt file at -o — each mark becomes a cue titled
    /// with the chapter name, spanning to the next mark (TOC as soft
    /// subs; burn to preview where seek points land)
    #[arg(long)]
    pub srt: bool,
    /// Write marks as an EDL (edit decision list) at -o — CMX-style
    /// events Resolve/Premiere/DaVinci import as timeline markers at
    /// the clip's frame rate
    #[arg(long)]
    pub edl: bool,
    /// Export marks as Final Cut Pro XML — FCP/Resolve import each mark
    /// as a timeline marker (the editor-native TOC exchange format)
    #[arg(long)]
    pub fcpxml: bool,
    /// Import marks from a text file: lines "TIME|TITLE" or "TIME,TITLE"
    /// ('#' comments and blank lines skipped)
    #[arg(long)]
    pub import: Option<PathBuf>,
    /// List the input's embedded chapter marks as JSON (no output written)
    #[arg(long)]
    pub list: bool,
    /// Strip every chapter on remux (platforms that mangle them)
    #[arg(long)]
    pub remove: bool,
    /// Shift every mark by SEC (negative pulls earlier — re-time marks
    /// after adding/removing an intro)
    #[arg(long)]
    pub shift: Option<f64>,
    /// Multiply every mark time by this factor — marks authored for a cut
    /// that was then retimed (mirrors subs --rate)
    #[arg(long)]
    pub rate: Option<f64>,
    /// Drop marks closer than SEC to the previous kept mark — tidy a dense
    /// TOC (--scenes over-firing on rapid cuts; first mark always kept;
    /// extras: min_gap_dropped)
    #[arg(long)]
    pub min_gap: Option<f64>,
    /// Snap each mark to the nearest keyframe — chapter points that land on
    /// seekable frames (HLS/DASH players seek to keyframes anyway; without
    /// snapping a seek lands late). extras: snapped
    #[arg(long)]
    pub snap: bool,
    /// Frames/sec for reading the ff field of .edl imports (default 30 —
    /// NTSC EDLs; PAL exchange is 25)
    #[arg(long)]
    pub fps: Option<f64>,
    /// Silence threshold for --auto in dB (default -35 — quiet podcasts /
    /// ASMR rooms need -45 so their softer pauses still mark chapters)
    #[arg(long, allow_hyphen_values = true)]
    pub thresh: Option<f64>,
}

#[derive(clap::Args, Debug)]
pub struct FreezeArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Hold the frame at this time (h:mm:ss or seconds)
    #[arg(long)]
    pub at: Option<String>,
    /// Hold length in seconds (default 1.0 with --at)
    #[arg(long)]
    pub dur: Option<f64>,
    /// Freeze the LAST frame for this many seconds (outro freeze)
    #[arg(long)]
    pub end: Option<f64>,
    /// Seconds before the freeze played at half-speed (swoop-into-hold)
    #[arg(long)]
    pub ease: Option<f64>,
    /// Seconds before the freeze replayed backwards (rewind-into-hold)
    #[arg(long)]
    pub reverse: Option<f64>,
    /// Slow push-in on the held frame to this end scale (1.02–2, e.g. 1.15);
    /// mid-clip --at freezes only
    #[arg(long)]
    pub zoom: Option<f64>,
}

#[derive(clap::Args, Debug)]
pub struct CensorArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Region to censor, x:y:w:h in pixels — comma list covers several spots at once
    #[arg(long)]
    pub region: String,
    /// Mosaic blocks or gaussian blur
    #[arg(long, value_enum, default_value_t = CensorMode::Pixel)]
    pub mode: CensorMode,
    /// Censor only inside this window — comma list for several spots (needs --dur; `end` ok)
    #[arg(long)]
    pub at: Option<String>,
    /// Window length in seconds (default: to the end)
    #[arg(long)]
    pub dur: Option<f64>,
    /// Effect intensity: mosaic block size in px (pixel) / blur sigma (blur)
    #[arg(long, default_value_t = 12.0)]
    pub strength: f64,
    /// Mask shape: box (default) or circle (ellipse inside each region)
    #[arg(long, value_enum, default_value_t = CensorShape::Box)]
    pub shape: CensorShape,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum CensorShape {
    Box,
    Circle,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug, Default, PartialEq)]
pub enum CensorMode {
    #[default]
    Pixel,
    Blur,
    /// Solid black bar — the "CLASSIFIED" redact look
    Solid,
}

#[derive(clap::Args, Debug)]
pub struct GridArgs {
    pub inputs: Vec<PathBuf>,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Tile layout cols x rows
    #[arg(long, default_value = "2x2")]
    pub layout: String,
    /// Canvas WxH
    #[arg(long, default_value = "1920x1080")]
    pub size: String,
    /// Take audio from this input index instead of mixing all tracks
    #[arg(long)]
    pub audio: Option<usize>,
    /// Per-tile labels (comma-separated, one per input)
    #[arg(long)]
    pub labels: Option<String>,
    /// Pixel gap around each tile (default 0 = flush)
    #[arg(long)]
    pub gap: Option<u32>,
    /// Crop tiles to fill the cell instead of letterboxing
    #[arg(long)]
    pub fill: bool,
    /// Hero layout: first input fills a big left column (~2/3), the rest
    /// stack down the right (podcast/interview, ignores --layout grid)
    #[arg(long)]
    pub focus: bool,
    /// Gutter / letterbox color behind the tiles (name or RRGGBB; default black)
    #[arg(long)]
    pub bg: Option<String>,
    /// Stamp an mm:ss readout on every tile (same clock across the grid)
    #[arg(long)]
    pub time: bool,
}

#[derive(clap::Args, Debug)]
pub struct BlurArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Gaussian sigma (0.5–20)
    #[arg(long, default_value_t = 2.0)]
    pub sigma: f64,
    /// Engine: gblur (default) | directional — linear streaks along --angle
    /// (speed-line / motion-smear look)
    #[arg(long, value_enum)]
    pub engine: Option<BlurEngine>,
    /// Streak direction degrees 0..360 for --engine directional (default 45)
    #[arg(long)]
    pub angle: Option<f64>,
    /// Blur only from this time — comma list for several windows (needs --dur)
    #[arg(long)]
    pub at: Option<String>,
    /// ..for this many seconds (default: to the end)
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum BlurEngine {
    Gblur,
    /// dblur — directed streaks (speed lines, fake motion)
    Directional,
    /// boxblur — classic box kernel: faster than gblur, blockier texture
    /// (draft blur, stylized defocus)
    Box,
    /// avgblur — area-average box blur (lightest kernel; pixel-art-safe
    /// when strength stays small, huge radii for washes)
    Avg,
}

#[derive(clap::Args, Debug)]
pub struct BatchArgs {
    pub dir: PathBuf,
    #[arg(short, long)]
    pub output_dir: PathBuf,
    /// 0 = auto (available parallelism)
    #[arg(long, default_value_t = 0)]
    pub jobs: usize,
    /// Recurse into subdirectories
    #[arg(long)]
    pub recursive: bool,
    /// Verb and its flags; input and -o are injected
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub rest: Vec<String>,
}

#[derive(clap::Args, Debug)]
pub struct SonifyArgs {
    /// Image or video to play as sound (spectrogram scan)
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Duration seconds for still images (ignored for video input)
    #[arg(long, default_value_t = 5.0)]
    pub dur: f64,
    /// Scan speed multiplier (1 = sweep the width once per --dur)
    #[arg(long, default_value_t = 1.0)]
    pub speed: f64,
    /// Sample rate Hz
    #[arg(long, default_value_t = 44100)]
    pub sample_rate: u32,
}

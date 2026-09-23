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
    Remux(RemuxArgs),
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
    /// Smooth gradient banding (skies, backdrops) windowed
    Deband(DebandArgs),
    /// Drop near-duplicate frames (screen recordings, slide decks)
    Dedup(DedupArgs),
    /// Auto-contrast for flat/washed footage (histeq)
    Equalize(EqualizeArgs),
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
}

#[derive(clap::Args, Debug)]
pub struct ConcatArgs {
    /// Input clips, in order
    pub inputs: Vec<PathBuf>,
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
    /// Denoise only inside this window — comma list for several (needs --dur)
    #[arg(long)]
    pub at: Option<String>,
    /// Window length in seconds (default: to the end)
    #[arg(long)]
    pub dur: Option<f64>,
}

#[derive(clap::Args, Debug)]
pub struct CompressArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Target size, e.g. 10MB (Discord), 16MB (WhatsApp), 25MB (email); KB/MB/GB
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
}

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum SharpenEngine {
    #[default]
    Unsharp,
    /// Contrast-adaptive sharpening — crisper edges, no white halos
    Cas,
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
    /// AV1 (svt-av1 on ffmpeg ≥7, libaom on 4.x) — smallest web delivery
    Av1,
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
    /// Burn this .srt/.vtt onto the delivery canvas (captioned Reels in one pass)
    #[arg(long)]
    pub subs: Option<PathBuf>,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum DeliverPlatform {
    Social,
    Reels,
    Tiktok,
    Shorts,
    /// 1:1 feed grid (1080x1080, -14 LUFS)
    Square,
    /// 16:9 landscape upload (1920x1080, -14 LUFS)
    Youtube,
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
}

#[derive(clap::Args, Debug)]
pub struct GlitchArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Glitch intensity 0.5-20 (channel shift px + noise)
    #[arg(long, default_value_t = 3.0)]
    pub strength: f64,
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
    /// Add a 1kHz tone bed
    #[arg(long, default_value_t = true)]
    pub tone: bool,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
pub enum ScopeMode {
    /// Color vectorscope
    Vector,
    /// Luma/RGB waveform monitor
    Wave,
}

#[derive(clap::Args, Debug)]
pub struct ScopeArgs {
    pub input: PathBuf,
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
    #[arg(long, required_unless_present = "ramp")]
    pub factor: Option<f64>,
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
    /// Extract EVERY subtitle stream to stem_0.srt, stem_1.srt … (batch)
    #[arg(long)]
    pub all: bool,
    /// Rescale every cue time by this factor — 25→23.976 fps drift ≈ 0.959
    #[arg(long)]
    pub rate: Option<f64>,
    /// Convert between subtitle formats (.srt ↔ .vtt) — input is the cue file
    #[arg(long)]
    pub convert: bool,
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
    /// Fix the display rotation flag (0/90/180/270) without re-encoding
    #[arg(long)]
    pub rotate: Option<u32>,
    /// Strip ALL container metadata (privacy clean before publishing)
    #[arg(long)]
    pub clear: bool,
    /// Copy metadata + chapters from this file onto the output
    #[arg(long)]
    pub copy: Option<PathBuf>,
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
    #[arg(long)]
    pub audio: PathBuf,
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
}

#[derive(clap::Args, Debug)]
pub struct KeyArgs {
    /// Foreground footage with the color to remove
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Background image or video (sized to the foreground canvas)
    #[arg(long)]
    pub bg: PathBuf,
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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum SlideMotion {
    None,
    Kenburns,
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
    #[arg(long)]
    pub text: String,
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
    /// Wet tail amount (0..0.9; 0.3 ≈ subtle room)
    #[arg(long, default_value_t = 0.3)]
    pub wet: f64,
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
    /// With --mode pan: stereo position -1 (full left) .. 1 (full right)
    #[arg(long, allow_hyphen_values = true)]
    pub pan: Option<f64>,
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
    /// Deinterlacer engine (default yadif; bwdif smoother motion)
    #[arg(long, value_enum)]
    pub engine: Option<DeintEngine>,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum DeintEngine {
    Yadif,
    Bwdif,
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
    /// Count DOWN to the window end instead of up from --at
    #[arg(long)]
    pub down: bool,
    /// Draw at this % opacity (0-100 — ghost/watermark overlay)
    #[arg(long)]
    pub opacity: Option<f64>,
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
}

#[derive(clap::Args, Debug)]
pub struct QaArgs {
    /// Reference (original) clip
    pub a: PathBuf,
    /// Processed clip to measure against A (auto-rescaled to match)
    pub b: PathBuf,
    /// psnr, ssim, or both (default both)
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
    /// Pad color for the letterbox: name or RRGGBB/0xRRGGBB (needs --size)
    #[arg(long)]
    pub pad: Option<String>,
    /// Fill the letterbox with a blurred copy of the video (needs --size)
    #[arg(long)]
    pub blur: bool,
    /// Letterbox anchor: top|bottom|left|right (default centered; needs --pad)
    #[arg(long)]
    pub anchor: Option<String>,
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
    #[arg(short, long)]
    pub output: PathBuf,
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
    /// Splice point in the base (h:mm:ss or seconds, or `end` to append)
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
pub struct VdenoiseArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Denoise strength 0.5..=30 (default 4; heavier is slower and softer)
    #[arg(long, default_value_t = 4.0)]
    pub strength: f64,
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
}

#[derive(clap::Args, Debug)]
pub struct ChapterArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Chapter as TIME|TITLE, repeatable (time: h:mm:ss or seconds)
    #[arg(long = "at", required_unless_present_any = ["auto", "import", "export", "yt", "list", "remove"])]
    pub at: Vec<String>,
    /// Auto-place chapters after each silence >= N seconds (podcast segments)
    #[arg(long)]
    pub auto: Option<f64>,
    /// Write the chapter marks as an ffmetadata text file at -o instead of
    /// embedding them (hand the marks to an editor/DAW)
    #[arg(long)]
    pub export: bool,
    /// Write marks in YouTube description format ("0:00 Intro") at -o —
    /// paste under the video for platform seek chapters
    #[arg(long)]
    pub yt: bool,
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
    /// Blur only from this time — comma list for several windows (needs --dur)
    #[arg(long)]
    pub at: Option<String>,
    /// ..for this many seconds (default: to the end)
    #[arg(long)]
    pub dur: Option<f64>,
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

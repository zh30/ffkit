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
    /// Scale, crop, pad, rotate to a frame
    Fit(FitArgs),
    /// Extract audio, a frame, or subtitles
    Extract(ExtractArgs),
    /// Overlay an image or video (logo, PiP)
    Overlay(OverlayArgs),
    /// Burn or mux subtitles
    Caption(CaptionArgs),
    /// EBU R128 loudness normalisation (two-pass)
    Loudnorm(LoudnormArgs),
    /// Transcode to a delivery preset (h264, webm, gif)
    Transcode(TranscodeArgs),
    /// One-shot 9:16 social export (Reels / TikTok / Shorts)
    Deliver(DeliverArgs),
    /// Change playback speed (talking-head 1.1–2×, slow-mo)
    Speed(SpeedArgs),
    /// Mix a music bed under speech, with ducking
    Music(MusicArgs),
    /// Cut internal silence (talking-head jump cuts)
    Jumpcut(JumpcutArgs),
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
}

#[derive(clap::Args, Debug)]
pub struct ConcatArgs {
    /// Input clips, in order
    pub inputs: Vec<PathBuf>,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Crossfade between two clips (2 inputs only)
    #[arg(long)]
    pub transition: Option<String>,
    #[arg(long, default_value_t = 0.5)]
    pub duration: f64,
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
    #[arg(long)]
    pub rotate: Option<u32>,
    #[arg(long, value_enum)]
    pub flip: Option<FlipMode>,
    #[arg(long)]
    pub fps: Option<f64>,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum FitMode {
    Pad,
    Crop,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum FlipMode {
    H,
    V,
}

#[derive(clap::Args, Debug)]
pub struct ExtractArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Timestamp for a still frame
    #[arg(long)]
    pub at: Option<String>,
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
    #[arg(long)]
    pub font: Option<String>,
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

#[derive(clap::Args, Debug)]
pub struct LoudnormArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Integrated loudness target (LUFS)
    #[arg(short = 'I', long, default_value_t = -14.0, allow_hyphen_values = true)]
    pub i: f64,
    /// True peak (dBTP)
    #[arg(long, default_value_t = -1.5, allow_hyphen_values = true)]
    pub tp: f64,
    #[arg(long, default_value_t = 11.0, allow_hyphen_values = true)]
    pub lra: f64,
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
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum TranscodePreset {
    H264,
    Webm,
    Gif,
}

#[derive(clap::Args, Debug)]
pub struct DeliverArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Destination canvas; all map to 1080x1920 / -14 LUFS
    #[arg(long, value_enum, default_value_t = DeliverPlatform::Social)]
    pub platform: DeliverPlatform,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum DeliverPlatform {
    Social,
    Reels,
    Tiktok,
    Shorts,
}

#[derive(clap::Args, Debug)]
pub struct SpeedArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Playback factor: 2 = twice as fast, 0.5 = slow-mo
    #[arg(long)]
    pub factor: f64,
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
pub struct CoverArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Timestamp to grab; default 0
    #[arg(long)]
    pub at: Option<String>,
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
}

#[derive(clap::Args, Debug)]
pub struct TitleArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Hook text (newlines allowed)
    #[arg(long)]
    pub text: String,
    /// Seconds the title stays on screen
    #[arg(long, default_value_t = 1.0)]
    pub duration: f64,
    #[arg(long)]
    pub font: Option<String>,
    /// center (default) or top
    #[arg(long, default_value = "center")]
    pub position: String,
}

#[derive(clap::Args, Debug)]
pub struct LoopArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Play this many times (2–12)
    #[arg(long, default_value_t = 2)]
    pub times: u32,
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
}

#[derive(clap::Args, Debug)]
pub struct ZoomArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// 1.25 = 25% punch-in on the center
    #[arg(long, default_value_t = 1.25)]
    pub factor: f64,
}

#[derive(clap::Args, Debug)]
pub struct SharpenArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// luma unsharp amount (0.3–2)
    #[arg(long, default_value_t = 1.0)]
    pub amount: f64,
}

#[derive(clap::Args, Debug)]
pub struct VignetteArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// vignette angle in radians (smaller = stronger; default PI/4)
    #[arg(long, default_value_t = 0.785)]
    pub angle: f64,
}

#[derive(clap::Args, Debug)]
pub struct BwArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
}

#[derive(clap::Args, Debug)]
pub struct VolumeArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Gain in dB (negative = quieter)
    #[arg(long, allow_hyphen_values = true)]
    pub db: f64,
}

#[derive(clap::Args, Debug)]
pub struct BlurArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Gaussian sigma (0.5–20)
    #[arg(long, default_value_t = 2.0)]
    pub sigma: f64,
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

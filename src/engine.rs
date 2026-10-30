use std::path::Path;
use std::time::Duration;

use crate::cli::Globals;
use crate::contract::Contract;
use crate::error::Error;
use crate::paths;
use crate::probe::{self, Probe};
use crate::spawn::{self, Argv};

pub fn commands_of(argvs: &[Argv]) -> Vec<Vec<String>> {
    argvs.iter().map(Argv::display).collect()
}

pub fn run_argvs(argvs: &[Argv], g: &Globals) -> Result<(), Error> {
    for argv in argvs {
        let spawned = spawn::run(argv, g.timeout, g.progress)?;
        spawn::require_ok(argv, spawned)?;
    }
    Ok(())
}

pub fn write_job(
    tool: &str,
    inputs: &[&Path],
    output: &Path,
    argvs: Vec<Argv>,
    g: &Globals,
) -> Result<Contract, Error> {
    for input in inputs {
        paths::ensure_input(input)?;
    }
    paths::ensure_output_allowed(output, inputs, g.overwrite)?;
    let mut argvs = argvs;
    retarget_audio_codec(&mut argvs, output);
    let commands = commands_of(&argvs);

    if g.dry_run {
        let probe = probe::probe(inputs[0], Duration::from_secs(60)).ok();
        return Ok(
            Contract::dry_run(tool, Some(paths::display(output)), probe).with_commands(commands)
        );
    }

    if let Err(e) = run_argvs(&argvs, g) {
        return Ok(Contract::failed(tool, &e).with_commands(commands));
    }

    if !output.exists() {
        return Err(Error::verification(format!(
            "output was not written: {}",
            output.display()
        )));
    }
    let meta = std::fs::metadata(output)?;
    if meta.len() == 0 {
        return Err(Error::verification(format!(
            "output is empty: {}",
            output.display()
        )));
    }

    let probed = probe::probe(output, Duration::from_secs(60))?;
    Ok(Contract::ok(tool, Some(paths::display(output)), Some(probed)).with_commands(commands))
}

/// `write_job` for outputs ffprobe can't read (raw attachment dumps —
/// `-dump_attachment` writes the payload bytes verbatim, no container).
/// Same guarantees minus the probe pass.
pub fn write_job_raw(
    tool: &str,
    inputs: &[&Path],
    output: &Path,
    argvs: Vec<Argv>,
    g: &Globals,
) -> Result<Contract, Error> {
    for input in inputs {
        paths::ensure_input(input)?;
    }
    paths::ensure_output_allowed(output, inputs, g.overwrite)?;
    let mut argvs = argvs;
    retarget_audio_codec(&mut argvs, output);
    let commands = commands_of(&argvs);

    if g.dry_run {
        let probe = probe::probe(inputs[0], Duration::from_secs(60)).ok();
        return Ok(
            Contract::dry_run(tool, Some(paths::display(output)), probe).with_commands(commands)
        );
    }

    if let Err(e) = run_argvs(&argvs, g) {
        return Ok(Contract::failed(tool, &e).with_commands(commands));
    }

    let ok = std::fs::metadata(output)
        .map(|m| m.len() > 0)
        .unwrap_or(false);
    if !ok {
        return Err(Error::verification(format!(
            "output missing or empty: {}",
            output.display()
        )));
    }
    let mut c = Contract::ok(tool, Some(paths::display(output)), None).with_commands(commands);
    c.verified = Some(true);
    Ok(c)
}

/// Verbs hard-code `-c:a aac`, which writes an AAC payload into whatever
/// container `-o` names — AAC-in-.wav fails to decode on some ffmpeg 4.x
/// builds. Rewrite aac to a codec the extension actually carries.
fn retarget_audio_codec(argvs: &mut [Argv], output: &Path) {
    let codec = match output
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .as_deref()
    {
        Some("wav") => "pcm_s16le",
        Some("flac") => "flac",
        // Ogg Opus when the build has no libvorbis
        Some("ogg") | Some("oga") if !has_encoder("libvorbis") => "libopus",
        Some("ogg") | Some("oga") => "libvorbis",
        Some("opus") => "libopus",
        Some("mp3") => "libmp3lame",
        _ => return,
    };
    for argv in argvs.iter_mut() {
        for i in 0..argv.args.len().saturating_sub(1) {
            if argv.args[i] == "-c:a" && argv.args[i + 1] == "aac" {
                argv.args[i + 1] = spawn::os(codec);
            }
        }
    }
}

pub fn ffmpeg_base(progress: bool) -> Argv {
    let mut argv = Argv::ffmpeg();
    argv.push("-y");
    if progress {
        argv.push("-stats");
    } else {
        argv.extend(["-loglevel", "error", "-nostats"]);
    }
    argv
}

/// Major version of the ffmpeg on PATH (from `ffmpeg -version`), if parseable.
/// Asked once per process.
pub fn ffmpeg_major() -> Option<u32> {
    static MAJOR: std::sync::OnceLock<Option<u32>> = std::sync::OnceLock::new();
    *MAJOR.get_or_init(read_ffmpeg_major)
}

fn read_ffmpeg_major() -> Option<u32> {
    let mut argv = Argv::ffmpeg();
    argv.push("-version");
    let spawned = spawn::run(&argv, Duration::from_secs(10), false).ok()?;
    if !spawned.status_ok {
        return None;
    }
    let line = String::from_utf8_lossy(&spawned.stdout);
    let line = line.lines().next()?;
    let ver = line.split_whitespace().nth(2)?;
    ver.split(|c: char| !c.is_ascii_digit())
        .next()
        .and_then(|s| s.parse().ok())
}

/// Whether this ffmpeg build has encoder `name` (`ffmpeg -encoders`).
/// Release notes drop encoders (v410/v308/ayuv in 7.0) and minimal builds
/// leave out library ones (libwebp, libvorbis), so callers ask, not assume.
pub fn has_encoder(name: &str) -> bool {
    static LIST: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    listed(&LIST, "-encoders", name)
}

/// Whether this ffmpeg build has filter `name` (`ffmpeg -filters`);
/// library filters (zscale, subtitles, vidstab*) are build options.
pub fn has_filter(name: &str) -> bool {
    static LIST: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    listed(&LIST, "-filters", name)
}

fn listed(cache: &std::sync::OnceLock<String>, flag: &str, name: &str) -> bool {
    let list = cache.get_or_init(|| {
        let mut argv = Argv::ffmpeg();
        argv.push(flag);
        spawn::run(&argv, Duration::from_secs(10), false)
            .map(|s| String::from_utf8_lossy(&s.stdout).into_owned())
            .unwrap_or_default()
    });
    list.lines()
        .any(|l| l.split_whitespace().nth(1) == Some(name))
}

/// `ffmpeg -h filter=NAME` text, for options and modes that come and go
/// between releases. Empty when ffmpeg cannot be asked.
pub fn filter_help(name: &str) -> String {
    let mut argv = Argv::ffmpeg();
    argv.extend(["-h".to_string(), format!("filter={name}")]);
    spawn::run(&argv, Duration::from_secs(10), false)
        .map(|s| String::from_utf8_lossy(&s.stdout).into_owned())
        .unwrap_or_default()
}

/// Vorbis audio args: libvorbis when the build has it, else ffmpeg's own
/// experimental encoder, which only takes stereo.
pub fn vorbis_args() -> Vec<&'static str> {
    if has_encoder("libvorbis") {
        vec!["-c:a", "libvorbis"]
    } else {
        vec!["-c:a", "vorbis", "-strict", "-2", "-ac", "2"]
    }
}

/// Whether muxer `muxer` still accepts private option `opt`
/// (`ffmpeg -h muxer=NAME`); options get removed between majors.
pub fn muxer_has_option(muxer: &str, opt: &str) -> bool {
    let mut argv = Argv::ffmpeg();
    argv.extend(["-h".to_string(), format!("muxer={muxer}")]);
    spawn::run(&argv, Duration::from_secs(10), false)
        .map(|s| {
            String::from_utf8_lossy(&s.stdout)
                .lines()
                .any(|l| l.split_whitespace().next() == Some(&format!("-{opt}")))
        })
        .unwrap_or(false)
}

/// Frame-rate sync mode in the spelling this ffmpeg accepts: `-fps_mode`
/// (5+, the only one left from 7) or the old `-vsync` (4.x). Takes the
/// `-vsync` numbers or the `-fps_mode` names.
pub fn push_fps_mode(argv: &mut Argv, mode: &str) {
    let name = match mode {
        "0" => "passthrough",
        "1" => "cfr",
        "2" => "vfr",
        "-1" => "auto",
        m => m,
    };
    if ffmpeg_major().unwrap_or(7) >= 5 {
        argv.extend(["-fps_mode", name]);
    } else {
        argv.extend(["-vsync", name]);
    }
}

pub fn need_video(probe: &Probe, tool: &str) -> Result<(), Error> {
    if probe.has_video {
        Ok(())
    } else {
        Err(Error::input(format!("{tool}: input has no video stream")))
    }
}

pub fn probe_or_err(path: &Path, g: &Globals) -> Result<Probe, Error> {
    probe::probe(path, g.timeout.min(Duration::from_secs(120)))
}

/// Probe with extra demuxer options (e.g. `-skip_initial_bytes N` ahead of
/// a junk-prefixed input so ffprobe still reaches the moov atom).
pub fn probe_or_err_opts(path: &Path, g: &Globals, opts: &[String]) -> Result<Probe, Error> {
    probe::probe_with_opts(path, g.timeout.min(Duration::from_secs(120)), opts)
}

/// Audio window for filters that lack timeline `enable` (ffmpeg 4.4 applies
/// it to most audio FX): dry feed is ducked to 0 inside [at, end), the FX
/// chain runs on the whole input and its window is trimmed/delayed into place.
/// `dur=None` = to input end. Emits a filter_complex body ending in `[aout]`.
pub fn audio_window(fx: &str, at: f64, dur: Option<f64>) -> String {
    let (gate, slice) = match dur {
        Some(d) => (
            format!("1-between(t,{at:.3},{:.3})", at + d),
            format!("atrim=start={at:.3}:duration={d:.3}"),
        ),
        None => (format!("lt(t,{at:.3})"), format!("atrim=start={at:.3}")),
    };
    format!(
        "[0:a]asplit=2[d][w];[d]volume='{gate}':eval=frame[dout];[w]{fx},{slice},asetpts=PTS-STARTPTS,adelay={:.0}:all=1[wx];[dout][wx]amix=inputs=2:duration=first:normalize=0[aout]",
        at * 1000.0
    )
}

/// `--at` may be a comma list: resolve it to windows and emit one wet branch
/// per window — the dry gate ANDs `1-between(t,..)` terms so the original
/// audio dips under every FX window. Comma lists need `--dur`.
pub fn audio_window_for(
    fx: &str,
    at: &str,
    dur: Option<f64>,
    duration: f64,
) -> Result<String, crate::error::Error> {
    let windows = crate::time::enable_windows(at, dur, duration)?;
    if windows.len() == 1 {
        return Ok(audio_window(fx, windows[0].0, dur));
    }
    let n = windows.len();
    let gate = windows
        .iter()
        .map(|(s, e)| format!("1-between(t,{s:.3},{e:.3})"))
        .collect::<Vec<_>>()
        .join("*");
    let mut fc = format!("[0:a]asplit={}[d]", n + 1);
    for i in 0..n {
        fc.push_str(&format!("[w{i}]"));
    }
    fc.push_str(&format!(";[d]volume='{gate}':eval=frame[dout];"));
    for (i, (s, e)) in windows.iter().enumerate() {
        fc.push_str(&format!(
            "[w{i}]{fx},atrim=start={s:.3}:duration={:.3},asetpts=PTS-STARTPTS,adelay={:.0}:all=1[wx{i}];",
            e - s,
            s * 1000.0
        ));
    }
    fc.push_str("[dout]");
    for i in 0..n {
        fc.push_str(&format!("[wx{i}]"));
    }
    fc.push_str(&format!(
        "amix=inputs={}:duration=first:normalize=0[aout]",
        n + 1
    ));
    Ok(fc)
}

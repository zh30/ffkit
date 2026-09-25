use serde_json::json;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::cli::{FramesArgs, Globals};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::paths;

/// Dump stills every `--every` seconds to `-o` with a `%03d` suffix
/// (same `stem_%02d.ext` convention as `split`).
pub fn run(args: FramesArgs, g: &Globals) -> Result<Contract, Error> {
    if args.every <= 0.0 {
        return Err(Error::input("--every must be > 0"));
    }
    let probe = engine::probe_or_err(&args.input, g)?;
    engine::need_video(&probe, "frames")?;
    paths::ensure_input(&args.input)?;

    let out_s = args.output.to_string_lossy().into_owned();
    let template: PathBuf = if out_s.contains('%') {
        PathBuf::from(out_s)
    } else {
        let ext = args
            .output
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("png");
        let stem = args
            .output
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "frame".into());
        args.output
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(format!("{stem}_%03d.{ext}"))
    };

    if args.untile.is_some() && (!args.at.is_empty() || args.count.is_some()) {
        return Err(Error::input(
            "--untile splits every frame (no --at/--count)",
        ));
    }
    let mut vf = match &args.untile {
        Some(u) => {
            let (c, r) = u
                .split_once('x')
                .ok_or_else(|| Error::input("--untile wants COLSxROWS (e.g. 4x3)"))?;
            let cols: u32 = c
                .parse()
                .map_err(|_| Error::input("--untile wants COLSxROWS (e.g. 4x3)"))?;
            let rows: u32 = r
                .parse()
                .map_err(|_| Error::input("--untile wants COLSxROWS (e.g. 4x3)"))?;
            if !(1..=64).contains(&cols) || !(1..=64).contains(&rows) {
                return Err(Error::input("--untile tiles must be 1..=64"));
            }
            // each source frame bursts into cols*rows tile frames — the way
            // to break a contact sheet or mosaic clip back into stills
            format!("untile=layout={cols}x{rows}")
        }
        None => format!("fps=1/{}", args.every),
    };
    if !args.at.is_empty() {
        // --at: N seek+grab jobs, each its own output file
        let probe = engine::probe_or_err(&args.input, g)?;
        let ext = args
            .output
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("png");
        let stem = args
            .output
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "shot".into());
        let mut cmds = Vec::new();
        let mut outs = Vec::new();
        for (i, s) in args.at.iter().enumerate() {
            let t = crate::time::resolve_frame_at(s, probe.duration)?;
            if !(0.0..probe.duration).contains(&t) {
                return Err(Error::input(format!(
                    "frames --at {s} is outside the {:.2}s source",
                    probe.duration
                )));
            }
            let out = args
                .output
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or_else(|| Path::new("."))
                .join(format!("{stem}_{:03}.{ext}", i + 1));
            let mut av = ffmpeg_base(g.progress);
            av.extend(["-ss", &format!("{t:.3}")]);
            av.push("-i");
            av.push(&args.input);
            if let Some(w) = args.width {
                av.extend(["-vf", &format!("scale={w}:-1")]);
            }
            av.extend(["-frames:v", "1"]);
            av.push(&out);
            cmds.push(av);
            outs.push(out);
        }
        let commands = engine::commands_of(&cmds);
        if g.dry_run {
            return Ok(Contract::dry_run(
                "frames",
                Some(paths::display(&args.output)),
                Some(probe),
            )
            .with_commands(commands));
        }
        if let Err(e) = engine::run_argvs(&cmds, g) {
            return Ok(Contract::failed("frames", &e).with_commands(commands));
        }
        let first = crate::probe::probe(&outs[0], Duration::from_secs(60))?;
        return Ok(
            Contract::ok("frames", Some(paths::display(&args.output)), Some(first))
                .with_commands(commands)
                .with_extra(serde_json::json!({
                    "stills": outs.iter().map(|p| paths::display(p)).collect::<Vec<_>>()
                })),
        );
    }
    // --nth: every Nth frame — dataset/QC sampling by frame count, not seconds
    let mut argv_vsync: Option<&str> = None;
    if let Some(n) = args.nth {
        if n < 2 {
            return Err(Error::input("--nth wants N >= 2 (every Nth frame)"));
        }
        if args.count.is_some() || args.untile.is_some() || !args.at.is_empty() {
            return Err(Error::input(
                "--nth overrides --every — drop --count/--untile/--at",
            ));
        }
        // -vsync 0 keeps the dropped-frame gaps — default cfr duplicates
        // the selected frames back into nearly every slot
        vf = format!("select='not(mod(n\\,{n}))'");
        argv_vsync = Some("0");
    }
    // --number: grab exactly frame N (0-based decoded order) — pinpoint a
    // known-bad frame by index where --at's time math would drift on VFR
    if let Some(raw) = &args.number {
        if args.count.is_some()
            || args.untile.is_some()
            || !args.at.is_empty()
            || args.nth.is_some()
        {
            return Err(Error::input(
                "--number overrides --every — drop --count/--untile/--at/--nth",
            ));
        }
        let mut terms = Vec::new();
        for part in raw.split(',') {
            let n: u32 = part
                .trim()
                .parse()
                .map_err(|_| Error::input("frames --number wants a 0-based frame index"))?;
            terms.push(format!("eq(n\\,{n})"));
        }
        vf = format!("select='{}'", terms.join("+"));
        argv_vsync = Some("0");
    }
    // --count spreads N stills across ~95% of the clip (thumb --count spacing);
    // a higher rate lands the last pts past EOF and drops a frame.
    let mut frame_cap: Option<u32> = None;
    if let Some(n) = args.count {
        if !(1..=500).contains(&n) {
            return Err(Error::input("--count must be 1..=500"));
        }
        let fps = (n as f64 - 0.5).max(0.5) / probe.duration.max(0.05);
        vf = format!("fps={fps:.6}");
        frame_cap = Some(n);
    }
    if let Some(w) = args.width {
        vf.push_str(&format!(",scale={w}:-2"));
    }
    let mut argv = ffmpeg_base(g.progress);
    argv.push("-i");
    argv.push(&args.input);
    if let Some(v) = argv_vsync {
        argv.extend(["-vsync", v]);
    }
    argv.extend(["-vf", &vf]);
    if let Some(n) = frame_cap {
        argv.extend(["-frames:v", &n.to_string()]);
    }
    argv.push(&template);

    let commands = engine::commands_of(std::slice::from_ref(&argv));
    if g.dry_run {
        return Ok(
            Contract::dry_run("frames", Some(paths::display(&template)), Some(probe))
                .with_commands(commands),
        );
    }
    if let Err(e) = engine::run_argvs(&[argv], g) {
        return Ok(Contract::failed("frames", &e).with_commands(commands));
    }

    let dir = template
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let name = template
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let (pre, post) = match name.split_once('%') {
        Some((a, b)) => match b.find('.') {
            Some(d) => (a.to_string(), b[d..].to_string()),
            None => (a.to_string(), String::new()),
        },
        None => (name.clone(), String::new()),
    };
    let mut parts = Vec::new();
    for e in std::fs::read_dir(dir)? {
        let e = e?;
        let fname = e.file_name().to_string_lossy().into_owned();
        if fname.starts_with(&pre) && fname.ends_with(&post) {
            parts.push(e.path());
        }
    }
    parts.sort();
    if parts.is_empty() {
        return Err(Error::verification(format!(
            "frames wrote no files matching {}",
            template.display()
        )));
    }
    let first = crate::probe::probe(&parts[0], Duration::from_secs(60))?;
    let names: Vec<String> = parts.iter().map(|p| paths::display(p)).collect();
    let c = Contract::ok("frames", Some(paths::display(&template)), Some(first))
        .with_commands(commands)
        .with_extra(json!({
            "every": args.every,
            "untile": args.untile,
            "count": parts.len(),
            "files": names,
        }));
    Ok(c)
}

use serde_json::json;

use crate::cli::{Globals, GridArgs};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::spawn::Argv;

fn parse_wxh(s: &str, flag: &str) -> Result<(u32, u32), Error> {
    let (w, h) = s
        .split_once('x')
        .and_then(|(a, b)| Some((a.trim().parse::<u32>().ok()?, b.trim().parse::<u32>().ok()?)))
        .filter(|(w, h)| *w > 0 && *h > 0)
        .ok_or_else(|| Error::input(format!("{flag} must look like 1920x1080 / 2x2")))?;
    Ok((w, h))
}

pub fn run(args: GridArgs, g: &Globals) -> Result<Contract, Error> {
    let n = args.inputs.len();
    if n < 2 {
        return Err(Error::input("grid needs at least 2 inputs"));
    }
    if n > 16 {
        return Err(Error::input("grid accepts at most 16 inputs"));
    }
    let (cols, rows) = parse_wxh(&args.layout, "--layout")?;
    let cells = (cols * rows) as usize;
    if n > cells {
        return Err(Error::input(format!(
            "{n} inputs don't fit a {}x{} grid ({cells} cells)",
            cols, rows
        )));
    }
    let (cw, ch) = parse_wxh(&args.size, "--size")?;
    let (tw, th) = (cw / cols, ch / rows);
    if tw < 16 || th < 16 {
        return Err(Error::input(
            "--size too small for that --layout (tiles < 16px)",
        ));
    }

    let mut probes = Vec::new();
    for f in &args.inputs {
        let p = engine::probe_or_err(f, g)?;
        engine::need_video(&p, "grid")?;
        probes.push(p);
    }
    let all_audio = probes.iter().all(|p| p.has_audio);

    let mut seg: Vec<String> = Vec::new();
    let mut layout_str = String::new();
    for i in 0..n {
        let col = i as u32 % cols;
        let row = i as u32 / cols;
        seg.push(format!(
            "[{i}:v]scale={tw}:{th}:force_original_aspect_ratio=decrease,pad={tw}:{th}:(ow-iw)/2:(oh-ih)/2,setsar=1,fps=30,format=yuv420p[v{i}]"
        ));
        if !layout_str.is_empty() {
            layout_str.push('|');
        }
        layout_str.push_str(&format!("{}_{}", col * tw, row * th));
        if all_audio {
            seg.push(format!(
                "[{i}:a]aresample=48000,aformat=channel_layouts=stereo[a{i}]"
            ));
        }
    }
    let ins: String = (0..n).map(|i| format!("[v{i}]")).collect();
    seg.push(format!("{ins}xstack=inputs={n}:layout={layout_str}[vout]"));
    if all_audio {
        let ains: String = (0..n).map(|i| format!("[a{i}]")).collect();
        seg.push(format!("{ains}amix=inputs={n}:normalize=0[aout]"));
    }
    let fc = seg.join(";");

    let mut argv = ffmpeg_base(g.progress);
    for f in &args.inputs {
        argv.push("-i");
        argv.push(f);
    }
    argv.extend(["-filter_complex", &fc, "-map", "[vout]"]);
    if all_audio {
        argv.extend(["-map", "[aout]", "-c:a", "aac"]);
    }
    argv.extend([
        "-c:v",
        "libx264",
        "-preset",
        "fast",
        "-crf",
        "18",
        "-pix_fmt",
        "yuv420p",
        "-shortest",
    ]);
    argv.push(&args.output);

    let refs: Vec<&std::path::Path> = args.inputs.iter().map(|p| p.as_path()).collect();
    let argvs: Vec<Argv> = vec![argv];
    let c = engine::write_job("grid", &refs, &args.output, argvs, g)?;
    let extra = json!({
        "inputs": n,
        "layout": args.layout,
        "size": args.size,
        "tile": format!("{tw}x{th}"),
        "mixed_audio": all_audio,
    });
    Ok(c.with_extra(extra))
}

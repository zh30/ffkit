use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

fn ffkit() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ffkit"))
}

fn has_ffmpeg() -> bool {
    Command::new("ffmpeg")
        .args(["-hide_banner", "-version"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn fixture(dir: &Path) -> PathBuf {
    let out = dir.join("f.mp4");
    let status = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc=duration=1:size=320x240:rate=30",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=1",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            "-shortest",
        ])
        .arg(&out)
        .status()
        .expect("spawn ffmpeg");
    assert!(status.success(), "ffmpeg fixture failed");
    out
}

fn run_json(args: &[&str]) -> Value {
    let out = ffkit()
        .arg("--json")
        .args(args)
        .output()
        .expect("run ffkit");
    let stdout = String::from_utf8_lossy(&out.stdout);
    serde_json::from_str(&stdout).unwrap_or_else(|_| {
        panic!(
            "expected json, status={} stdout={} stderr={}",
            out.status,
            stdout,
            String::from_utf8_lossy(&out.stderr)
        )
    })
}

#[test]
fn doctor_ok() {
    if !has_ffmpeg() {
        return;
    }
    let v = run_json(&["doctor"]);
    assert_eq!(v["status"], "ok");
    assert_eq!(v["tool"], "doctor");
    assert_eq!(v["extra"]["ffmpeg"]["usable"], true);
}

#[test]
fn probe_fixture() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let f = fixture(dir.path());
    let v = run_json(&["probe", f.to_str().unwrap()]);
    assert_eq!(v["status"], "ok");
    let p = &v["probe"];
    assert!(p["duration"].as_f64().unwrap() > 0.8);
    assert_eq!(p["width"], 320);
    assert_eq!(p["height"], 240);
    assert_eq!(p["has_video"], true);
    assert_eq!(p["has_audio"], true);
}

#[test]
fn cut_copy_and_extract() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let f = fixture(dir.path());
    let cut = dir.path().join("cut.mp4");
    let v = run_json(&[
        "cut",
        f.to_str().unwrap(),
        "--start",
        "0",
        "--end",
        "0.5",
        "-o",
        cut.to_str().unwrap(),
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    assert!(v["probe"]["duration"].as_f64().unwrap() <= 0.7);

    let wav = dir.path().join("a.wav");
    let v = run_json(&["extract", f.to_str().unwrap(), "-o", wav.to_str().unwrap()]);
    assert_eq!(v["status"], "ok", "{v}");
    assert_eq!(v["probe"]["has_audio"], true);
    assert_eq!(v["probe"]["has_video"], false);
}

#[test]
fn concat_two() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let f = fixture(dir.path());
    let out = dir.path().join("join.mp4");
    let v = run_json(&[
        "concat",
        f.to_str().unwrap(),
        f.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    assert!(v["probe"]["duration"].as_f64().unwrap() > 1.5);
}

#[test]
fn fit_look_transcode() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let f = fixture(dir.path());
    let vertical = dir.path().join("v.mp4");
    let v = run_json(&[
        "fit",
        f.to_str().unwrap(),
        "--aspect",
        "9:16",
        "--fit",
        "pad",
        "-o",
        vertical.to_str().unwrap(),
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    assert_eq!(v["probe"]["width"], 1080);
    assert_eq!(v["probe"]["height"], 1920);

    let sheet = dir.path().join("sheet.png");
    let v = run_json(&[
        "look",
        vertical.to_str().unwrap(),
        "--tiles",
        "2x2",
        "-o",
        sheet.to_str().unwrap(),
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    assert!(sheet.exists());

    let strip = dir.path().join("strip.png");
    let v = run_json(&[
        "look",
        f.to_str().unwrap(),
        "--at",
        "0",
        "--at",
        "0.5",
        "-o",
        strip.to_str().unwrap(),
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    assert!(strip.exists());

    let gif = dir.path().join("p.gif");
    let v = run_json(&[
        "transcode",
        f.to_str().unwrap(),
        "--preset",
        "gif",
        "-o",
        gif.to_str().unwrap(),
    ]);
    assert_eq!(v["status"], "ok", "{v}");
}

#[test]
fn overlay_and_refuse_source() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let f = fixture(dir.path());
    let logo = dir.path().join("logo.png");
    let status = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "color=c=red:s=64x64:d=1",
            "-frames:v",
            "1",
        ])
        .arg(&logo)
        .status()
        .unwrap();
    assert!(status.success());

    let out = dir.path().join("ov.mp4");
    let v = run_json(&[
        "overlay",
        f.to_str().unwrap(),
        "--image",
        logo.to_str().unwrap(),
        "--position",
        "top-right",
        "-o",
        out.to_str().unwrap(),
    ]);
    assert_eq!(v["status"], "ok", "{v}");

    let v = run_json(&[
        "cut",
        f.to_str().unwrap(),
        "--duration",
        "0.2",
        "-o",
        f.to_str().unwrap(),
    ]);
    assert_eq!(v["status"], "failed");
    assert_eq!(v["error"]["kind"], "input");
}

fn color_clip(dir: &Path, name: &str, color: &str) -> PathBuf {
    let out = dir.join(name);
    let status = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            &format!("color=c={color}:s=320x240:d=1:rate=30"),
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=1",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            "-shortest",
        ])
        .arg(&out)
        .status()
        .expect("color clip");
    assert!(status.success());
    out
}

fn mean_rgb(png: &Path) -> (f64, f64, f64) {
    let img = image::open(png).expect("png").to_rgb8();
    let n = img.pixels().len() as f64;
    let mut r = 0.0;
    let mut g = 0.0;
    let mut b = 0.0;
    for p in img.pixels() {
        r += f64::from(p[0]);
        g += f64::from(p[1]);
        b += f64::from(p[2]);
    }
    (r / n, g / n, b / n)
}

#[test]
fn broll_cutaway_keeps_audio_and_duration() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let aroll = color_clip(dir.path(), "a.mp4", "0x0033aa");
    let insert = color_clip(dir.path(), "b.mp4", "0xcc0000");
    let out = dir.path().join("cutaway.mp4");
    let v = run_json(&[
        "broll",
        aroll.to_str().unwrap(),
        "--insert",
        insert.to_str().unwrap(),
        "--at",
        "0.35",
        "--duration",
        "0.30",
        "-o",
        out.to_str().unwrap(),
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    assert_eq!(v["tool"], "broll");
    assert_eq!(v["probe"]["has_audio"], true, "{v}");
    let d = v["probe"]["duration"].as_f64().unwrap();
    assert!(
        (d - 1.0).abs() < 0.15,
        "A-roll duration must stay ~1s, got {d}; {v}"
    );

    let frame = |at: &str, name: &str| {
        let p = dir.path().join(name);
        let look = run_json(&[
            "look",
            out.to_str().unwrap(),
            "--at",
            at,
            "-o",
            p.to_str().unwrap(),
        ]);
        assert_eq!(look["status"], "ok", "{look}");
        mean_rgb(&p)
    };
    let before = frame("0.10", "t0.png");
    let mid = frame("0.50", "t1.png");
    let after = frame("0.90", "t2.png");
    assert!(
        before.2 > before.0 + 20.0,
        "before cutaway should be A-roll blue, got {before:?}"
    );
    assert!(
        mid.0 > mid.2 + 20.0,
        "during cutaway should be B-roll red, got {mid:?}"
    );
    assert!(
        after.2 > after.0 + 20.0,
        "after cutaway should return to A-roll blue, got {after:?}"
    );
}

fn color_clip_dur(dir: &Path, name: &str, color: &str, seconds: f64) -> PathBuf {
    let out = dir.join(name);
    let status = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            &format!("color=c={color}:s=320x240:d={seconds}:rate=30"),
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
        ])
        .arg(&out)
        .status()
        .expect("color clip");
    assert!(status.success());
    out
}

/// B-roll shorter than --at, first half red / second half green.
/// Without setpts the window would freeze on green (B already EOF).
#[test]
fn broll_plays_insert_from_start() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let aroll = color_clip_dur(dir.path(), "a.mp4", "0x0033aa", 2.0);
    let red = color_clip_dur(dir.path(), "red.mp4", "0xcc0000", 0.25);
    let green = color_clip_dur(dir.path(), "green.mp4", "0x00cc00", 0.25);
    let insert = dir.path().join("b.mp4");
    let concat = run_json(&[
        "concat",
        red.to_str().unwrap(),
        green.to_str().unwrap(),
        "-o",
        insert.to_str().unwrap(),
    ]);
    assert_eq!(concat["status"], "ok", "{concat}");
    let out = dir.path().join("cutaway.mp4");
    let v = run_json(&[
        "broll",
        aroll.to_str().unwrap(),
        "--insert",
        insert.to_str().unwrap(),
        "--at",
        "0.80",
        "--duration",
        "0.50",
        "-o",
        out.to_str().unwrap(),
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let d = v["probe"]["duration"].as_f64().unwrap();
    assert!((d - 2.0).abs() < 0.2, "A-roll duration ~2s, got {d}; {v}");

    let frame = |at: &str, name: &str| {
        let p = dir.path().join(name);
        let look = run_json(&[
            "look",
            out.to_str().unwrap(),
            "--at",
            at,
            "-o",
            p.to_str().unwrap(),
        ]);
        assert_eq!(look["status"], "ok", "{look}");
        mean_rgb(&p)
    };
    let early = frame("0.90", "early.png");
    let late = frame("1.15", "late.png");
    let after = frame("1.50", "after.png");
    assert!(
        early.0 > early.1 + 20.0,
        "start of cutaway must be B's first half (red), not frozen last frame; got {early:?}"
    );
    assert!(
        late.1 > late.0 + 20.0,
        "later in the window must be B's second half (green); got {late:?}"
    );
    assert!(
        after.2 > after.0 + 20.0,
        "after the window A-roll blue returns, got {after:?}"
    );
}

#[test]
fn raw_ffmpeg_and_graph() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let f = fixture(dir.path());
    let out = dir.path().join("raw.mp4");
    let v = run_json(&[
        "ffmpeg",
        "--because",
        "test escape hatch",
        "--",
        "-i",
        f.to_str().unwrap(),
        "-t",
        "0.3",
        "-c",
        "copy",
        out.to_str().unwrap(),
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    assert_eq!(v["extra"]["escape"], true);
    assert_eq!(v["extra"]["because"], "test escape hatch");

    let plan_path = dir.path().join("plan.json");
    let graph_out = dir.path().join("g.mp4");
    let plan = serde_json::json!({
        "inputs": [f.to_str().unwrap()],
        "filter_complex": [{
            "filter": "scale",
            "in": ["0:v"],
            "opts": { "w": 160, "h": -2 },
            "out": ["vout"]
        }],
        "map": ["vout", "0:a"],
        "output": graph_out.to_str().unwrap(),
        "args": { "c:v": "libx264", "c:a": "copy" }
    });
    std::fs::write(&plan_path, serde_json::to_string(&plan).unwrap()).unwrap();
    let v = run_json(&["graph", plan_path.to_str().unwrap()]);
    assert_eq!(v["status"], "ok", "{v}");
    assert_eq!(v["probe"]["width"], 160);
}

#[test]
fn caption_and_loudnorm() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let f = fixture(dir.path());
    let srt = dir.path().join("c.srt");
    std::fs::write(&srt, "1\n00:00:00,000 --> 00:00:01,000\nHELLO\n").unwrap();
    let burned = dir.path().join("cap.mp4");
    let v = run_json(&[
        "caption",
        f.to_str().unwrap(),
        "--srt",
        srt.to_str().unwrap(),
        "--mode",
        "mux",
        "-o",
        burned.to_str().unwrap(),
    ]);
    assert_eq!(v["status"], "ok", "{v}");

    let loud = dir.path().join("ln.mp4");
    let v = run_json(&[
        "loudnorm",
        f.to_str().unwrap(),
        "-I",
        "-16",
        "-o",
        loud.to_str().unwrap(),
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    assert!((v["extra"]["target_i"].as_f64().unwrap() + 16.0).abs() < 0.01);
}

#[test]
fn caption_burn_overlay_without_libass() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("blue.mp4");
    let status = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "color=c=0x0033aa:s=320x240:d=1:rate=30",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=1",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            "-shortest",
        ])
        .arg(&f)
        .status()
        .unwrap();
    assert!(status.success());
    let srt = dir.path().join("c.srt");
    std::fs::write(&srt, "1\n00:00:00,000 --> 00:00:01,000\nHELLO\n").unwrap();
    let out = dir.path().join("burn.mp4");
    let mut args = vec![
        "caption".to_string(),
        f.to_string_lossy().into_owned(),
        "--srt".into(),
        srt.to_string_lossy().into_owned(),
        "--mode".into(),
        "burn".into(),
        "-o".into(),
        out.to_string_lossy().into_owned(),
    ];
    let arial = "/System/Library/Fonts/Supplemental/Arial.ttf";
    if std::path::Path::new(arial).is_file() {
        args.push("--font".into());
        args.push(arial.into());
    }
    let argv: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let v = run_json(&argv);
    assert_eq!(v["status"], "ok", "{v}");
    assert_eq!(v["extra"]["renderer"], "overlay", "{v}");
    assert_eq!(v["probe"]["has_video"], true);
    assert!(v["probe"]["duration"].as_f64().unwrap() > 0.8);

    let frame = dir.path().join("cap.png");
    let look = run_json(&[
        "look",
        out.to_str().unwrap(),
        "--at",
        "0.4",
        "-o",
        frame.to_str().unwrap(),
    ]);
    assert_eq!(look["status"], "ok", "{look}");
    let img = image::open(&frame).expect("frame png").to_rgb8();
    let white = img
        .pixels()
        .filter(|p| p[0] > 230 && p[1] > 230 && p[2] > 230)
        .count();
    assert!(
        white > 20,
        "burned caption should put near-white glyphs on the blue frame, got {white} white pixels"
    );
}

#[test]
fn caption_burn_social_clears_bottom_fifth() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("blue.mp4");
    let status = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "color=c=0x0033aa:s=360x640:d=1:rate=30",
            "-pix_fmt",
            "yuv420p",
            "-c:v",
            "libx264",
        ])
        .arg(&f)
        .status()
        .unwrap();
    assert!(status.success());
    let srt = dir.path().join("c.srt");
    std::fs::write(&srt, "1\n00:00:00,000 --> 00:00:01,000\nHELLO\n").unwrap();
    let out = dir.path().join("burn.mp4");
    let mut args = vec![
        "caption".to_string(),
        f.to_string_lossy().into_owned(),
        "--srt".into(),
        srt.to_string_lossy().into_owned(),
        "--mode".into(),
        "burn".into(),
        "-o".into(),
        out.to_string_lossy().into_owned(),
    ];
    let arial = "/System/Library/Fonts/Supplemental/Arial.ttf";
    if std::path::Path::new(arial).is_file() {
        args.push("--font".into());
        args.push(arial.into());
    }
    let argv: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let v = run_json(&argv);
    assert_eq!(v["status"], "ok", "{v}");
    assert_eq!(v["extra"]["safe"], "social", "{v}");
    assert!((v["extra"]["bottom_frac"].as_f64().unwrap() - 0.20).abs() < 1e-6);

    let frame = dir.path().join("cap.png");
    let look = run_json(&[
        "look",
        out.to_str().unwrap(),
        "--at",
        "0.4",
        "-o",
        frame.to_str().unwrap(),
    ]);
    assert_eq!(look["status"], "ok", "{look}");
    let img = image::open(&frame).expect("frame png").to_rgb8();
    let (w, h) = img.dimensions();
    let y0 = (h as f64 * 0.80).ceil() as u32;
    let mut white_all = 0u32;
    let mut white_bottom = 0u32;
    for y in 0..h {
        for x in 0..w {
            let p = img.get_pixel(x, y);
            if p[0] > 230 && p[1] > 230 && p[2] > 230 {
                white_all += 1;
                if y >= y0 {
                    white_bottom += 1;
                }
            }
        }
    }
    assert!(
        white_all > 20,
        "caption glyphs should be visible, got {white_all} white pixels"
    );
    assert_eq!(
        white_bottom, 0,
        "social safe-zone must keep burn-in above the bottom 20% ({white_bottom} white pixels in y>={y0} of {w}x{h})"
    );
}

#[test]
fn deliver_social_1080x1920() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let f = fixture(dir.path());
    let out = dir.path().join("reel.mp4");
    let v = run_json(&[
        "deliver",
        f.to_str().unwrap(),
        "--platform",
        "reels",
        "-o",
        out.to_str().unwrap(),
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    assert_eq!(v["probe"]["width"], 1080, "{v}");
    assert_eq!(v["probe"]["height"], 1920, "{v}");
    assert_eq!(v["probe"]["has_audio"], true, "{v}");
    assert_eq!(v["probe"]["has_video"], true, "{v}");
    assert!(v["probe"]["duration"].as_f64().unwrap() > 0.8, "{v}");
    assert_eq!(v["extra"]["frame"], "1080x1920");
    assert!((v["extra"]["target_i"].as_f64().unwrap() + 14.0).abs() < 0.01);
    assert!(out.metadata().unwrap().len() > 0);
}

#[test]
fn speed_2x_halves_duration() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let f = fixture(dir.path());
    let out = dir.path().join("fast.mp4");
    let v = run_json(&[
        "speed",
        f.to_str().unwrap(),
        "--factor",
        "2",
        "-o",
        out.to_str().unwrap(),
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let d = v["probe"]["duration"].as_f64().unwrap();
    assert!(
        d > 0.35 && d < 0.7,
        "2x of 1s clip should be ~0.5s, got {d}; {v}"
    );
    assert_eq!(v["extra"]["factor"], 2.0);
    assert!(out.metadata().unwrap().len() > 0);
}

#[test]
fn music_ducks_bed_to_talk_duration() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let talk = fixture(dir.path());
    let bed = dir.path().join("bed.wav");
    let status = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=220:duration=2",
            "-c:a",
            "pcm_s16le",
        ])
        .arg(&bed)
        .status()
        .unwrap();
    assert!(status.success());
    let out = dir.path().join("mix.mp4");
    let v = run_json(&[
        "music",
        talk.to_str().unwrap(),
        "--track",
        bed.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    assert_eq!(v["probe"]["has_audio"], true, "{v}");
    assert_eq!(v["probe"]["has_video"], true, "{v}");
    let d = v["probe"]["duration"].as_f64().unwrap();
    assert!(
        d > 0.8 && d < 1.4,
        "mix should follow the 1s talk, not the 2s bed; got {d}; {v}"
    );
    assert_eq!(v["extra"]["duck"], true);
    assert!(out.metadata().unwrap().len() > 0);
}

#[test]
fn jumpcut_drops_middle_silence() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("gaps.mp4");
    let status = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc=duration=1.2:size=320x240:rate=30",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=1.2",
            "-af",
            "volume=0:enable='between(t,0.4,0.9)'",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            "-shortest",
        ])
        .arg(&f)
        .status()
        .unwrap();
    assert!(status.success());
    let out = dir.path().join("tight.mp4");
    let v = run_json(&[
        "jumpcut",
        f.to_str().unwrap(),
        "--min-duration",
        "0.25",
        "-o",
        out.to_str().unwrap(),
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let d = v["probe"]["duration"].as_f64().unwrap();
    assert!(
        d > 0.5 && d < 1.05,
        "1.2s with ~0.5s silence should land under 1.05s, got {d}; {v}"
    );
    assert!(v["extra"]["removed_seconds"].as_f64().unwrap() > 0.2, "{v}");
    assert!(out.metadata().unwrap().len() > 0);
}

fn gappy_talk(dir: &Path) -> PathBuf {
    let f = dir.join("gaps.mp4");
    let status = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc=duration=1.2:size=320x240:rate=30",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=1.2",
            "-af",
            "volume=0:enable='between(t,0.4,0.9)'",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            "-shortest",
        ])
        .arg(&f)
        .status()
        .unwrap();
    assert!(status.success());
    f
}

#[test]
fn rough_lists_speech_islands_without_writing() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let f = gappy_talk(dir.path());
    let v = run_json(&[
        "rough",
        f.to_str().unwrap(),
        "--min-duration",
        "0.25",
        "--pad",
        "0.05",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    assert_eq!(v["tool"], "rough");
    assert!(v["output"].is_null(), "{v}");
    let kept = v["extra"]["kept"].as_u64().unwrap();
    assert!(kept >= 2, "expected two speech islands, got {v}");
    let speech = v["extra"]["speech_seconds"].as_f64().unwrap();
    assert!(
        speech > 0.4 && speech < 1.05,
        "speech should be the take minus the hole, got {speech}; {v}"
    );
}

#[test]
fn rough_assembles_only_speech() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let f = gappy_talk(dir.path());
    let out = dir.path().join("rough.mp4");
    let v = run_json(&[
        "rough",
        f.to_str().unwrap(),
        "--min-duration",
        "0.25",
        "--pad",
        "0.05",
        "-o",
        out.to_str().unwrap(),
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let d = v["probe"]["duration"].as_f64().unwrap();
    assert!(
        d > 0.5 && d < 1.05,
        "assembled rough cut should drop the hole, got {d}; {v}"
    );
    assert_eq!(v["probe"]["has_audio"], true, "{v}");
    assert!(out.metadata().unwrap().len() > 0);
}

#[test]
fn cover_is_1080x1920_png() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let f = fixture(dir.path());
    let out = dir.path().join("cover.png");
    let v = run_json(&[
        "cover",
        f.to_str().unwrap(),
        "--at",
        "0.2",
        "-o",
        out.to_str().unwrap(),
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    assert_eq!(v["probe"]["width"], 1080, "{v}");
    assert_eq!(v["probe"]["height"], 1920, "{v}");
    assert_eq!(v["extra"]["frame"], "1080x1920");
    assert!(out.metadata().unwrap().len() > 0);
}

#[test]
fn fade_in_darkens_first_frame() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let f = fixture(dir.path());
    let out = dir.path().join("faded.mp4");
    let v = run_json(&[
        "fade",
        f.to_str().unwrap(),
        "--in",
        "0.4",
        "--out",
        "0.2",
        "-o",
        out.to_str().unwrap(),
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    assert!(v["probe"]["duration"].as_f64().unwrap() > 0.8, "{v}");
    let start = dir.path().join("t0.png");
    let mid = dir.path().join("t5.png");
    assert_eq!(
        run_json(&[
            "look",
            out.to_str().unwrap(),
            "--at",
            "0",
            "-o",
            start.to_str().unwrap()
        ])["status"],
        "ok"
    );
    assert_eq!(
        run_json(&[
            "look",
            out.to_str().unwrap(),
            "--at",
            "0.5",
            "-o",
            mid.to_str().unwrap()
        ])["status"],
        "ok"
    );
    let luma = |p: &Path| {
        let img = image::open(p).unwrap().to_rgb8();
        let s: u64 = img
            .pixels()
            .map(|px| px[0] as u64 + px[1] as u64 + px[2] as u64)
            .sum();
        s / (img.width() as u64 * img.height() as u64)
    };
    let a = luma(&start);
    let b = luma(&mid);
    assert!(
        a < b,
        "fade-in first frame should be darker than mid-clip ({a} vs {b})"
    );
}

#[test]
fn title_hook_puts_white_on_blue() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("blue.mp4");
    let status = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "color=c=0x0033aa:s=320x240:d=1:rate=30",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=1",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            "-shortest",
        ])
        .arg(&f)
        .status()
        .unwrap();
    assert!(status.success());
    let out = dir.path().join("hook.mp4");
    let mut args = vec![
        "title".to_string(),
        f.to_string_lossy().into_owned(),
        "--text".into(),
        "HELLO".into(),
        "--duration".into(),
        "0.6".into(),
        "-o".into(),
        out.to_string_lossy().into_owned(),
    ];
    let arial = "/System/Library/Fonts/Supplemental/Arial.ttf";
    if std::path::Path::new(arial).is_file() {
        args.push("--font".into());
        args.push(arial.into());
    }
    let argv: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let v = run_json(&argv);
    assert_eq!(v["status"], "ok", "{v}");
    assert_eq!(v["extra"]["text"], "HELLO");
    let frame = dir.path().join("t.png");
    assert_eq!(
        run_json(&[
            "look",
            out.to_str().unwrap(),
            "--at",
            "0.2",
            "-o",
            frame.to_str().unwrap()
        ])["status"],
        "ok"
    );
    let img = image::open(&frame).unwrap().to_rgb8();
    let white = img
        .pixels()
        .filter(|p| p[0] > 230 && p[1] > 230 && p[2] > 230)
        .count();
    assert!(white > 20, "title should paint white glyphs, got {white}");
}

#[test]
fn loop_triples_duration() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let f = fixture(dir.path());
    let out = dir.path().join("looped.mp4");
    let v = run_json(&[
        "loop",
        f.to_str().unwrap(),
        "--times",
        "3",
        "-o",
        out.to_str().unwrap(),
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let d = v["probe"]["duration"].as_f64().unwrap();
    assert!(
        d > 2.7 && d < 3.4,
        "3× a 1s clip should be ~3s, got {d}; {v}"
    );
    assert_eq!(v["extra"]["times"], 3);
    assert!(out.metadata().unwrap().len() > 0);
}

#[test]
fn stabilize_keeps_duration() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let f = fixture(dir.path());
    let out = dir.path().join("steady.mp4");
    let v = run_json(&[
        "stabilize",
        f.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    assert_eq!(v["probe"]["has_video"], true, "{v}");
    let d = v["probe"]["duration"].as_f64().unwrap();
    assert!(d > 0.8 && d < 1.3, "deshake should keep ~1s, got {d}; {v}");
    assert_eq!(v["extra"]["filter"], "deshake");
    assert!(out.metadata().unwrap().len() > 0);
}

#[test]
fn reverse_first_frame_differs_from_source() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let f = fixture(dir.path());
    let out = dir.path().join("rev.mp4");
    let v = run_json(&["reverse", f.to_str().unwrap(), "-o", out.to_str().unwrap()]);
    assert_eq!(v["status"], "ok", "{v}");
    let d = v["probe"]["duration"].as_f64().unwrap();
    assert!(d > 0.8 && d < 1.3, "reverse should keep ~1s, got {d}; {v}");
    let a = dir.path().join("src.png");
    let b = dir.path().join("rev.png");
    assert_eq!(
        run_json(&[
            "look",
            f.to_str().unwrap(),
            "--at",
            "0",
            "-o",
            a.to_str().unwrap()
        ])["status"],
        "ok"
    );
    assert_eq!(
        run_json(&[
            "look",
            out.to_str().unwrap(),
            "--at",
            "0",
            "-o",
            b.to_str().unwrap()
        ])["status"],
        "ok"
    );
    let pa = image::open(&a).unwrap().to_rgb8();
    let pb = image::open(&b).unwrap().to_rgb8();
    let diff: u64 = pa
        .pixels()
        .zip(pb.pixels())
        .map(|(x, y)| {
            (x[0] as i16 - y[0] as i16).unsigned_abs() as u64
                + (x[1] as i16 - y[1] as i16).unsigned_abs() as u64
                + (x[2] as i16 - y[2] as i16).unsigned_abs() as u64
        })
        .sum();
    assert!(
        diff > 10_000,
        "reversed t=0 should not match source t=0 (diff={diff})"
    );
}

#[test]
fn grade_raises_chroma() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let f = fixture(dir.path());
    let out = dir.path().join("pop.mp4");
    let v = run_json(&[
        "grade",
        f.to_str().unwrap(),
        "--saturation",
        "2",
        "--contrast",
        "1",
        "--brightness",
        "0",
        "-o",
        out.to_str().unwrap(),
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let src = dir.path().join("s.png");
    let grd = dir.path().join("g.png");
    assert_eq!(
        run_json(&[
            "look",
            f.to_str().unwrap(),
            "--at",
            "0.3",
            "-o",
            src.to_str().unwrap()
        ])["status"],
        "ok"
    );
    assert_eq!(
        run_json(&[
            "look",
            out.to_str().unwrap(),
            "--at",
            "0.3",
            "-o",
            grd.to_str().unwrap()
        ])["status"],
        "ok"
    );
    let chroma = |p: &std::path::Path| {
        let img = image::open(p).unwrap().to_rgb8();
        img.pixels()
            .map(|px| {
                (px[0] as i32 - px[1] as i32).unsigned_abs() as u64
                    + (px[1] as i32 - px[2] as i32).unsigned_abs() as u64
                    + (px[2] as i32 - px[0] as i32).unsigned_abs() as u64
            })
            .sum::<u64>()
    };
    let a = chroma(&src);
    let b = chroma(&grd);
    assert!(b > a, "sat 2.0 should raise chroma ({a} -> {b})");
}

#[test]
fn zoom_keeps_frame_but_changes_pixels() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let f = fixture(dir.path());
    let out = dir.path().join("z.mp4");
    let v = run_json(&[
        "zoom",
        f.to_str().unwrap(),
        "--factor",
        "1.5",
        "-o",
        out.to_str().unwrap(),
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    assert_eq!(v["probe"]["width"], 320, "{v}");
    assert_eq!(v["probe"]["height"], 240, "{v}");
    let a = dir.path().join("s.png");
    let b = dir.path().join("z.png");
    assert_eq!(
        run_json(&[
            "look",
            f.to_str().unwrap(),
            "--at",
            "0.3",
            "-o",
            a.to_str().unwrap()
        ])["status"],
        "ok"
    );
    assert_eq!(
        run_json(&[
            "look",
            out.to_str().unwrap(),
            "--at",
            "0.3",
            "-o",
            b.to_str().unwrap()
        ])["status"],
        "ok"
    );
    let pa = image::open(&a).unwrap().to_rgb8();
    let pb = image::open(&b).unwrap().to_rgb8();
    let diff: u64 = pa
        .pixels()
        .zip(pb.pixels())
        .map(|(x, y)| {
            (x[0] as i16 - y[0] as i16).unsigned_abs() as u64
                + (x[1] as i16 - y[1] as i16).unsigned_abs() as u64
                + (x[2] as i16 - y[2] as i16).unsigned_abs() as u64
        })
        .sum();
    assert!(
        diff > 10_000,
        "1.5x center crop should change pixels (diff={diff})"
    );
}

#[test]
fn sharpen_raises_edge_energy() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let f = fixture(dir.path());
    let out = dir.path().join("sh.mp4");
    let v = run_json(&[
        "sharpen",
        f.to_str().unwrap(),
        "--amount",
        "1.5",
        "-o",
        out.to_str().unwrap(),
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let src = dir.path().join("s.png");
    let shp = dir.path().join("k.png");
    assert_eq!(
        run_json(&[
            "look",
            f.to_str().unwrap(),
            "--at",
            "0.3",
            "-o",
            src.to_str().unwrap()
        ])["status"],
        "ok"
    );
    assert_eq!(
        run_json(&[
            "look",
            out.to_str().unwrap(),
            "--at",
            "0.3",
            "-o",
            shp.to_str().unwrap()
        ])["status"],
        "ok"
    );
    let edge = |p: &std::path::Path| {
        let img = image::open(p).unwrap().to_luma8();
        let (w, h) = img.dimensions();
        let mut e = 0u64;
        for y in 0..h.saturating_sub(1) {
            for x in 0..w.saturating_sub(1) {
                let p0 = img.get_pixel(x, y)[0] as i16;
                let pr = img.get_pixel(x + 1, y)[0] as i16;
                let pd = img.get_pixel(x, y + 1)[0] as i16;
                e += (p0 - pr).unsigned_abs() as u64 + (p0 - pd).unsigned_abs() as u64;
            }
        }
        e
    };
    let a = edge(&src);
    let b = edge(&shp);
    assert!(b > a, "unsharp should raise edge energy ({a} -> {b})");
}

#[test]
fn vignette_darkens_corners() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let f = fixture(dir.path());
    let out = dir.path().join("v.mp4");
    let v = run_json(&["vignette", f.to_str().unwrap(), "-o", out.to_str().unwrap()]);
    assert_eq!(v["status"], "ok", "{v}");
    let png = dir.path().join("f.png");
    assert_eq!(
        run_json(&[
            "look",
            out.to_str().unwrap(),
            "--at",
            "0.3",
            "-o",
            png.to_str().unwrap()
        ])["status"],
        "ok"
    );
    let img = image::open(&png).unwrap().to_luma8();
    let (w, h) = img.dimensions();
    let luma = |x0: u32, y0: u32, ww: u32, hh: u32| {
        let mut s = 0u64;
        let mut n = 0u64;
        for y in y0..(y0 + hh).min(h) {
            for x in x0..(x0 + ww).min(w) {
                s += img.get_pixel(x, y)[0] as u64;
                n += 1;
            }
        }
        s / n.max(1)
    };
    let corner = luma(0, 0, 12, 12);
    let center = luma(w / 2 - 8, h / 2 - 8, 16, 16);
    assert!(
        corner < center,
        "vignette corners should be darker than center ({corner} vs {center})"
    );
}

#[test]
fn bw_drops_chroma() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let f = fixture(dir.path());
    let out = dir.path().join("bw.mp4");
    let v = run_json(&["bw", f.to_str().unwrap(), "-o", out.to_str().unwrap()]);
    assert_eq!(v["status"], "ok", "{v}");
    let src = dir.path().join("s.png");
    let bw = dir.path().join("b.png");
    assert_eq!(
        run_json(&[
            "look",
            f.to_str().unwrap(),
            "--at",
            "0.3",
            "-o",
            src.to_str().unwrap()
        ])["status"],
        "ok"
    );
    assert_eq!(
        run_json(&[
            "look",
            out.to_str().unwrap(),
            "--at",
            "0.3",
            "-o",
            bw.to_str().unwrap()
        ])["status"],
        "ok"
    );
    let chroma = |p: &std::path::Path| {
        let img = image::open(p).unwrap().to_rgb8();
        img.pixels()
            .map(|px| {
                (px[0] as i32 - px[1] as i32).unsigned_abs() as u64
                    + (px[1] as i32 - px[2] as i32).unsigned_abs() as u64
                    + (px[2] as i32 - px[0] as i32).unsigned_abs() as u64
            })
            .sum::<u64>()
    };
    let a = chroma(&src);
    let b = chroma(&bw);
    assert!(b < a / 4, "bw should collapse chroma ({a} -> {b})");
}

#[test]
fn volume_db_lowers_mean() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let f = fixture(dir.path());
    let out = dir.path().join("quiet.mp4");
    let v = run_json(&[
        "volume",
        f.to_str().unwrap(),
        "--db",
        "-6",
        "-o",
        out.to_str().unwrap(),
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let mean = v["extra"]["mean_volume"]
        .as_f64()
        .expect("mean_volume in extra");
    let src = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-i",
            f.to_str().unwrap(),
            "-af",
            "volumedetect",
            "-vn",
            "-f",
            "null",
            "-",
        ])
        .output()
        .unwrap();
    let err = String::from_utf8_lossy(&src.stderr);
    let src_mean = err
        .lines()
        .find_map(|l| {
            l.split("mean_volume:")
                .nth(1)
                .and_then(|r| r.split_whitespace().next())
                .and_then(|t| t.parse::<f64>().ok())
        })
        .expect("source mean_volume");
    assert!(
        mean < src_mean - 2.5,
        "−6 dB should drop mean_volume ({src_mean} -> {mean}); {v}"
    );
}

#[test]
fn blur_lowers_edge_energy() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let f = fixture(dir.path());
    let out = dir.path().join("b.mp4");
    let v = run_json(&[
        "blur",
        f.to_str().unwrap(),
        "--sigma",
        "4",
        "-o",
        out.to_str().unwrap(),
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let src = dir.path().join("s.png");
    let bl = dir.path().join("k.png");
    assert_eq!(
        run_json(&[
            "look",
            f.to_str().unwrap(),
            "--at",
            "0.3",
            "-o",
            src.to_str().unwrap()
        ])["status"],
        "ok"
    );
    assert_eq!(
        run_json(&[
            "look",
            out.to_str().unwrap(),
            "--at",
            "0.3",
            "-o",
            bl.to_str().unwrap()
        ])["status"],
        "ok"
    );
    let edge = |p: &std::path::Path| {
        let img = image::open(p).unwrap().to_luma8();
        let (w, h) = img.dimensions();
        let mut e = 0u64;
        for y in 0..h.saturating_sub(1) {
            for x in 0..w.saturating_sub(1) {
                let p0 = img.get_pixel(x, y)[0] as i16;
                let pr = img.get_pixel(x + 1, y)[0] as i16;
                let pd = img.get_pixel(x, y + 1)[0] as i16;
                e += (p0 - pr).unsigned_abs() as u64 + (p0 - pd).unsigned_abs() as u64;
            }
        }
        e
    };
    let a = edge(&src);
    let b = edge(&bl);
    assert!(b < a, "gblur should lower edge energy ({a} -> {b})");
}

#[test]
fn pipeline_runs_cut_then_fit() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let f = fixture(dir.path());
    let cut = dir.path().join("01.mp4");
    let out = dir.path().join("reel.mp4");
    let plan_path = dir.path().join("plan.json");
    let plan = serde_json::json!({
        "goal": "9:16 clip of the first half second",
        "steps": [
            {
                "tool": "cut",
                "argv": [f.to_str().unwrap(), "--start", "0", "--end", "0.5", "-o", cut.to_str().unwrap()]
            },
            {
                "tool": "fit",
                "argv": [cut.to_str().unwrap(), "--aspect", "9:16", "--fit", "pad", "-o", out.to_str().unwrap()]
            }
        ]
    });
    std::fs::write(&plan_path, serde_json::to_string(&plan).unwrap()).unwrap();
    let v = run_json(&["pipeline", plan_path.to_str().unwrap()]);
    assert_eq!(v["status"], "ok", "{v}");
    assert_eq!(v["tool"], "pipeline");
    assert_eq!(v["extra"]["goal"], "9:16 clip of the first half second");
    assert_eq!(v["probe"]["width"], 1080, "{v}");
    assert_eq!(v["probe"]["height"], 1920, "{v}");
    let d = v["probe"]["duration"].as_f64().unwrap();
    assert!(d > 0.35 && d < 0.7, "cut to 0.5s then fit, got {d}; {v}");
    assert_eq!(v["extra"]["steps"].as_array().unwrap().len(), 2);
    assert!(out.metadata().unwrap().len() > 0);
}

#[test]
fn pipeline_src_in_and_expect() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let f = fixture(dir.path());
    let cut = dir.path().join("01.mp4");
    let out = dir.path().join("reel.mp4");
    let plan_path = dir.path().join("plan.json");
    let plan = serde_json::json!({
        "goal": "9:16 clip of the first half second",
        "input": f.to_str().unwrap(),
        "expect": {
            "aspect": "9:16",
            "width": 1080,
            "height": 1920,
            "duration_gt": 0.35,
            "duration_lt": 0.7,
            "has_video": true,
            "ext": "mp4"
        },
        "steps": [
            {
                "tool": "cut",
                "label": "剪前半秒",
                "argv": ["$src", "--start", "0", "--end", "0.5", "-o", cut.to_str().unwrap()]
            },
            {
                "tool": "fit",
                "label": "竖屏",
                "argv": ["$in", "--aspect", "9:16", "--fit", "pad", "-o", out.to_str().unwrap()]
            }
        ]
    });
    std::fs::write(&plan_path, serde_json::to_string(&plan).unwrap()).unwrap();
    let v = run_json(&["pipeline", plan_path.to_str().unwrap()]);
    assert_eq!(v["status"], "ok", "{v}");
    assert_eq!(v["verified"], true, "{v}");
    assert_eq!(v["extra"]["expect"]["ok"], true, "{v}");
    assert_eq!(v["extra"]["steps"][0]["label"], "剪前半秒");
    assert_eq!(v["probe"]["width"], 1080, "{v}");
    assert_eq!(v["probe"]["height"], 1920, "{v}");
    assert!(out.metadata().unwrap().len() > 0);
}

#[test]
fn pipeline_expect_mismatch_keeps_file() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let f = fixture(dir.path());
    let out = dir.path().join("out.mp4");
    let plan_path = dir.path().join("plan.json");
    let plan = serde_json::json!({
        "goal": "should be two seconds (it will not)",
        "input": f.to_str().unwrap(),
        "expect": { "duration_gt": 2.0 },
        "steps": [{
            "tool": "cut",
            "argv": ["$src", "--start", "0", "--end", "0.5", "-o", out.to_str().unwrap()]
        }]
    });
    std::fs::write(&plan_path, serde_json::to_string(&plan).unwrap()).unwrap();
    let v = run_json(&["pipeline", plan_path.to_str().unwrap()]);
    assert_eq!(v["status"], "ok", "{v}");
    assert_eq!(v["verified"], false, "{v}");
    assert_eq!(v["extra"]["expect"]["ok"], false, "{v}");
    assert!(out.metadata().unwrap().len() > 0);
}

#[test]
fn pipeline_in_on_first_step_fails() {
    let dir = tempfile::tempdir().unwrap();
    let plan_path = dir.path().join("plan.json");
    let plan = serde_json::json!({
        "goal": "no previous output",
        "steps": [{
            "tool": "cut",
            "argv": ["$in", "--start", "0", "--end", "0.5", "-o", "out.mp4"]
        }]
    });
    std::fs::write(&plan_path, serde_json::to_string(&plan).unwrap()).unwrap();
    let v = run_json(&["pipeline", plan_path.to_str().unwrap()]);
    assert_eq!(v["status"], "failed", "{v}");
    let msg = v["error"]["message"].as_str().unwrap_or("");
    assert!(msg.contains("$in"), "{v}");
}

#[test]
fn pipeline_rejects_nested_pipeline() {
    let dir = tempfile::tempdir().unwrap();
    let plan_path = dir.path().join("plan.json");
    let plan = serde_json::json!({
        "goal": "must not recurse",
        "steps": [{"tool": "pipeline", "argv": ["other.json"]}]
    });
    std::fs::write(&plan_path, serde_json::to_string(&plan).unwrap()).unwrap();
    let v = run_json(&["pipeline", plan_path.to_str().unwrap()]);
    assert_eq!(v["status"], "failed", "{v}");
    let msg = v["error"]["message"].as_str().unwrap_or("");
    assert!(msg.contains("pipeline"), "{v}");
}

#[test]
fn pipeline_ffmpeg_step_needs_because() {
    let dir = tempfile::tempdir().unwrap();
    let plan_path = dir.path().join("plan.json");
    let out = dir.path().join("out.mp4");
    let plan = serde_json::json!({
        "goal": "raw ffmpeg without because",
        "steps": [{
            "tool": "ffmpeg",
            "argv": ["--", "-i", "in.mp4", out.to_str().unwrap()]
        }]
    });
    std::fs::write(&plan_path, serde_json::to_string(&plan).unwrap()).unwrap();
    let v = run_json(&["pipeline", plan_path.to_str().unwrap()]);
    assert_eq!(v["status"], "failed", "{v}");
    let msg = v["error"]["message"].as_str().unwrap_or("");
    assert!(msg.contains("because"), "{v}");
}

#[test]
fn ffmpeg_requires_because() {
    let out = ffkit()
        .args(["ffmpeg", "--", "-i", "in.mp4", "out.mp4"])
        .output()
        .unwrap();
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("because"),
        "stderr should require --because, got {err}"
    );
}

#[test]
fn version_reports_match() {
    let v = run_json(&["version"]);
    assert_eq!(v["status"], "ok", "{v}");
    assert_eq!(v["extra"]["ffkit"], env!("CARGO_PKG_VERSION"));
    assert_eq!(v["extra"]["skill"], env!("CARGO_PKG_VERSION"));
    assert_eq!(v["extra"]["skill_match"], true);
}

#[test]
fn doctor_reports_skill_match() {
    if !has_ffmpeg() {
        return;
    }
    let v = run_json(&["doctor"]);
    assert_eq!(v["extra"]["ffkit"]["skill_match"], true);
    assert_eq!(v["extra"]["ffkit"]["version"], env!("CARGO_PKG_VERSION"));
}

#[test]
fn install_skill_dry_run() {
    let v = run_json(&["install-skill", "--dry-run"]);
    assert!(v["status"] == "ok" || v["status"] == "dry_run", "{v}");
    assert!(!v["extra"]["targets"].as_array().unwrap().is_empty());
}

#[test]
fn pack_release_zip_runs() {
    let dir = tempfile::tempdir().unwrap();
    let dist = dir.path();
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let pack = root.join("scripts/pack-release.sh");
    let status = Command::new("bash")
        .arg(&pack)
        .env("BIN", env!("CARGO_BIN_EXE_ffkit"))
        .env("DIST", dist)
        .current_dir(root)
        .status()
        .expect("pack-release.sh");
    assert!(status.success(), "pack-release.sh failed");
    let host =
        String::from_utf8(Command::new("rustc").args(["-vV"]).output().unwrap().stdout).unwrap();
    let target = host
        .lines()
        .find_map(|l| l.strip_prefix("host: "))
        .expect("rustc host");
    let ver = env!("CARGO_PKG_VERSION");
    let zip = dist.join(format!("ffkit-{ver}-{target}.zip"));
    assert!(zip.is_file(), "missing {zip:?}");
    let dest = dist.join("out");
    std::fs::create_dir_all(&dest).unwrap();
    let unzip = Command::new("unzip")
        .args(["-q", zip.to_str().unwrap(), "-d", dest.to_str().unwrap()])
        .status()
        .expect("unzip");
    assert!(unzip.success());
    let folder = dest.join(format!("ffkit-{ver}-{target}"));
    assert!(folder.join("SKILL.md").is_file());
    assert!(folder.join("README.md").is_file());
    assert!(folder.join("README.zh.md").is_file());
    assert!(folder.join("install.sh").is_file());
    assert!(folder.join("references/pipeline.md").is_file());
    assert!(folder.join("references/recipes.md").is_file());
    let bin = folder.join("ffkit");
    let out = Command::new(&bin)
        .args(["--json", "version"])
        .output()
        .expect("zip ffkit version");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let v: Value = serde_json::from_str(&stdout).unwrap_or_else(|_| panic!("{stdout}"));
    assert_eq!(v["status"], "ok", "{v}");
    assert_eq!(v["extra"]["ffkit"], ver, "{v}");
}

#[test]
fn denoise_keeps_streams() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let f = fixture(dir.path());
    let out = dir.path().join("clean.mp4");
    let v = run_json(&["denoise", f.to_str().unwrap(), "-o", out.to_str().unwrap()]);
    assert_eq!(v["status"], "ok", "{v}");
    assert_eq!(v["tool"], "denoise");
    assert_eq!(v["probe"]["has_audio"], true, "{v}");
    assert_eq!(v["probe"]["has_video"], true, "{v}");
    let af = v["extra"]["af"].as_str().unwrap();
    assert!(af.contains("afwtdn") || af.contains("afftdn"), "{v}");
    assert_eq!(v["extra"]["video_denoise"], false, "{v}");
    let d = v["probe"]["duration"].as_f64().unwrap();
    assert!(d > 0.8 && d < 1.3, "denoise keeps ~1s, got {d}; {v}");
}

#[test]
fn compress_lands_under_size() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let f = fixture(dir.path());
    let out = dir.path().join("small.mp4");
    let v = run_json(&[
        "compress",
        f.to_str().unwrap(),
        "--size",
        "150KB",
        "-o",
        out.to_str().unwrap(),
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    assert_eq!(v["extra"]["passes"], 2, "{v}");
    assert!(v["extra"]["video_kbps"].as_f64().unwrap() > 0.0, "{v}");
    let bytes = out.metadata().unwrap().len() as f64;
    assert!(
        bytes < 150_000.0,
        "150KB target should land under it, got {bytes}; {v}"
    );
    let d = v["probe"]["duration"].as_f64().unwrap();
    assert!(
        d > 0.8 && d < 1.3,
        "compressed clip keeps ~1s, got {d}; {v}"
    );
}

#[test]
fn compress_refuses_impossible_size() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let f = fixture(dir.path());
    let out = dir.path().join("tiny.mp4");
    let v = run_json(&[
        "compress",
        f.to_str().unwrap(),
        "--size",
        "20KB",
        "-o",
        out.to_str().unwrap(),
    ]);
    assert_eq!(v["status"], "failed", "{v}");
    let msg = v["error"]["message"].as_str().unwrap_or("");
    assert!(msg.contains("--size"), "{v}");
}

#[test]
fn fit_blur_pillarboxes() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let f = fixture(dir.path());
    let out = dir.path().join("blurred.mp4");
    let v = run_json(&[
        "fit",
        f.to_str().unwrap(),
        "--aspect",
        "9:16",
        "--fit",
        "blur",
        "-o",
        out.to_str().unwrap(),
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    assert_eq!(v["probe"]["width"], 1080, "{v}");
    assert_eq!(v["probe"]["height"], 1920, "{v}");

    let png = dir.path().join("top.png");
    assert_eq!(
        run_json(&[
            "look",
            out.to_str().unwrap(),
            "--at",
            "0.3",
            "-o",
            png.to_str().unwrap()
        ])["status"],
        "ok"
    );
    let img = image::open(&png).unwrap().to_rgb8();
    // The blurred backdrop fills where black bars would sit — the top of the
    // frame should carry the clip's colours, not pad black.
    let (w, _) = img.dimensions();
    let mut lit = 0u32;
    for x in 0..w {
        let p = img.get_pixel(x, 8);
        if p[0] > 40 || p[1] > 40 || p[2] > 40 {
            lit += 1;
        }
    }
    assert!(
        lit > w / 2,
        "blur fill should cover the top band (lit {lit} of {w} pixels)"
    );
}

#[test]
fn audiogram_paints_waves_on_cover() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("talk.wav");
    let status = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=1",
            "-c:a",
            "pcm_s16le",
        ])
        .arg(&wav)
        .status()
        .unwrap();
    assert!(status.success());
    let out = dir.path().join("reel.mp4");
    let v = run_json(&[
        "audiogram",
        wav.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    assert_eq!(v["probe"]["width"], 1080, "{v}");
    assert_eq!(v["probe"]["height"], 1920, "{v}");
    assert_eq!(v["probe"]["has_audio"], true, "{v}");
    let d = v["probe"]["duration"].as_f64().unwrap();
    assert!(d > 0.8 && d < 1.3, "audiogram keeps ~1s, got {d}; {v}");
}

#[test]
fn replace_swaps_audio_keeps_video_and_duration() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let f = fixture(dir.path());
    // 2s replacement audio; fixture video is ~1s so the bed must be trimmed.
    let wav = dir.path().join("bed.wav");
    let status = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=880:duration=2",
            "-c:a",
            "pcm_s16le",
        ])
        .arg(&wav)
        .status()
        .unwrap();
    assert!(status.success());
    let out = dir.path().join("swapped.mp4");
    let v = run_json(&[
        "replace",
        f.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--audio",
        wav.to_str().unwrap(),
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    assert_eq!(v["tool"], "replace");
    assert_eq!(v["probe"]["has_video"], true, "{v}");
    assert_eq!(v["probe"]["has_audio"], true, "{v}");
    let d = v["probe"]["duration"].as_f64().unwrap();
    assert!(
        d > 0.8 && d < 1.3,
        "replace keeps video length, got {d}; {v}"
    );
}

#[test]
fn replace_refuses_audio_without_stream() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let f = fixture(dir.path());
    // A silent video as the --audio source must be refused.
    let silent = dir.path().join("silent.mp4");
    let status = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc=duration=1:size=64x64:rate=10",
            "-an",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
        ])
        .arg(&silent)
        .status()
        .unwrap();
    assert!(status.success());
    let v = run_json(&[
        "replace",
        f.to_str().unwrap(),
        "-o",
        dir.path().join("x.mp4").to_str().unwrap(),
        "--audio",
        silent.to_str().unwrap(),
    ]);
    assert_eq!(v["status"], "failed", "{v}");
    // An mp4 that *does* carry audio is a valid --audio source too.
    let v = run_json(&[
        "replace",
        f.to_str().unwrap(),
        "-o",
        dir.path().join("y.mp4").to_str().unwrap(),
        "--audio",
        f.to_str().unwrap(),
    ]);
    assert_eq!(v["status"], "ok", "{v}");
}

#[test]
fn grade_lut_shifts_pixels() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let f = fixture(dir.path());
    // Tiny 2x2x2 .cube that crushes red.
    let lut = dir.path().join("crush.cube");
    std::fs::write(
        &lut,
        "LUT_3D_SIZE 2\n0 0 0\n0 0 1\n0 1 0\n0 1 1\n0 0 0\n0 0 1\n0 1 0\n0 1 1\n",
    )
    .unwrap();
    let out = dir.path().join("lutted.mp4");
    let v = run_json(&[
        "grade",
        f.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--lut",
        lut.to_str().unwrap(),
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    assert_eq!(
        v["extra"]["lut"].as_str().unwrap(),
        lut.to_str().unwrap(),
        "{v}"
    );
}

#[test]
fn slideshow_assembles_stills_with_bed() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let mut imgs = vec![];
    for (i, c) in ["red", "green", "blue"].iter().enumerate() {
        let p = dir.path().join(format!("i{i}.png"));
        let status = Command::new("ffmpeg")
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                &format!("color=c={c}:s=320x240:d=0.5"),
                "-frames:v",
                "1",
            ])
            .arg(&p)
            .status()
            .unwrap();
        assert!(status.success());
        imgs.push(p);
    }
    let out = dir.path().join("show.mp4");
    let v = run_json(&[
        "slideshow",
        imgs[0].to_str().unwrap(),
        imgs[1].to_str().unwrap(),
        imgs[2].to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--per",
        "1",
        "--fade",
        "0.2",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    assert_eq!(v["tool"], "slideshow");
    // 3*1 - 2*0.2 = 2.6 expected
    let d = v["probe"]["duration"].as_f64().unwrap();
    assert!((d - 2.6).abs() < 0.4, "slideshow ≈2.6s, got {d}; {v}");
    assert_eq!(v["probe"]["width"], 1920, "{v}");
    assert_eq!(v["probe"]["height"], 1080, "{v}");
    assert_eq!(v["probe"]["has_audio"], true, "{v}");
}

#[test]
fn slideshow_refuses_fade_longer_than_per() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a.png");
    let b = dir.path().join("b.png");
    for p in [&a, &b] {
        let status = Command::new("ffmpeg")
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                "color=c=red:s=64x64:d=0.5",
                "-frames:v",
                "1",
            ])
            .arg(p)
            .status()
            .unwrap();
        assert!(status.success());
    }
    let v = run_json(&[
        "slideshow",
        a.to_str().unwrap(),
        b.to_str().unwrap(),
        "-o",
        dir.path().join("s.mp4").to_str().unwrap(),
        "--per",
        "0.5",
        "--fade",
        "0.6",
    ]);
    assert_eq!(v["status"], "failed", "{v}");
}

#[test]
fn caption_chunk_splits_long_cues() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let f = fixture(dir.path());
    let srt = dir.path().join("c.srt");
    std::fs::write(
        &srt,
        "1\n00:00:00,000 --> 00:00:01,000\nHELLO THERE BRAVE NEW WORLD\n\n\
         2\n00:00:01,000 --> 00:00:01,800\nBYE NOW\n",
    )
    .unwrap();
    let out = dir.path().join("cap.mp4");
    let v = run_json(&[
        "caption",
        f.to_str().unwrap(),
        "--srt",
        srt.to_str().unwrap(),
        "--chunk",
        "2",
        "-o",
        out.to_str().unwrap(),
    ]);
    // 5 words -> 3 chunks of <=2; the short cue passes through: 4 total.
    assert_eq!(v["status"], "ok", "{v}");
    assert_eq!(v["extra"]["cues"], 4, "{v}");
    assert_eq!(v["extra"]["chunk"], 2, "{v}");
}

#[test]
fn caption_chunk_rejects_out_of_range() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let f = fixture(dir.path());
    let srt = dir.path().join("c.srt");
    std::fs::write(&srt, "1\n00:00:00,000 --> 00:00:01,000\nHELLO WORLD\n").unwrap();
    let v = run_json(&[
        "caption",
        f.to_str().unwrap(),
        "--srt",
        srt.to_str().unwrap(),
        "--chunk",
        "0",
        "-o",
        dir.path().join("x.mp4").to_str().unwrap(),
    ]);
    assert_eq!(v["status"], "failed", "{v}");
}

#[test]
fn replace_mix_blends_original_under_new_audio() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let f = fixture(dir.path());
    let bed = dir.path().join("bed.m4a");
    let status = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=880:duration=1",
            "-c:a",
            "aac",
        ])
        .arg(&bed)
        .status()
        .unwrap();
    assert!(status.success());
    let out = dir.path().join("mix.mp4");
    let v = run_json(&[
        "replace",
        f.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--audio",
        bed.to_str().unwrap(),
        "--mix",
        "0.3",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    assert_eq!(v["extra"]["mix"], 0.3, "{v}");
    assert!(out.exists());
    let d = v["probe"]["duration"].as_f64().unwrap();
    assert!(
        (d - 1.0).abs() < 0.3,
        "mix keeps video duration, got {d}; {v}"
    );

    // --mix on an input with no audio must be refused.
    let silent = dir.path().join("silent.mp4");
    let status = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc=duration=1:size=64x64:rate=10",
            "-an",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
        ])
        .arg(&silent)
        .status()
        .unwrap();
    assert!(status.success());
    let v = run_json(&[
        "replace",
        silent.to_str().unwrap(),
        "-o",
        dir.path().join("x.mp4").to_str().unwrap(),
        "--audio",
        bed.to_str().unwrap(),
        "--mix",
        "0.5",
    ]);
    assert_eq!(v["status"], "failed", "{v}");
}

#[test]
fn slideshow_kenburns_and_wipe_transition() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let mut imgs = vec![];
    for (i, c) in ["red", "green", "blue"].iter().enumerate() {
        let p = dir.path().join(format!("k{i}.png"));
        let status = Command::new("ffmpeg")
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                &format!("color=c={c}:s=320x240:d=0.5"),
                "-frames:v",
                "1",
            ])
            .arg(&p)
            .status()
            .unwrap();
        assert!(status.success());
        imgs.push(p);
    }
    let out = dir.path().join("kb.mp4");
    let v = run_json(&[
        "slideshow",
        imgs[0].to_str().unwrap(),
        imgs[1].to_str().unwrap(),
        imgs[2].to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--per",
        "1",
        "--fade",
        "0.2",
        "--motion",
        "kenburns",
        "--transition",
        "wipeleft",
        "--size",
        "640x360",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    assert_eq!(v["extra"]["motion"], "kenburns", "{v}");
    assert_eq!(v["extra"]["transition"], "wipeleft", "{v}");
    let d = v["probe"]["duration"].as_f64().unwrap();
    assert!(
        (d - 2.6).abs() < 0.4,
        "kenburns slideshow ≈2.6s, got {d}; {v}"
    );
    assert_eq!(v["probe"]["width"], 640, "{v}");
}

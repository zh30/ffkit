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

fn has_filter(name: &str) -> bool {
    Command::new("ffmpeg")
        .args(["-hide_banner", "-filters"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains(name))
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

#[test]
fn concat_transition_chains_three_clips() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let mut clips = vec![];
    for i in 0..3 {
        let p = dir.path().join(format!("c{i}.mp4"));
        let status = Command::new("ffmpeg")
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                "testsrc=duration=2:size=320x240:rate=30",
                "-f",
                "lavfi",
                "-i",
                &format!("sine=frequency={}:duration=2", 300 + i * 200),
                "-c:v",
                "libx264",
                "-pix_fmt",
                "yuv420p",
                "-c:a",
                "aac",
                "-shortest",
            ])
            .arg(&p)
            .status()
            .unwrap();
        assert!(status.success());
        clips.push(p);
    }
    let out = dir.path().join("cat.mp4");
    let v = run_json(&[
        "concat",
        clips[0].to_str().unwrap(),
        clips[1].to_str().unwrap(),
        clips[2].to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--transition",
        "wipeleft",
        "--duration",
        "0.3",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    assert_eq!(v["extra"]["transition"], "wipeleft", "{v}");
    assert_eq!(v["extra"]["clips"], 3, "{v}");
    // 3*2 - 2*0.3 = 5.4
    let d = v["probe"]["duration"].as_f64().unwrap();
    assert!((d - 5.4).abs() < 0.5, "concat chain ≈5.4s, got {d}; {v}");
    assert_eq!(v["probe"]["has_audio"], true, "{v}");
}

#[test]
fn concat_transition_refuses_clip_shorter_than_fade() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let a = fixture(dir.path());
    let b = fixture(dir.path());
    let v = run_json(&[
        "concat",
        a.to_str().unwrap(),
        b.to_str().unwrap(),
        "-o",
        dir.path().join("x.mp4").to_str().unwrap(),
        "--transition",
        "fade",
        "--duration",
        "2",
    ]);
    // fixture is 1s; a 2s transition can't fit.
    assert_eq!(v["status"], "failed", "{v}");
}

#[test]
fn split_cuts_equal_parts() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("long.mp4");
    let status = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc=duration=5:size=320x240:rate=30",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=5",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            "-shortest",
        ])
        .arg(&src)
        .status()
        .unwrap();
    assert!(status.success());
    let v = run_json(&[
        "split",
        src.to_str().unwrap(),
        "-o",
        dir.path().join("part.mp4").to_str().unwrap(),
        "--every",
        "2",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let parts = v["extra"]["parts"].as_array().unwrap();
    assert_eq!(parts.len(), 3, "5s / every 2 -> 3 parts; {v}");
    let d0 = v["probe"]["duration"].as_f64().unwrap();
    assert!((d0 - 2.0).abs() < 0.3, "first part ≈2s, got {d0}; {v}");
    for p in parts {
        assert!(std::path::Path::new(p.as_str().unwrap()).exists(), "{v}");
    }
}

#[test]
fn key_composites_greenscreen_over_background() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let fg = dir.path().join("fg.mp4");
    let status = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "color=c=0x00ff00:size=320x240:duration=1",
            "-f",
            "lavfi",
            "-i",
            "color=c=red:size=80x80:duration=1",
            "-filter_complex",
            "[0:v][1:v]overlay=100:80",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=1",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            "-shortest",
        ])
        .arg(&fg)
        .status()
        .unwrap();
    assert!(status.success());
    let bg = dir.path().join("bg.png");
    let status = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "color=c=blue:size=640x480:duration=0.5",
            "-frames:v",
            "1",
        ])
        .arg(&bg)
        .status()
        .unwrap();
    assert!(status.success());
    let out = dir.path().join("key.mp4");
    let v = run_json(&[
        "key",
        fg.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--bg",
        bg.to_str().unwrap(),
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    assert_eq!(v["extra"]["bg_is_still"], true, "{v}");
    let d = v["probe"]["duration"].as_f64().unwrap();
    assert!((d - 1.0).abs() < 0.3, "key keeps fg duration, got {d}; {v}");
}

#[test]
fn key_refuses_bad_color() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let f = fixture(dir.path());
    let v = run_json(&[
        "key",
        f.to_str().unwrap(),
        "-o",
        dir.path().join("x.mp4").to_str().unwrap(),
        "--bg",
        f.to_str().unwrap(),
        "--color",
        "green",
    ]);
    assert_eq!(v["status"], "failed", "{v}");
}

#[test]
fn split_at_chapter_points() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("long.mp4");
    let status = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc=duration=5:size=320x240:rate=30",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=5",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            "-shortest",
        ])
        .arg(&src)
        .status()
        .unwrap();
    assert!(status.success());
    let v = run_json(&[
        "split",
        src.to_str().unwrap(),
        "-o",
        dir.path().join("ch.mp4").to_str().unwrap(),
        "--at",
        "2,3.5",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let parts = v["extra"]["parts"].as_array().unwrap();
    assert_eq!(parts.len(), 3, "cuts at 2,3.5 -> 3 parts; {v}");
    let d0 = v["probe"]["duration"].as_f64().unwrap();
    assert!((d0 - 2.0).abs() < 0.3, "first part ≈2s, got {d0}; {v}");

    // --every and --at together is refused.
    let v = run_json(&[
        "split",
        src.to_str().unwrap(),
        "-o",
        dir.path().join("y.mp4").to_str().unwrap(),
        "--every",
        "2",
        "--at",
        "3",
    ]);
    assert_eq!(v["status"], "failed", "{v}");
}

#[test]
fn volume_at_mutes_only_the_window() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let out = dir.path().join("v.mp4");
    // Crush the second half.
    let v = run_json(&[
        "volume",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--db",
        "-24",
        "--at",
        "0.5",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let mean_at = |ss: &str, dur: &str| -> f64 {
        let o = Command::new("ffmpeg")
            .args(["-ss", ss, "-t", dur, "-i"])
            .arg(&out)
            .args(["-af", "volumedetect", "-vn", "-f", "null", "-"])
            .output()
            .unwrap();
        let s = String::from_utf8_lossy(&o.stderr);
        s.lines()
            .find_map(|l| {
                l.split("mean_volume:")
                    .nth(1)
                    .and_then(|r| r.split_whitespace().next())
                    .and_then(|x| x.parse::<f64>().ok())
            })
            .unwrap_or(0.0)
    };
    let first = mean_at("0", "0.5");
    let last = mean_at("0.55", "0.4");
    assert!(
        last < first - 10.0,
        "windowed gain should hit only the tail ({first} -> {last}); {v}"
    );
    // --dur without --at is refused.
    let v = run_json(&[
        "volume",
        src.to_str().unwrap(),
        "-o",
        dir.path().join("w.mp4").to_str().unwrap(),
        "--db",
        "-6",
        "--dur",
        "1",
    ]);
    assert_eq!(v["status"], "failed", "{v}");
}

#[test]
fn progress_bar_fills_over_duration() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let out = dir.path().join("p.mp4");
    let v = run_json(&[
        "progress",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let bar = |n: u32| -> f64 {
        let o = Command::new("ffmpeg")
            .args(["-i"])
            .arg(&out)
            .args([
                "-vf",
                &format!("select=eq(n\\,{n}),crop=320:4:0:236"),
                "-frames:v",
                "1",
                "-f",
                "rawvideo",
                "-pix_fmt",
                "rgb24",
                "-",
            ])
            .output()
            .unwrap();
        assert!(o.status.success());
        o.stdout
            .chunks(3)
            .filter(|c| c.len() == 3 && c.iter().all(|b| *b > 200))
            .count() as f64
            / 320.0
            / 4.0
    };
    let early = bar(3);
    let late = bar(28);
    assert!(early < 0.5, "bar barely filled at 0.1s ({early}); {v}");
    assert!(late > 0.8, "bar nearly full at 0.95s ({late}); {v}");
}

#[test]
fn grid_stacks_four_tiles() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let mut inputs = Vec::new();
    for (i, c) in ["red", "green", "blue", "yellow"].iter().enumerate() {
        let f = dir.path().join(format!("g{i}.mp4"));
        let status = Command::new("ffmpeg")
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                &format!("color=c={c}:duration=1:size=320x240:rate=30"),
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
        inputs.push(f);
    }
    let out = dir.path().join("grid.mp4");
    let v = run_json(&[
        "grid",
        inputs[0].to_str().unwrap(),
        inputs[1].to_str().unwrap(),
        inputs[2].to_str().unwrap(),
        inputs[3].to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--size",
        "640x480",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    assert_eq!(v["probe"]["width"].as_u64().unwrap(), 640);
    assert_eq!(v["probe"]["height"].as_u64().unwrap(), 480);
    // 3 inputs into a 2x2 is fine; 5 is refused.
    let v = run_json(&[
        "grid",
        inputs[0].to_str().unwrap(),
        inputs[1].to_str().unwrap(),
        inputs[2].to_str().unwrap(),
        inputs[3].to_str().unwrap(),
        src.to_str().unwrap(),
        "-o",
        dir.path().join("too.mp4").to_str().unwrap(),
        "--size",
        "640x480",
    ]);
    assert_eq!(v["status"], "failed", "{v}");
}

#[test]
fn freeze_end_pads_last_frame() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let out = dir.path().join("fr.mp4");
    let v = run_json(&[
        "freeze",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--end",
        "1.5",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let d = v["extra"]["probe"]["duration"].as_f64().unwrap();
    assert!(
        (d - 2.5).abs() < 0.3,
        "1s clip + 1.5s freeze = ~2.5s, got {d}"
    );
    // frozen tail: frame at t≈2.0 identical to last source frame
    let px = |n: u32| -> Vec<u8> {
        Command::new("ffmpeg")
            .args(["-i"])
            .arg(&out)
            .args([
                "-vf",
                &format!("select=eq(n\\,{n})"),
                "-frames:v",
                "1",
                "-f",
                "rawvideo",
                "-pix_fmt",
                "rgb24",
                "-",
            ])
            .output()
            .unwrap()
            .stdout
    };
    let a = px(29); // last source frame (~0.97s)
    let b = px(59); // deep in the frozen tail (~1.97s)
    assert!(!a.is_empty() && !b.is_empty());
    let diff: u64 = a
        .iter()
        .zip(b.iter())
        .map(|(x, y)| x.abs_diff(*y) as u64)
        .sum();
    assert!(
        diff < a.len() as u64,
        "frozen frames should be near-identical (diff {diff} on {} bytes)",
        a.len()
    );
}

#[test]
fn censor_pixelizes_the_region() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let out = dir.path().join("c.mp4");
    let v = run_json(&[
        "censor",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--region",
        "100:100:64:64",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    // inside the box the mosaic changes pixels vs the source; outside stays.
    let region = |f: &Path| -> Vec<u8> {
        Command::new("ffmpeg")
            .args(["-i"])
            .arg(f)
            .args([
                "-vf",
                "crop=64:64:100:100",
                "-frames:v",
                "1",
                "-f",
                "rawvideo",
                "-pix_fmt",
                "gray",
                "-",
            ])
            .output()
            .unwrap()
            .stdout
    };
    let src_px = region(&src);
    let out_px = region(&out);
    let diff: u64 = src_px
        .iter()
        .zip(out_px.iter())
        .map(|(x, y)| x.abs_diff(*y) as u64)
        .sum();
    assert!(
        diff > 5000,
        "mosaic must alter the region (diff {diff}); {v}"
    );
    let v = run_json(&[
        "censor",
        src.to_str().unwrap(),
        "-o",
        dir.path().join("x.mp4").to_str().unwrap(),
        "--region",
        "500:500:64:64",
    ]);
    assert_eq!(v["status"], "failed", "out-of-frame region refused; {v}");
}

#[test]
fn speed_at_ramps_only_the_window() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let out = dir.path().join("s.mp4");
    let v = run_json(&[
        "speed",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--factor",
        "2",
        "--at",
        "0.4",
        "--dur",
        "0.2",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let p = run_json(&["probe", out.to_str().unwrap()]);
    let d = p["probe"]["duration"]
        .as_f64()
        .unwrap_or_else(|| p["extra"]["probe"]["duration"].as_f64().unwrap_or(0.0));
    assert!(
        (d - 0.9).abs() < 0.15,
        "1s with a 0.2s window at 2x = 0.9s, got {d}; {v}"
    );
}

#[test]
fn boomerang_doubles_and_last_frame_matches_first() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let out = dir.path().join("b.mp4");
    let v = run_json(&[
        "boomerang",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let d = v["extra"]["probe"]["duration"].as_f64().unwrap();
    assert!((d - 2.0).abs() < 0.3, "1s fwd + 1s rev = ~2s, got {d}; {v}");
    let px = |n: u32| -> Vec<u8> {
        Command::new("ffmpeg")
            .args(["-i"])
            .arg(&out)
            .args([
                "-vf",
                &format!("select=eq(n\\,{n})"),
                "-frames:v",
                "1",
                "-f",
                "rawvideo",
                "-pix_fmt",
                "rgb24",
                "-",
            ])
            .output()
            .unwrap()
            .stdout
    };
    let a = px(0);
    let b = px(59); // last frame of 60 == reverse's last frame == source frame 0
    assert!(!a.is_empty() && !b.is_empty());
    let diff: u64 = a
        .iter()
        .zip(b.iter())
        .map(|(x, y)| x.abs_diff(*y) as u64)
        .sum();
    assert!(
        diff < a.len() as u64,
        "boomerang ends where it starts (diff {diff} on {})",
        a.len()
    );
}

#[test]
fn chapter_embeds_named_marks() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let out = dir.path().join("ch.mp4");
    let v = run_json(&[
        "chapter",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--at",
        "0|Intro",
        "--at",
        "0.5|Middle",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let o = Command::new("ffprobe")
        .args(["-v", "error", "-show_chapters", "-of", "csv"])
        .arg(&out)
        .output()
        .unwrap();
    let s = String::from_utf8_lossy(&o.stdout);
    assert!(s.contains("Intro") && s.contains("Middle"), "chapters: {s}");
    // bad entry refused
    let v = run_json(&[
        "chapter",
        src.to_str().unwrap(),
        "-o",
        dir.path().join("x.mp4").to_str().unwrap(),
        "--at",
        "no title separator",
    ]);
    assert_eq!(v["status"], "failed", "{v}");
}

#[test]
fn zoom_at_punches_only_the_window() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let out = dir.path().join("z.mp4");
    let v = run_json(&[
        "zoom",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--factor",
        "1.5",
        "--at",
        "0.4",
        "--dur",
        "0.3",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let d = v["probe"]["duration"]
        .as_f64()
        .unwrap_or_else(|| v["extra"]["probe"]["duration"].as_f64().unwrap_or(0.0));
    assert!(
        (d - 1.0).abs() < 0.15,
        "windowed zoom keeps duration, got {d}; {v}"
    );
    // frame inside the window differs from source; frame outside matches
    let px = |f: &Path, n: u32| -> Vec<u8> {
        Command::new("ffmpeg")
            .args(["-i"])
            .arg(f)
            .args([
                "-vf",
                &format!("select=eq(n\\,{n}),crop=64:64:128:88"),
                "-frames:v",
                "1",
                "-f",
                "rawvideo",
                "-pix_fmt",
                "rgb24",
                "-",
            ])
            .output()
            .unwrap()
            .stdout
    };
    let inside_src = px(&src, 15); // ~0.5s inside window
    let inside_out = px(&out, 15);
    let diff: u64 = inside_src
        .iter()
        .zip(inside_out.iter())
        .map(|(x, y)| x.abs_diff(*y) as u64)
        .sum();
    assert!(diff > 500, "zoomed center must differ inside window; {v}");
}

#[test]
fn key_despill_keeps_composite_working() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let bg = dir.path().join("bg.png");
    let ok = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "color=c=0x0000ff:size=640x480",
            "-frames:v",
            "1",
        ])
        .arg(&bg)
        .status()
        .unwrap()
        .success();
    assert!(ok);
    let out = dir.path().join("k.mp4");
    let v = run_json(&[
        "key",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--bg",
        bg.to_str().unwrap(),
        "--color",
        "0xff7f00",
        "--despill",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
}

#[test]
fn autocrop_strips_letterbox() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let padded = dir.path().join("padded.mp4");
    let ok = Command::new("ffmpeg")
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
            "-vf",
            "pad=320:320:0:40:color=black",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            "-shortest",
        ])
        .arg(&padded)
        .status()
        .unwrap()
        .success();
    assert!(ok);
    let out = dir.path().join("cropped.mp4");
    let v = run_json(&[
        "autocrop",
        padded.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let d = v["extra"]["detected"].as_object().unwrap();
    let h = d["h"].as_u64().unwrap();
    assert!(
        h <= 242,
        "letterboxed 240 of 320 rows should be detected, got {h}"
    );
}

#[test]
fn sheet_tiles_the_clip() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let out = dir.path().join("sheet.png");
    let v = run_json(&[
        "sheet",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--cols",
        "2",
        "--rows",
        "2",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    // sheet output is a single PNG with 4 tile regions differing in pixels
    let o = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height",
            "-of",
            "csv=p=0",
        ])
        .arg(&out)
        .output()
        .unwrap();
    let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
    let (w, h) = s
        .split_once(',')
        .map(|(a, b)| (a.parse::<u32>().unwrap_or(0), b.parse::<u32>().unwrap_or(0)))
        .unwrap_or((0, 0));
    assert!(
        w >= 640 && h >= 480,
        "2x2 @ 320 tiles+padding ≥ 640x480, got {w}x{h}"
    );
}

#[test]
fn title_at_shows_mid_clip() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let out = dir.path().join("t.mp4");
    let v = run_json(&[
        "title",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--text",
        "MID",
        "--at",
        "0.4",
        "--duration",
        "0.4",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    assert!(
        (v["extra"]["at"].as_f64().unwrap() - 0.4).abs() < 1e-6,
        "{v}"
    );
}

#[test]
fn replace_duck_dips_original_under_voice() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    // video with a loud bed; replacement = quiet tone
    let src = dir.path().join("bed.mp4");
    let voice = dir.path().join("voice.wav");
    for (args, path) in [
        (
            vec![
                "-f",
                "lavfi",
                "-i",
                "testsrc=duration=2:size=160x120:rate=15",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=220:duration=2",
            ],
            &src,
        ),
        (
            vec!["-f", "lavfi", "-i", "sine=frequency=880:duration=2"],
            &voice,
        ),
    ] {
        let mut a = vec!["-hide_banner", "-loglevel", "error", "-y"];
        a.extend(args.iter().copied());
        a.extend([
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            "-shortest",
        ]);
        if path == &voice {
            a = vec!["-hide_banner", "-loglevel", "error", "-y"];
            a.extend(args.iter().copied());
        }
        let st = Command::new("ffmpeg").args(&a).arg(path).status().unwrap();
        assert!(st.success());
    }
    let out = dir.path().join("ducked.mp4");
    let v = run_json(&[
        "replace",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--audio",
        voice.to_str().unwrap(),
        "--mix",
        "1.0",
        "--duck",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    // both streams mixed: second has the 880Hz voice AND (compressed) 220Hz bed
    let o = Command::new("ffmpeg")
        .args(["-i"])
        .arg(&out)
        .args(["-af", "volumedetect", "-f", "null", "-"])
        .output()
        .unwrap();
    let s = String::from_utf8_lossy(&o.stderr);
    let mean: f64 = s
        .split("mean_volume:")
        .nth(1)
        .and_then(|r| r.split_whitespace().next())
        .and_then(|r| r.trim_end_matches("dB").parse().ok())
        .unwrap_or(-99.0);
    assert!(
        mean > -60.0,
        "mixed output should carry audio, mean {mean}dB; {v}"
    );
}

#[test]
fn pitch_semitones_shift_keeps_duration() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let out = dir.path().join("p.m4a");
    let v = run_json(&[
        "pitch",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--semitones",
        "7",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let d = v["probe"]["duration"].as_f64().unwrap_or(0.0);
    assert!(
        (d - 1.0).abs() < 0.2,
        "pitch keeps duration ~1s, got {d}; {v}"
    );
    // 440Hz -> ~659Hz at +7st: zero-crossing count should rise ~1.5x
    let zc = |f: &Path| -> f64 {
        let o = Command::new("ffmpeg")
            .args(["-i"])
            .arg(f)
            .args(["-af", "astats", "-f", "null", "-"])
            .output()
            .unwrap();
        let s = String::from_utf8_lossy(&o.stderr);
        s.lines()
            .filter(|l| l.contains("Zero crossings:"))
            .filter_map(|l| l.rsplit(':').next())
            .filter_map(|v| v.trim().parse::<f64>().ok())
            .fold(0.0, f64::max)
    };
    let (a, b) = (zc(&src), zc(&out));
    assert!(b > a * 1.3, "zero crossings should rise: {a} -> {b}");
}

#[test]
fn grade_grain_adds_noise() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let flat = dir.path().join("flat.mp4");
    let ok = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "color=c=0x808080:size=160x120:rate=15:duration=1",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            "-crf",
            "23",
        ])
        .arg(&flat)
        .status()
        .unwrap()
        .success();
    assert!(ok);
    let out = dir.path().join("g.mp4");
    let v = run_json(&[
        "grade",
        flat.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--grain",
        "12",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let variance = |f: &Path| -> f64 {
        let o = Command::new("ffmpeg")
            .args(["-i"])
            .arg(f)
            .args([
                "-vf",
                "crop=80:60:40:30,signalstats,metadata=print:key=lavfi.signalstats.YMIN:file=-",
                "-f",
                "null",
                "-",
            ])
            .output()
            .unwrap();
        let s = String::from_utf8_lossy(&o.stdout);
        s.lines()
            .filter_map(|l| l.split("YMIN=").nth(1))
            .filter_map(|v| v.trim().parse::<f64>().ok())
            .fold(f64::NEG_INFINITY, f64::max)
    };
    // flat gray: YMIN stays ~128 without grain, dips with it
    let (a, b) = (variance(&flat), variance(&out));
    assert!(
        b < a - 4.0,
        "grain should push dark pixels in flat area: {a} -> {b}"
    );
}

#[test]
fn split_scenes_cuts_at_shot_change() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    // red 0.5s -> blue 0.5s -> green 0.5s = 2 hard scene cuts
    let src = dir.path().join("shots.mp4");
    let ok = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "color=c=red:size=160x120:rate=20:duration=0.5",
            "-f",
            "lavfi",
            "-i",
            "color=c=blue:size=160x120:rate=20:duration=0.5",
            "-f",
            "lavfi",
            "-i",
            "color=c=green:size=160x120:rate=20:duration=0.5",
            "-filter_complex",
            "[0:v][1:v][2:v]concat=n=3:v=1[v]",
            "-map",
            "[v]",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            "-g",
            "20",
        ])
        .arg(&src)
        .status()
        .unwrap()
        .success();
    assert!(ok);
    let v = run_json(&[
        "split",
        src.to_str().unwrap(),
        "-o",
        dir.path().join("shot_%02d.mp4").to_str().unwrap(),
        "--scenes",
        "0.3",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let count = v["extra"]["count"].as_u64().unwrap_or(0);
    assert!(
        count >= 2,
        "two color cuts should split into >=2 parts, got {count}; {v}"
    );
}

#[test]
fn grid_audio_follows_one_input() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let mk = |name: &str, color: &str, hz: u32| -> PathBuf {
        let p = dir.path().join(name);
        let st = Command::new("ffmpeg")
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                &format!("color=c={color}:size=160x120:rate=15:duration=1"),
                "-f",
                "lavfi",
                "-i",
                &format!("sine=frequency={hz}:duration=1"),
                "-c:v",
                "libx264",
                "-pix_fmt",
                "yuv420p",
                "-c:a",
                "aac",
                "-shortest",
            ])
            .arg(&p)
            .status()
            .unwrap();
        assert!(st.success());
        p
    };
    let a = mk("a.mp4", "red", 220);
    let b = mk("b.mp4", "blue", 440);
    let c = mk("c.mp4", "green", 660);
    let d = mk("d.mp4", "yellow", 880);
    let out = dir.path().join("g.mp4");
    let v = run_json(&[
        "grid",
        a.to_str().unwrap(),
        b.to_str().unwrap(),
        c.to_str().unwrap(),
        d.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--audio",
        "2",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    // output audio should be the 660Hz sine only — zero-crossings ~1320/s
    let o = Command::new("ffmpeg")
        .args(["-i"])
        .arg(&out)
        .args(["-af", "astats", "-f", "null", "-"])
        .output()
        .unwrap();
    let s = String::from_utf8_lossy(&o.stderr);
    let zc: f64 = s
        .lines()
        .filter(|l| l.contains("Zero crossings:"))
        .filter_map(|l| l.rsplit(':').next())
        .filter_map(|v| v.trim().parse::<f64>().ok())
        .fold(0.0, f64::max);
    assert!(
        zc > 1000.0 && zc < 1600.0,
        "audio from input 2 = 660Hz ~1320 zc, got {zc}; {v}"
    );
}

#[test]
fn cutsil_strips_head_and_tail_silence() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    // 0.5s silence + 1s tone + 0.5s silence
    let src = dir.path().join("pad.wav");
    let ok = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "anullsrc=r=48000:cl=stereo:d=0.5",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=1",
            "-f",
            "lavfi",
            "-i",
            "anullsrc=r=48000:cl=stereo:d=0.5",
            "-filter_complex",
            "[0:a][1:a][2:a]concat=n=3:v=0:a=1[a]",
            "-map",
            "[a]",
        ])
        .arg(&src)
        .status()
        .unwrap()
        .success();
    assert!(ok);
    let out = dir.path().join("cut.m4a");
    let v = run_json(&["cutsil", src.to_str().unwrap(), "-o", out.to_str().unwrap()]);
    assert_eq!(v["status"], "ok", "{v}");
    let d = v["probe"]["duration"].as_f64().unwrap_or(0.0);
    assert!(
        d > 0.5 && d < 1.5,
        "padded 2s -> ~1s after both-end strip, got {d}; {v}"
    );
}

#[test]
fn overlay_tile_repeats_watermark() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let logo = dir.path().join("logo.png");
    let ok = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "color=c=white:size=40x40",
            "-frames:v",
            "1",
        ])
        .arg(&logo)
        .status()
        .unwrap()
        .success();
    assert!(ok);
    let out = dir.path().join("t.mp4");
    let v = run_json(&[
        "overlay",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--image",
        logo.to_str().unwrap(),
        "--scale",
        "40",
        "--tile",
        "4",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    assert_eq!(v["extra"]["tile"].as_u64(), Some(4), "{v}");
    // four white copies on a testsrc frame: two opposite corners differ from source
    let px = |f: &Path| -> Vec<u8> {
        Command::new("ffmpeg")
            .args(["-i"])
            .arg(f)
            .args([
                "-vf",
                "select=eq(n\\,15)",
                "-frames:v",
                "1",
                "-f",
                "rawvideo",
                "-pix_fmt",
                "rgb24",
                "-",
            ])
            .output()
            .unwrap()
            .stdout
    };
    let a = px(&src);
    let b = px(&out);
    let diff: u64 = a
        .iter()
        .zip(b.iter())
        .map(|(x, y)| x.abs_diff(*y) as u64)
        .sum();
    assert!(diff > 5000, "4 white tiles must change pixels; {v}");
}

#[test]
fn caption_shift_moves_cue_timing() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let srt = dir.path().join("c.srt");
    std::fs::write(&srt, "1\n00:00:00,100 --> 00:00:00,400\nHI\n").unwrap();
    let out = dir.path().join("c.mp4");
    let v = run_json(&[
        "caption",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--srt",
        srt.to_str().unwrap(),
        "--mode",
        "burn",
        "--safe",
        "off",
        "--shift",
        "0.4",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    // cue now lives at 0.5-0.8s: frame 5 (0.17s) == source; frame 18 (0.6s) differs
    let px = |f: &Path, n: u32| -> Vec<u8> {
        Command::new("ffmpeg")
            .args(["-i"])
            .arg(f)
            .args([
                "-vf",
                &format!("select=eq(n\\,{n})"),
                "-frames:v",
                "1",
                "-f",
                "rawvideo",
                "-pix_fmt",
                "rgb24",
                "-",
            ])
            .output()
            .unwrap()
            .stdout
    };
    let d = |n: u32| -> u64 {
        let a = px(&src, n);
        let b = px(&out, n);
        a.iter()
            .zip(b.iter())
            .map(|(x, y)| x.abs_diff(*y) as u64)
            .sum()
    };
    let pre = d(5);
    let inside = d(18);
    assert!(
        inside > pre + 2000,
        "caption must appear after shift: pre {pre} vs inside {inside}; {v}"
    );
}

#[test]
fn transcode_gif_honors_fps_and_width() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let out = dir.path().join("g.gif");
    let v = run_json(&[
        "transcode",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--preset",
        "gif",
        "--fps",
        "5",
        "--width",
        "160",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let o = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,avg_frame_rate",
            "-of",
            "csv=p=0",
        ])
        .arg(&out)
        .output()
        .unwrap();
    let s = String::from_utf8_lossy(&o.stdout);
    assert!(s.contains("160"), "gif width 160, got {s}");
    assert!(s.contains("5/1"), "gif fps 5, got {s}");
}

#[test]
fn channel_dualmono_fills_both_ears() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    // mono voice file → dualmono → 2 channels
    let src = dir.path().join("mono.wav");
    let ok = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=1",
            "-ac",
            "1",
        ])
        .arg(&src)
        .status()
        .unwrap()
        .success();
    assert!(ok);
    let out = dir.path().join("stereo.m4a");
    let v = run_json(&[
        "channel",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--mode",
        "dualmono",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let ch = v["probe"]["channels"].as_u64().unwrap_or(0);
    assert_eq!(ch, 2, "dualmono → stereo, got {ch}; {v}");
}

#[test]
fn channel_swap_flips_left_right() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    // left-ear only sine: pan stereo c0→FL, silence FR
    let src = dir.path().join("left.wav");
    let ok = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=1",
            "-af",
            "pan=stereo|c0=c0|c1=0*c0",
        ])
        .arg(&src)
        .status()
        .unwrap()
        .success();
    assert!(ok);
    let out = dir.path().join("swapped.m4a");
    let v = run_json(&[
        "channel",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--mode",
        "swap",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    // measure per-channel RMS via pan extract of each ear
    let rms = |f: &Path, sel: &str| -> f64 {
        let o = Command::new("ffmpeg")
            .args(["-i"])
            .arg(f)
            .args([
                "-af",
                &format!("pan=mono|c0={sel},volumedetect"),
                "-f",
                "null",
                "-",
            ])
            .output()
            .unwrap();
        let s = String::from_utf8_lossy(&o.stderr);
        s.split("mean_volume:")
            .nth(1)
            .and_then(|r| r.split_whitespace().next())
            .and_then(|r| r.trim_end_matches("dB").parse().ok())
            .unwrap_or(-99.0)
    };
    let (l0, r0) = (rms(&src, "c0"), rms(&src, "c1"));
    let (l1, r1) = (rms(&out, "c0"), rms(&out, "c1"));
    assert!(
        r1 > r0 + 15.0 && l1 < l0 - 15.0,
        "swap must move the ear: ({l0:.1},{r0:.1}) -> ({l1:.1},{r1:.1}); {v}"
    );
}

#[test]
fn loop_until_hits_target_duration() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path()); // 1s
    let out = dir.path().join("l.mp4");
    let v = run_json(&[
        "loop",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--until",
        "2.5",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let d = v["probe"]["duration"].as_f64().unwrap_or(0.0);
    assert!(
        (d - 2.5).abs() < 0.35,
        "loop --until 2.5 → ~2.5s, got {d}; {v}"
    );
    assert!(v["extra"]["times"].as_u64().unwrap_or(0) >= 3, "{v}");
}

#[test]
fn title_tile_stamps_multiple_copies() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let out = dir.path().join("tw.mp4");
    let v = run_json(&[
        "title",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--text",
        "DRAFT",
        "--tile",
        "4",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    assert_eq!(
        v["extra"]["position"].as_str().unwrap_or(""),
        "tile-4",
        "{v}"
    );
}

#[test]
fn eq_bass_raises_low_energy() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("sine.wav");
    let ok = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=110:duration=1",
        ])
        .arg(&src)
        .status()
        .unwrap()
        .success();
    assert!(ok);
    let out = dir.path().join("bass.m4a");
    let v = run_json(&[
        "eq",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--bass",
        "8",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let mean = |f: &Path| -> f64 {
        let o = Command::new("ffmpeg")
            .args(["-i"])
            .arg(f)
            .args(["-af", "volumedetect", "-f", "null", "-"])
            .output()
            .unwrap();
        let s = String::from_utf8_lossy(&o.stderr);
        s.split("mean_volume:")
            .nth(1)
            .and_then(|r| r.split_whitespace().next())
            .and_then(|r| r.trim_end_matches("dB").parse().ok())
            .unwrap_or(-99.0)
    };
    let (a, b) = (mean(&src), mean(&out));
    assert!(
        b > a + 3.0,
        "+8dB bass on 110Hz should raise level: {a} -> {b}; {v}"
    );
}

#[test]
fn zoom_motion_kenburns_pushes_over_time() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let out = dir.path().join("z.mp4");
    let v = run_json(&[
        "zoom",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--factor",
        "1.4",
        "--motion",
        "kenburns",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    // early frame ≈ unzoomed, late frame zoomed: diff vs source grows
    let px = |f: &Path, n: u32| -> Vec<u8> {
        Command::new("ffmpeg")
            .args(["-i"])
            .arg(f)
            .args([
                "-vf",
                &format!("select=eq(n\\,{n}),crop=64:64:128:88"),
                "-frames:v",
                "1",
                "-f",
                "rawvideo",
                "-pix_fmt",
                "rgb24",
                "-",
            ])
            .output()
            .unwrap()
            .stdout
    };
    let d = |n: u32| -> u64 {
        let a = px(&src, n);
        let b = px(&out, n);
        a.iter()
            .zip(b.iter())
            .map(|(x, y)| x.abs_diff(*y) as u64)
            .sum()
    };
    let (e, l) = (d(3), d(27));
    assert!(
        l > e + 300,
        "kenburns diff grows over clip: {e} -> {l}; {v}"
    );
}

#[test]
fn broll_still_inserts_image() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let still = dir.path().join("s.png");
    let ok = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "color=c=0xff0000:size=320x240",
            "-frames:v",
            "1",
        ])
        .arg(&still)
        .status()
        .unwrap()
        .success();
    assert!(ok);
    let out = dir.path().join("b.mp4");
    let v = run_json(&[
        "broll",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--insert",
        still.to_str().unwrap(),
        "--at",
        "0.4",
        "--duration",
        "0.4",
        "--still",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    // inside the window the frame is red
    let o = Command::new("ffmpeg")
        .args(["-i"])
        .arg(&out)
        .args([
            "-vf",
            "select=eq(n\\,18),signalstats,metadata=print:key=lavfi.signalstats.YAVG:file=-",
            "-frames:v",
            "1",
            "-f",
            "null",
            "-",
        ])
        .output()
        .unwrap();
    let s = String::from_utf8_lossy(&o.stdout);
    let y: f64 = s
        .split("YAVG=")
        .nth(1)
        .and_then(|r| r.trim().parse().ok())
        .unwrap_or(0.0);
    // red (0xff0000) in bt601 Y ≈ 76; testsrc center isn't uniformly red
    assert!(
        (60.0..95.0).contains(&y),
        "red still fills the window, YAVG {y}; {v}"
    );
}

#[test]
fn rotate_swaps_dimensions() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let out = dir.path().join("r.mp4");
    let v = run_json(&[
        "rotate",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--deg",
        "90",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let o = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height",
            "-of",
            "csv=p=0",
        ])
        .arg(&out)
        .output()
        .unwrap();
    let s = String::from_utf8_lossy(&o.stdout);
    assert_eq!(
        s.trim(),
        "240,320",
        "90deg should swap 320x240 -> 240x320; {v}"
    );
}

#[test]
fn delogo_blends_out_box() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    // Burn a solid white box at 10,10 60x40
    let marked = dir.path().join("marked.mp4");
    let ok = Command::new("ffmpeg")
        .args(["-hide_banner", "-loglevel", "error", "-y", "-i"])
        .arg(&src)
        .args([
            "-vf",
            "drawbox=x=10:y=10:w=60:h=40:color=white:t=fill",
            "-c:v",
            "libx264",
            "-preset",
            "fast",
            "-crf",
            "18",
            "-c:a",
            "copy",
        ])
        .arg(&marked)
        .status()
        .unwrap()
        .success();
    assert!(ok);
    let out = dir.path().join("d.mp4");
    let v = run_json(&[
        "delogo",
        marked.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--x",
        "10",
        "--y",
        "10",
        "--w",
        "60",
        "--h",
        "40",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let mean_luma = |f: &Path| -> f64 {
        let o = Command::new("ffmpeg")
            .args(["-i"])
            .arg(f)
            .args([
                "-vf",
                "crop=40:20:20:20,signalstats,metadata=print:key=lavfi.signalstats.YAVG:file=-",
                "-frames:v",
                "1",
                "-f",
                "null",
                "-",
            ])
            .output()
            .unwrap();
        let s = String::from_utf8_lossy(&o.stdout);
        s.split("YAVG=")
            .nth(1)
            .and_then(|r| r.lines().next())
            .and_then(|r| r.trim().parse().ok())
            .unwrap_or(-1.0)
    };
    let (a, b) = (mean_luma(&marked), mean_luma(&out));
    assert!(
        b < a - 30.0,
        "logo box luma drops after delogo: {a} -> {b}; {v}"
    );
}

#[test]
fn speed_interp_adds_frames() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let plain = dir.path().join("p.mp4");
    let interp = dir.path().join("i.mp4");
    let v1 = run_json(&[
        "speed",
        src.to_str().unwrap(),
        "-o",
        plain.to_str().unwrap(),
        "--factor",
        "0.5",
    ]);
    assert_eq!(v1["status"], "ok", "{v1}");
    let v2 = run_json(&[
        "speed",
        src.to_str().unwrap(),
        "-o",
        interp.to_str().unwrap(),
        "--factor",
        "0.5",
        "--interp",
    ]);
    assert_eq!(v2["status"], "ok", "{v2}");
    let frames = |f: &Path| -> u64 {
        let o = Command::new("ffprobe")
            .args([
                "-v",
                "error",
                "-select_streams",
                "v:0",
                "-count_packets",
                "-show_entries",
                "stream=nb_read_packets",
                "-of",
                "csv=p=0",
            ])
            .arg(f)
            .output()
            .unwrap();
        String::from_utf8_lossy(&o.stdout)
            .trim()
            .parse()
            .unwrap_or(0)
    };
    let (a, b) = (frames(&plain), frames(&interp));
    assert!(b > a + 10, "interp should add frames: {a} -> {b}; {v2}");
}

#[test]
fn title_color_and_size_render() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    // red title, 2x size
    let out = dir.path().join("t.mp4");
    let v = run_json(&[
        "title",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--text",
        "HI",
        "--duration",
        "0.5",
        "--color",
        "ff0000",
        "--size",
        "2",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    // R channel should dominate in center rows where the title sits
    let o = Command::new("ffmpeg")
        .args(["-i"])
        .arg(&out)
        .args([
            "-vf",
            "select=eq(n\\,5),crop=200:80:60:80",
            "-frames:v",
            "1",
            "-f",
            "rawvideo",
            "-pix_fmt",
            "rgb24",
            "-",
        ])
        .output()
        .unwrap();
    let px = o.stdout;
    let mut r = 0u64;
    let mut b = 0u64;
    for c in px.chunks(3) {
        r += c[0] as u64;
        b += c[2] as u64;
    }
    assert!(r > b, "red title should dominate: R {r} vs B {b}; {v}");
}

#[test]
fn meta_writes_title_tag() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let out = dir.path().join("m.mp4");
    let v = run_json(&[
        "meta",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--title",
        "Episode 12",
        "--artist",
        "Pod Team",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let o = Command::new("ffprobe")
        .args(["-v", "error", "-show_entries", "format_tags", "-of", "json"])
        .arg(&out)
        .output()
        .unwrap();
    let s = String::from_utf8_lossy(&o.stdout);
    assert!(s.contains("Episode 12"), "title tag written: {s}");
    assert!(s.contains("Pod Team"), "artist tag written: {s}");
    // lossless copy: same duration
    assert!(
        (v["probe"]["duration"].as_f64().unwrap_or(0.0) - 1.0).abs() < 0.05,
        "{v}"
    );
}

#[test]
fn broll_still_motion_keeps_window() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let still = dir.path().join("s.png");
    let ok = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "color=c=0x00ff00:size=320x240",
            "-frames:v",
            "1",
        ])
        .arg(&still)
        .status()
        .unwrap()
        .success();
    assert!(ok);
    let out = dir.path().join("b.mp4");
    let v = run_json(&[
        "broll",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--insert",
        still.to_str().unwrap(),
        "--at",
        "0.3",
        "--duration",
        "0.5",
        "--still",
        "--motion",
        "kenburns",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    // window shows the green still, and duration stays at A-roll's 1s
    assert!(
        (v["probe"]["duration"].as_f64().unwrap_or(0.0) - 1.0).abs() < 0.2,
        "{v}"
    );
}

#[test]
fn overlay_windowed_shows_only_inside() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    // black 40x40 logo on the bright top-right gradient
    let logo = dir.path().join("logo.png");
    let ok = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "color=c=0x000000:size=40x40",
            "-frames:v",
            "1",
        ])
        .arg(&logo)
        .status()
        .unwrap()
        .success();
    assert!(ok);
    let out = dir.path().join("o.mp4");
    let v = run_json(&[
        "overlay",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--image",
        logo.to_str().unwrap(),
        "--position",
        "top-right",
        "--at",
        "0.5",
        "--dur",
        "0.4",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    // frame 3 (t=0.1) top-right corner should NOT be white; frame 20 (t=0.67) should be
    let corner = |n: u32| -> f64 {
        let o = Command::new("ffmpeg")
            .args(["-i"]).arg(&out)
            .args([
                "-vf",
                &format!("select=eq(n\\,{n}),crop=30:30:285:5,signalstats,metadata=print:key=lavfi.signalstats.YAVG:file=-"),
                "-frames:v", "1", "-f", "null", "-",
            ])
            .output()
            .unwrap();
        String::from_utf8_lossy(&o.stdout)
            .split("YAVG=")
            .nth(1)
            .and_then(|r| r.lines().next())
            .and_then(|r| r.trim().parse().ok())
            .unwrap_or(-1.0)
    };
    let (before, inside) = (corner(3), corner(20));
    assert!(
        before > inside + 40.0,
        "black logo drops luma only inside window: {before} -> {inside}; {v}"
    );
}

#[test]
fn caption_color_burns_red() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let srt = dir.path().join("c.srt");
    std::fs::write(&srt, "1\n00:00:00,000 --> 00:00:01,000\nRED TEXT\n").unwrap();
    let out = dir.path().join("c.mp4");
    let v = run_json(&[
        "caption",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--srt",
        srt.to_str().unwrap(),
        "--mode",
        "burn",
        "--color",
        "ff0000",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let white = dir.path().join("w.mp4");
    let v2 = run_json(&[
        "caption",
        src.to_str().unwrap(),
        "-o",
        white.to_str().unwrap(),
        "--srt",
        srt.to_str().unwrap(),
        "--mode",
        "burn",
    ]);
    assert_eq!(v2["status"], "ok", "{v2}");
    // red text drops the B channel vs the white default on the same strip
    let blue_energy = |f: &Path| -> u64 {
        let o = Command::new("ffmpeg")
            .args(["-i"])
            .arg(f)
            .args([
                "-vf",
                "select=eq(n\\,5),crop=300:80:10:140",
                "-frames:v",
                "1",
                "-f",
                "rawvideo",
                "-pix_fmt",
                "rgb24",
                "-",
            ])
            .output()
            .unwrap();
        o.stdout.chunks(3).map(|c| c[2] as u64).sum()
    };
    let (b_red, b_white) = (blue_energy(&out), blue_energy(&white));
    assert!(
        b_red + 3000 < b_white,
        "red caption drops B energy: {b_white} -> {b_red}; {v}"
    );
}

#[test]
fn subs_extracts_embedded() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    // mkv with embedded srt
    let srt = dir.path().join("in.srt");
    std::fs::write(&srt, "1\n00:00:00,000 --> 00:00:00,800\nhello subs\n").unwrap();
    let mkv = dir.path().join("with.mkv");
    let ok = Command::new("ffmpeg")
        .args(["-hide_banner", "-loglevel", "error", "-y", "-i"])
        .arg(&src)
        .args(["-i"])
        .arg(&srt)
        .args([
            "-map", "0", "-map", "1", "-c:v", "copy", "-c:a", "copy", "-c:s", "srt",
        ])
        .arg(&mkv)
        .status()
        .unwrap()
        .success();
    assert!(ok, "fixture mkv w/ subs");
    let out = dir.path().join("out.srt");
    let v = run_json(&["subs", mkv.to_str().unwrap(), "-o", out.to_str().unwrap()]);
    assert_eq!(v["status"], "ok", "{v}");
    let text = std::fs::read_to_string(&out).unwrap_or_default();
    assert!(text.contains("hello subs"), "extracted srt has cue: {text}");
}

#[test]
fn meta_rotate_writes_display_matrix() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let out = dir.path().join("rot.mp4");
    let v = run_json(&[
        "meta",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--rotate",
        "90",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let rot = |p: &std::path::Path| -> f64 {
        // ffprobe <7 lacks the `stream_side_data` show_entries section.
        let o = Command::new("ffprobe")
            .args(["-v", "error", "-select_streams", "v:0", "-show_streams"])
            .arg(p)
            .output()
            .unwrap();
        String::from_utf8_lossy(&o.stdout)
            .lines()
            .find_map(|l| {
                l.trim()
                    .strip_prefix("rotation=")
                    .and_then(|r| r.parse::<f64>().ok())
            })
            .unwrap_or(0.0)
    };
    assert!(
        (rot(&out).abs() - 90.0).abs() < 0.1,
        "display matrix rotation +/-90, got {}",
        rot(&out)
    );
    // lossless: duration unchanged
    assert!(
        (v["probe"]["duration"].as_f64().unwrap_or(0.0) - 1.0).abs() < 0.05,
        "{v}"
    );
    // clear it back
    let out2 = dir.path().join("rot0.mp4");
    let v2 = run_json(&[
        "meta",
        out.to_str().unwrap(),
        "-o",
        out2.to_str().unwrap(),
        "--rotate",
        "0",
    ]);
    assert_eq!(v2["status"], "ok", "{v2}");
    assert!(
        rot(&out2).abs() < 45.0,
        "rotation cleared to ~0, got {}",
        rot(&out2)
    );
}

#[test]
fn audiogram_mode_and_color_recolor_wave() {
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
    // red line-mode wave vs default white cline
    let red = dir.path().join("red.mp4");
    let vr = run_json(&[
        "audiogram",
        wav.to_str().unwrap(),
        "-o",
        red.to_str().unwrap(),
        "--mode",
        "line",
        "--color",
        "0xFF0000",
    ]);
    assert_eq!(vr["status"], "ok", "{vr}");
    let white = dir.path().join("white.mp4");
    let vw = run_json(&[
        "audiogram",
        wav.to_str().unwrap(),
        "-o",
        white.to_str().unwrap(),
    ]);
    assert_eq!(vw["status"], "ok", "{vw}");
    // R-B mean inside the wave band (overlay sits ~(H-h)*0.62 down)
    let rb = |p: &std::path::Path| -> f64 {
        let o = Command::new("ffmpeg")
            .args(["-ss", "0.5", "-i"])
            .arg(p)
            .args([
                "-vf",
                "crop=600:200:240:1050",
                "-frames:v",
                "1",
                "-f",
                "rawvideo",
                "-pix_fmt",
                "rgb24",
                "-",
            ])
            .output()
            .unwrap();
        let px = &o.stdout;
        assert!(px.len() >= 600 * 200 * 3, "raw rgb24 frame");
        let (mut rs, mut bs) = (0u64, 0u64);
        for c in px.chunks_exact(3) {
            rs += c[0] as u64;
            bs += c[2] as u64;
        }
        (rs as f64 - bs as f64) / (px.len() / 3) as f64
    };
    let (red_rb, white_rb) = (rb(&red), rb(&white));
    assert!(
        red_rb > white_rb + 10.0,
        "red wave is redder than white wave: red(R-B)={red_rb} white={white_rb}"
    );
}

#[test]
fn delogo_at_blurs_only_the_window() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let marked = dir.path().join("marked.mp4");
    let ok = Command::new("ffmpeg")
        .args(["-hide_banner", "-loglevel", "error", "-y", "-i"])
        .arg(&src)
        .args([
            "-vf",
            "drawbox=x=10:y=10:w=60:h=40:color=white:t=fill",
            "-c:v",
            "libx264",
            "-preset",
            "fast",
            "-crf",
            "18",
            "-c:a",
            "copy",
        ])
        .arg(&marked)
        .status()
        .unwrap()
        .success();
    assert!(ok, "marked fixture");
    let out = dir.path().join("d.mp4");
    let v = run_json(&[
        "delogo",
        marked.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--x",
        "10",
        "--y",
        "10",
        "--w",
        "60",
        "--h",
        "40",
        "--at",
        "0.5",
        "--dur",
        "0.5",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let luma = |n: u32| -> f64 {
        let o = Command::new("ffmpeg")
            .args(["-i"])
            .arg(&out)
            .args([
                "-vf",
                &format!("select=eq(n\\,{n}),crop=40:20:20:20,signalstats,metadata=print:key=lavfi.signalstats.YAVG:file=-"),
                "-frames:v", "1", "-f", "null", "-",
            ])
            .output()
            .unwrap();
        String::from_utf8_lossy(&o.stdout)
            .split("YAVG=")
            .nth(1)
            .and_then(|r| r.lines().next())
            .and_then(|r| r.trim().parse().ok())
            .unwrap_or(-1.0)
    };
    let (before, inside) = (luma(3), luma(21));
    assert!(
        before > inside + 30.0,
        "box intact before window ({before}) but blended inside ({inside}); {v}"
    );
}

#[test]
fn reverb_adds_tail_after_tone() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    // 0.2s tone then silence — the tail window stays silent unless reverb rings
    let wav = dir.path().join("pulse.wav");
    let ok = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=0.2,apad=whole_dur=1",
            "-c:a",
            "pcm_s16le",
        ])
        .arg(&wav)
        .status()
        .unwrap()
        .success();
    assert!(ok, "pulse fixture");
    let tail = |p: &std::path::Path| -> f64 {
        let o = Command::new("ffmpeg")
            .args(["-i"])
            .arg(p)
            .args([
                "-af",
                "atrim=0.3:0.5,volumedetect",
                "-vn",
                "-f",
                "null",
                "-",
            ])
            .output()
            .unwrap();
        String::from_utf8_lossy(&o.stderr)
            .lines()
            .find_map(|l| {
                l.split("mean_volume:")
                    .nth(1)
                    .and_then(|r| r.split_whitespace().next())
                    .and_then(|x| x.parse::<f64>().ok())
            })
            .unwrap_or(0.0)
    };
    let dry_tail = tail(&wav);
    let out = dir.path().join("wet.wav");
    let v = run_json(&[
        "reverb",
        wav.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--size",
        "hall",
        "--wet",
        "0.8",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let wet_tail = tail(&out);
    assert!(
        wet_tail > dry_tail + 10.0,
        "reverb rings into the silent tail: dry {dry_tail} dB -> wet {wet_tail} dB; {v}"
    );
}

#[test]
fn bleep_replaces_source_with_tone_in_window() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let out = dir.path().join("b.mp4");
    let v = run_json(&[
        "bleep",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--at",
        "0.3",
        "--dur",
        "0.4",
        "--freq",
        "1000",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let band = |f: &Path, win: &str, hz: u32| -> f64 {
        let o = Command::new("ffmpeg")
            .args(["-i"])
            .arg(f)
            .args([
                "-af",
                &format!("atrim={win},bandpass=f={hz}:w=200,volumedetect"),
                "-vn",
                "-f",
                "null",
                "-",
            ])
            .output()
            .unwrap();
        String::from_utf8_lossy(&o.stderr)
            .lines()
            .find_map(|l| {
                l.split("mean_volume:")
                    .nth(1)
                    .and_then(|r| r.split_whitespace().next())
                    .and_then(|x| x.parse::<f64>().ok())
            })
            .unwrap_or(-99.0)
    };
    let beep = band(&out, "0.3:0.7", 1000);
    let src_in = band(&out, "0.3:0.7", 440);
    let src_out = band(&out, "0:0.2", 440);
    assert!(beep > -50.0, "1kHz beep audible in window ({beep} dB); {v}");
    assert!(
        src_in < src_out - 20.0,
        "440 silenced in window ({src_in} vs {src_out} outside); {v}"
    );
}

#[test]
fn censor_at_pixelizes_only_the_window() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let out = dir.path().join("c.mp4");
    let v = run_json(&[
        "censor",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--region",
        "100:100:64:64",
        "--at",
        "0.5",
        "--dur",
        "0.5",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let region = |f: &Path, n: u32| -> Vec<u8> {
        Command::new("ffmpeg")
            .args(["-i"])
            .arg(f)
            .args([
                "-vf",
                &format!("select=eq(n\\,{n}),crop=64:64:100:100"),
                "-frames:v",
                "1",
                "-f",
                "rawvideo",
                "-pix_fmt",
                "gray",
                "-",
            ])
            .output()
            .unwrap()
            .stdout
    };
    let diff_at = |n: u32| -> u64 {
        region(&src, n)
            .iter()
            .zip(region(&out, n).iter())
            .map(|(x, y)| x.abs_diff(*y) as u64)
            .sum()
    };
    assert!(
        diff_at(3) < 5000,
        "region untouched before --at 0.5 (diff {})",
        diff_at(3)
    );
    assert!(
        diff_at(21) > 5000,
        "mosaic applied inside window (diff {})",
        diff_at(21)
    );
}

#[test]
fn grade_warm_shifts_red_up_and_blue_down() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let gray = dir.path().join("g.mp4");
    let ok = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "color=c=gray:duration=1:size=320x240:rate=30",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
        ])
        .arg(&gray)
        .status()
        .unwrap()
        .success();
    assert!(ok, "gray fixture");
    let out = dir.path().join("w.mp4");
    let v = run_json(&[
        "grade",
        gray.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--warm",
        "0.8",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let frame = |f: &Path| -> PathBuf {
        let png = f.with_extension("frame.png");
        Command::new("ffmpeg")
            .args(["-y", "-i"])
            .arg(f)
            .args(["-frames:v", "1"])
            .arg(&png)
            .output()
            .unwrap();
        png
    };
    let (r0, _, b0) = mean_rgb(&frame(&gray));
    let (r1, _, b1) = mean_rgb(&frame(&out));
    // Warmth opens the red-blue spread (blue falls faster than red on gray).
    assert!(
        (r1 - b1) > (r0 - b0) + 30.0,
        "warm grade opens R-B spread ({r0}-{b0} -> {r1}-{b1}); {v}"
    );
}

#[test]
fn vdenoise_smooths_noisy_footage() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let noisy = dir.path().join("n.mp4");
    let ok = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "color=c=gray:duration=1:size=320x240:rate=30,noise=alls=30:allf=t",
            "-c:v",
            "libx264",
            "-preset",
            "fast",
            "-crf",
            "18",
            "-pix_fmt",
            "yuv420p",
        ])
        .arg(&noisy)
        .status()
        .unwrap()
        .success();
    assert!(ok, "noisy fixture");
    let out = dir.path().join("d.mp4");
    let v = run_json(&[
        "vdenoise",
        noisy.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--strength",
        "8",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    // Sample variance of a flat crop: denoised footage is smoother.
    let spread = |f: &Path| -> f64 {
        let px = Command::new("ffmpeg")
            .args(["-i"])
            .arg(f)
            .args([
                "-vf",
                "crop=120:120:100:60",
                "-frames:v",
                "1",
                "-f",
                "rawvideo",
                "-pix_fmt",
                "gray",
                "-",
            ])
            .output()
            .unwrap()
            .stdout;
        let n = px.len() as f64;
        let mean = px.iter().map(|&b| f64::from(b)).sum::<f64>() / n;
        px.iter()
            .map(|&b| (f64::from(b) - mean).powi(2))
            .sum::<f64>()
            / n
    };
    let (before, after) = (spread(&noisy), spread(&out));
    assert!(
        after < before * 0.7,
        "denoise cuts noise variance: {before} -> {after}; {v}"
    );
}

#[test]
fn crop_region_and_aspect_reframe() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let out = dir.path().join("c.mp4");
    let v = run_json(&[
        "crop",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--region",
        "40:20:200:100",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let dims = |f: &Path| -> (u64, u64) {
        let p = run_json(&["probe", f.to_str().unwrap()]);
        let pr = &p["probe"];
        (
            pr["width"].as_u64().unwrap_or(0),
            pr["height"].as_u64().unwrap_or(0),
        )
    };
    assert_eq!(dims(&out), (200, 100), "region crop dims; {v}");
    let out2 = dir.path().join("sq.mp4");
    let v = run_json(&[
        "crop",
        src.to_str().unwrap(),
        "-o",
        out2.to_str().unwrap(),
        "--aspect",
        "1:1",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let (w, h) = dims(&out2);
    assert_eq!(w, h, "1:1 aspect gives a square ({w}x{h}); {v}");
    let v = run_json(&[
        "crop",
        src.to_str().unwrap(),
        "-o",
        dir.path().join("x.mp4").to_str().unwrap(),
        "--region",
        "300:200:64:64",
    ]);
    assert_eq!(v["status"], "failed", "out-of-frame region refused; {v}");
}

#[test]
fn title_position_bottom_puts_text_low() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let bot = dir.path().join("b.mp4");
    let v = run_json(&[
        "title",
        src.to_str().unwrap(),
        "-o",
        bot.to_str().unwrap(),
        "--text",
        "HELLO",
        "--position",
        "bottom",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    // Diff against the source frame per band: text should land in the bottom
    // quarter while the top quarter stays untouched.
    let band_diff = |y: u32| -> u64 {
        let px = |f: &Path| -> Vec<u8> {
            Command::new("ffmpeg")
                .args(["-i"])
                .arg(f)
                .args([
                    "-vf",
                    &format!("select=eq(n\\,10),crop=320:60:0:{y}"),
                    "-frames:v",
                    "1",
                    "-f",
                    "rawvideo",
                    "-pix_fmt",
                    "gray",
                    "-",
                ])
                .output()
                .unwrap()
                .stdout
        };
        px(&src)
            .iter()
            .zip(px(&bot).iter())
            .map(|(a, b)| a.abs_diff(*b) as u64)
            .sum()
    };
    let (top_band, bot_band) = (band_diff(0), band_diff(180));
    assert!(
        bot_band > top_band * 4,
        "title edits the bottom band (top diff {top_band} vs bottom {bot_band}); {v}"
    );
}

#[test]
fn waveform_renders_a_drawn_png() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let png = dir.path().join("w.png");
    let v = run_json(&[
        "waveform",
        src.to_str().unwrap(),
        "-o",
        png.to_str().unwrap(),
        "--size",
        "320x120",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let img = image::open(&png).expect("png").to_rgb8();
    let lit = img
        .pixels()
        .filter(|p| u32::from(p[0]) + u32::from(p[1]) + u32::from(p[2]) > 200)
        .count();
    assert!(lit > 300, "waveform PNG draws the wave ({lit} lit px); {v}");
}

#[test]
fn spectrogram_renders_a_drawn_png() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let png = dir.path().join("s.png");
    let v = run_json(&[
        "spectrogram",
        src.to_str().unwrap(),
        "-o",
        png.to_str().unwrap(),
        "--size",
        "320x240",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let img = image::open(&png).expect("png").to_rgb8();
    let lit = img
        .pixels()
        .filter(|p| u32::from(p[0]) + u32::from(p[1]) + u32::from(p[2]) > 60)
        .count();
    assert!(
        lit > 1000,
        "spectrogram PNG draws energy ({lit} lit px); {v}"
    );
}

#[test]
fn dehum_notches_the_mains_tone() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let hum = dir.path().join("hum.wav");
    // 440 Hz voice over a 60 Hz mains hum.
    let ok = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=1",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=60:duration=1",
            "-filter_complex",
            "[0:a][1:a]amix=inputs=2:normalize=0[a]",
            "-map",
            "[a]",
            "-c:a",
            "pcm_s16le",
        ])
        .arg(&hum)
        .status()
        .unwrap()
        .success();
    assert!(ok, "hum fixture");
    let out = dir.path().join("clean.m4a");
    let v = run_json(&[
        "dehum",
        hum.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--mains",
        "60",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let band = |f: &Path, hz: u32| -> f64 {
        let o = Command::new("ffmpeg")
            .args(["-i"])
            .arg(f)
            .args([
                "-af",
                &format!("bandpass=f={hz}:w=15,volumedetect"),
                "-f",
                "null",
                "-",
            ])
            .output()
            .unwrap();
        String::from_utf8_lossy(&o.stderr)
            .lines()
            .find_map(|l| {
                l.split("mean_volume:")
                    .nth(1)
                    .and_then(|r| r.split_whitespace().next())
                    .and_then(|x| x.parse::<f64>().ok())
            })
            .unwrap_or(-99.0)
    };
    let (hum_in, hum_out, voice_out) = (band(&hum, 60), band(&out, 60), band(&out, 440));
    assert!(
        hum_out < hum_in - 15.0,
        "60Hz hum notched: {hum_in} -> {hum_out} dB; {v}"
    );
    assert!(
        voice_out > -40.0,
        "440 voice survives ({voice_out} dB); {v}"
    );
}

#[test]
fn tempo_doubles_speed_without_pitch_loss() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("t.wav");
    let ok = Command::new("ffmpeg")
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
        .unwrap()
        .success();
    assert!(ok, "tone fixture");
    let out = dir.path().join("f.m4a");
    let v = run_json(&[
        "tempo",
        wav.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--factor",
        "2",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let p = run_json(&["probe", out.to_str().unwrap()]);
    let d = p["probe"]["duration"].as_f64().unwrap_or(0.0);
    assert!((d - 0.5).abs() < 0.12, "2x halves duration ({d}); {v}");
    // A 440 Hz sine stays at 440 Hz — atempo shifts time, not pitch.
    let o = Command::new("ffmpeg")
        .args(["-i"])
        .arg(&out)
        .args(["-af", "bandpass=f=440:w=60,volumedetect", "-f", "null", "-"])
        .output()
        .unwrap();
    let mean = String::from_utf8_lossy(&o.stderr)
        .lines()
        .find_map(|l| {
            l.split("mean_volume:")
                .nth(1)
                .and_then(|r| r.split_whitespace().next())
                .and_then(|x| x.parse::<f64>().ok())
        })
        .unwrap_or(-99.0);
    assert!(mean > -40.0, "440 Hz pitch preserved ({mean} dB); {v}");
    // Video inputs are refused — speed retimes those.
    let src = fixture(dir.path());
    let v = run_json(&[
        "tempo",
        src.to_str().unwrap(),
        "-o",
        dir.path().join("v.m4a").to_str().unwrap(),
    ]);
    assert_eq!(v["status"], "failed", "video refused for tempo; {v}");
}

#[test]
fn leveler_tames_loud_peaks() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    // Alternating loud/quiet 0.1s tones — compression narrows the gap.
    let wav = dir.path().join("dyn.wav");
    let ok = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=0.4",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=0.4",
            "-filter_complex",
            "[0:a]volume=0.9[a];[1:a]volume=0.05[b];[a][b]concat=n=2:v=0:a=1",
            "-c:a",
            "pcm_s16le",
        ])
        .arg(&wav)
        .status()
        .unwrap()
        .success();
    assert!(ok, "dyn fixture");
    let out = dir.path().join("l.m4a");
    let v = run_json(&[
        "leveler",
        wav.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--threshold",
        "-30",
        "--ratio",
        "10",
        "--makeup",
        "12",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let seg = |f: &Path, win: &str| -> f64 {
        let o = Command::new("ffmpeg")
            .args(["-i"])
            .arg(f)
            .args([
                "-af",
                &format!("atrim={win},volumedetect"),
                "-f",
                "null",
                "-",
            ])
            .output()
            .unwrap();
        String::from_utf8_lossy(&o.stderr)
            .lines()
            .find_map(|l| {
                l.split("mean_volume:")
                    .nth(1)
                    .and_then(|r| r.split_whitespace().next())
                    .and_then(|x| x.parse::<f64>().ok())
            })
            .unwrap_or(-99.0)
    };
    let (loud0, quiet0) = (seg(&wav, "0:0.3"), seg(&wav, "0.5:0.8"));
    let (loud1, quiet1) = (seg(&out, "0:0.3"), seg(&out, "0.5:0.8"));
    assert!(
        (loud1 - quiet1) < (loud0 - quiet0) - 8.0,
        "range narrows: {loud0}-{quiet0}dB gap -> {loud1}-{quiet1}dB; {v}"
    );
}

#[test]
fn gate_silences_the_quiet_parts() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("t.wav");
    // Loud tone then a quiet hiss — the gate should kill the hiss.
    let ok = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=0.5",
            "-f",
            "lavfi",
            "-i",
            "anoisesrc=color=white:duration=0.5:amplitude=0.05",
            "-filter_complex",
            "[0:a][1:a]concat=n=2:v=0:a=1",
            "-c:a",
            "pcm_s16le",
        ])
        .arg(&wav)
        .status()
        .unwrap()
        .success();
    assert!(ok, "gate fixture");
    let out = dir.path().join("g.m4a");
    let v = run_json(&[
        "gate",
        wav.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--threshold",
        "-20",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let tail = |f: &Path| -> f64 {
        let o = Command::new("ffmpeg")
            .args(["-i"])
            .arg(f)
            .args(["-af", "atrim=0.5:1,volumedetect", "-f", "null", "-"])
            .output()
            .unwrap();
        String::from_utf8_lossy(&o.stderr)
            .lines()
            .find_map(|l| {
                l.split("mean_volume:")
                    .nth(1)
                    .and_then(|r| r.split_whitespace().next())
                    .and_then(|x| x.parse::<f64>().ok())
            })
            .unwrap_or(0.0)
    };
    let (before, after) = (tail(&wav), tail(&out));
    assert!(
        after < before - 10.0,
        "gate drops the quiet tail: {before} -> {after} dB; {v}"
    );
}

#[test]
fn silence_inserts_quiet_mid_audio() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("t.wav");
    let ok = Command::new("ffmpeg")
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
        .unwrap()
        .success();
    assert!(ok, "tone fixture");
    let out = dir.path().join("s.m4a");
    let v = run_json(&[
        "silence",
        wav.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--at",
        "0.25",
        "--dur",
        "0.5",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let d = v["probe"]["duration"].as_f64().unwrap_or(0.0);
    assert!(
        (d - 1.5).abs() < 0.25,
        "1s + 0.5s pad = ~1.5s, got {d}; {v}"
    );
    let mean_at = |ss: &str, dur: &str| -> f64 {
        let o = Command::new("ffmpeg")
            .args(["-ss", ss, "-t", dur, "-i"])
            .arg(&out)
            .args(["-af", "volumedetect", "-vn", "-f", "null", "-"])
            .output()
            .unwrap();
        let s = String::from_utf8_lossy(&o.stderr);
        s.lines()
            .find_map(|l| {
                l.split("mean_volume:")
                    .nth(1)
                    .and_then(|r| r.split_whitespace().next())
                    .and_then(|x| x.parse::<f64>().ok())
            })
            .unwrap_or(0.0)
    };
    assert!(
        mean_at("0.3", "0.35") < -50.0,
        "inserted window is silent; {v}"
    );
    assert!(mean_at("0.9", "0.4") > -40.0, "tone resumes after pad; {v}");
    // Video inputs are refused — freeze holds frames.
    let src = fixture(dir.path());
    let v = run_json(&[
        "silence",
        src.to_str().unwrap(),
        "-o",
        dir.path().join("v.m4a").to_str().unwrap(),
        "--dur",
        "0.5",
    ]);
    assert_eq!(v["status"], "failed", "video refused for silence; {v}");
}

#[test]
fn transcode_fps_retimes_video() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let out = dir.path().join("r.mp4");
    let v = run_json(&[
        "transcode",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--preset",
        "h264",
        "--fps",
        "10",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let o = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-count_frames",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=nb_read_frames",
            "-of",
            "csv=p=0",
        ])
        .arg(&out)
        .output()
        .unwrap();
    let s = String::from_utf8_lossy(&o.stdout);
    let frames: u32 = s.trim().parse().unwrap_or(0);
    assert!(
        (9..=12).contains(&frames),
        "1s at 10fps = ~10 frames, got {frames}; {v}"
    );
}

#[test]
fn grade_preset_vintage_shifts_warm() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let out = dir.path().join("v.mp4");
    let v = run_json(&[
        "grade",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--preset",
        "vintage",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let frame = |f: &Path| -> (f64, f64) {
        let png = f.with_extension("warm.png");
        let ok = Command::new("ffmpeg")
            .args(["-hide_banner", "-loglevel", "error", "-y", "-i"])
            .arg(f)
            .args(["-frames:v", "1"])
            .arg(&png)
            .status()
            .unwrap()
            .success();
        assert!(ok, "frame extract");
        let (r, _g, b) = mean_rgb(&png);
        (r, b)
    };
    let (sr, sb) = frame(&src);
    let (r, b) = frame(&out);
    assert!(
        (r - b) > (sr - sb) + 4.0,
        "vintage warms the cast: R-B {} -> {}; {v}",
        sr - sb,
        r - b
    );
}

#[test]
fn vocal_karaoke_removes_center() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("st.wav");
    // Identical 440 Hz in both channels = maximally "centered" content.
    let ok = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=1,pan=stereo|c0=c0|c1=c0",
            "-c:a",
            "pcm_s16le",
        ])
        .arg(&wav)
        .status()
        .unwrap()
        .success();
    assert!(ok, "stereo fixture");
    let mean = |f: &Path| -> f64 {
        let o = Command::new("ffmpeg")
            .args(["-i"])
            .arg(f)
            .args(["-af", "volumedetect", "-vn", "-f", "null", "-"])
            .output()
            .unwrap();
        String::from_utf8_lossy(&o.stderr)
            .lines()
            .find_map(|l| {
                l.split("mean_volume:")
                    .nth(1)
                    .and_then(|r| r.split_whitespace().next())
                    .and_then(|x| x.parse::<f64>().ok())
            })
            .unwrap_or(0.0)
    };
    let kara = dir.path().join("k.m4a");
    let v = run_json(&[
        "vocal",
        wav.to_str().unwrap(),
        "-o",
        kara.to_str().unwrap(),
        "--mode",
        "karaoke",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    assert!(
        mean(&kara) < -60.0,
        "karaoke cancels centered content (~silent); {v}"
    );
    let iso = dir.path().join("i.m4a");
    let v = run_json(&[
        "vocal",
        wav.to_str().unwrap(),
        "-o",
        iso.to_str().unwrap(),
        "--mode",
        "isolate",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    assert!(mean(&iso) > -30.0, "isolate keeps the center loud; {v}");
}

#[test]
fn remux_swaps_container_without_reencode() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let out = dir.path().join("r.mkv");
    let v = run_json(&["remux", src.to_str().unwrap(), "-o", out.to_str().unwrap()]);
    assert_eq!(v["status"], "ok", "{v}");
    assert!(
        v["probe"]["format"]
            .as_str()
            .unwrap_or("")
            .contains("matroska"),
        "mkv container; {v}"
    );
    assert_eq!(
        v["probe"]["vcodec"].as_str().unwrap_or(""),
        "h264",
        "stream copy keeps h264; {v}"
    );
}

#[test]
fn meme_top_text_stays_in_band() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let out = dir.path().join("m.mp4");
    let v = run_json(&[
        "meme",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--top",
        "TOP TEXT",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let band_diff = |y: u32| -> u64 {
        let px = |f: &Path| -> Vec<u8> {
            Command::new("ffmpeg")
                .args(["-i"])
                .arg(f)
                .args([
                    "-vf",
                    &format!("select=eq(n\\,10),crop=320:60:0:{y}"),
                    "-frames:v",
                    "1",
                    "-f",
                    "rawvideo",
                    "-pix_fmt",
                    "gray",
                    "-",
                ])
                .output()
                .unwrap()
                .stdout
        };
        px(&src)
            .iter()
            .zip(px(&out).iter())
            .map(|(a, b)| a.abs_diff(*b) as u64)
            .sum()
    };
    let (top_band, bot_band) = (band_diff(0), band_diff(180));
    assert!(
        top_band > bot_band * 4,
        "meme edits the top band (top diff {top_band} vs bottom {bot_band}); {v}"
    );
}

#[test]
fn voice_chain_levels_and_loudness() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("v.wav");
    // Loud burst, a hiss-quiet stretch, then speech-level tone again.
    let ok = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=300:duration=0.4,volume=6dB",
            "-f",
            "lavfi",
            "-i",
            "anoisesrc=color=pink:duration=0.4:amplitude=0.02",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=300:duration=0.4,volume=-6dB",
            "-filter_complex",
            "[0:a][1:a][2:a]concat=n=3:v=0:a=1[a]",
            "-map",
            "[a]",
            "-c:a",
            "pcm_s16le",
        ])
        .arg(&wav)
        .status()
        .unwrap()
        .success();
    assert!(ok, "voice fixture");
    let out = dir.path().join("v.m4a");
    let v = run_json(&["voice", wav.to_str().unwrap(), "-o", out.to_str().unwrap()]);
    assert_eq!(v["status"], "ok", "{v}");
    let o = Command::new("ffmpeg")
        .args(["-i"])
        .arg(&out)
        .args(["-af", "volumedetect", "-vn", "-f", "null", "-"])
        .output()
        .unwrap();
    let s = String::from_utf8_lossy(&o.stderr);
    let mean = s
        .lines()
        .find_map(|l| {
            l.split("mean_volume:")
                .nth(1)
                .and_then(|r| r.split_whitespace().next())
                .and_then(|x| x.parse::<f64>().ok())
        })
        .unwrap_or(0.0);
    assert!(
        (-30.0..-5.0).contains(&mean),
        "leveled voice lands near broadcast mean ({mean} dB); {v}"
    );
}

#[test]
fn deinterlace_field_doubles_rate() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let out = dir.path().join("d.mp4");
    let v = run_json(&[
        "deinterlace",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--mode",
        "field",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let o = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-count_frames",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=nb_read_frames",
            "-of",
            "csv=p=0",
        ])
        .arg(&out)
        .output()
        .unwrap();
    let frames: u32 = String::from_utf8_lossy(&o.stdout)
        .trim()
        .parse()
        .unwrap_or(0);
    assert!(
        (55..=62).contains(&frames),
        "field mode doubles 30 frames to ~60, got {frames}; {v}"
    );
}

#[test]
fn fade_color_white_fades_to_white() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let out = dir.path().join("w.mp4");
    let v = run_json(&[
        "fade",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--out",
        "0.4",
        "--color",
        "white",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let png = dir.path().join("last.png");
    let ok = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-sseof",
            "-0.1",
            "-i",
        ])
        .arg(&out)
        .args(["-frames:v", "1"])
        .arg(&png)
        .status()
        .unwrap()
        .success();
    assert!(ok, "last frame");
    let (r, g, b) = mean_rgb(&png);
    let m = (r + g + b) / 3.0;
    assert!(
        m > 200.0,
        "fade --color white ends near white ({m:.0}); {v}"
    );
}

#[test]
fn crossfade_overlaps_two_audio_files() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let mk = |name: &str, hz: u32| -> PathBuf {
        let f = dir.path().join(name);
        let ok = Command::new("ffmpeg")
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                &format!("sine=frequency={hz}:duration=1"),
                "-c:a",
                "pcm_s16le",
            ])
            .arg(&f)
            .status()
            .unwrap()
            .success();
        assert!(ok, "tone {hz}");
        f
    };
    let a = mk("a.wav", 440);
    let b = mk("b.wav", 880);
    let out = dir.path().join("x.m4a");
    let v = run_json(&[
        "crossfade",
        a.to_str().unwrap(),
        "--second",
        b.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--dur",
        "0.3",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let d = v["probe"]["duration"].as_f64().unwrap_or(0.0);
    assert!(
        (d - 1.7).abs() < 0.25,
        "1+1-0.3 overlap = ~1.7s, got {d}; {v}"
    );
    // The overlap window should carry both tones (mixed), not silence.
    let o = Command::new("ffmpeg")
        .args(["-ss", "0.7", "-t", "0.25", "-i"])
        .arg(&out)
        .args(["-af", "volumedetect", "-vn", "-f", "null", "-"])
        .output()
        .unwrap();
    let s = String::from_utf8_lossy(&o.stderr);
    let mean = s
        .lines()
        .find_map(|l| {
            l.split("mean_volume:")
                .nth(1)
                .and_then(|r| r.split_whitespace().next())
                .and_then(|x| x.parse::<f64>().ok())
        })
        .unwrap_or(0.0);
    assert!(mean > -60.0, "overlap keeps signal ({mean} dB); {v}");
}

#[test]
fn strip_drops_metadata_tags() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let tagged = dir.path().join("t.mp4");
    let v = run_json(&[
        "meta",
        src.to_str().unwrap(),
        "-o",
        tagged.to_str().unwrap(),
        "--title",
        "Secret Title",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let out = dir.path().join("clean.mp4");
    let v = run_json(&[
        "strip",
        tagged.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let o = Command::new("ffprobe")
        .args(["-v", "error", "-show_entries", "format_tags", "-of", "json"])
        .arg(&out)
        .output()
        .unwrap();
    let s = String::from_utf8_lossy(&o.stdout);
    assert!(!s.contains("Secret Title"), "title stripped: {s}");
    let d = v["probe"]["duration"].as_f64().unwrap_or(0.0);
    assert!((d - 1.0).abs() < 0.3, "copy keeps duration; {v}");
}

#[test]
fn frames_dumps_stills_on_a_grid() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let base = dir.path().join("shot.png");
    let v = run_json(&[
        "frames",
        src.to_str().unwrap(),
        "-o",
        base.to_str().unwrap(),
        "--every",
        "0.34",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let count = v["extra"]["count"].as_u64().unwrap_or(0);
    assert!(
        (2..=4).contains(&count),
        "1s at 0.34s grid ≈ 3 stills, got {count}; {v}"
    );
    assert!(
        dir.path().join("shot_001.png").exists(),
        "stem_%03d.png naming; {v}"
    );
}

#[test]
fn invert_flips_channels() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let out = dir.path().join("n.mp4");
    let v = run_json(&["invert", src.to_str().unwrap(), "-o", out.to_str().unwrap()]);
    assert_eq!(v["status"], "ok", "{v}");
    let frame = |f: &Path, name: &str| -> (f64, f64, f64) {
        let png = dir.path().join(name);
        let ok = Command::new("ffmpeg")
            .args(["-hide_banner", "-loglevel", "error", "-y", "-i"])
            .arg(f)
            .args(["-frames:v", "1"])
            .arg(&png)
            .status()
            .unwrap()
            .success();
        assert!(ok, "frame extract");
        mean_rgb(&png)
    };
    let (sr, sg, sb) = frame(&src, "s.png");
    let (r, g, b) = frame(&out, "o.png");
    for (a, b2) in [(r, sr), (g, sg), (b, sb)] {
        assert!(
            (a - (255.0 - b2)).abs() < 25.0,
            "inverted: {b2} -> {a} (want ~{})",
            255.0 - b2
        );
    }
}

#[test]
fn split_size_aims_parts_at_target() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    // A 1 s testsrc clip lands ~15–50 KB; 8 KB forces ≥2 parts.
    let out = dir.path().join("part.mp4");
    let v = run_json(&[
        "split",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--size",
        "8KB",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let count = v["extra"]["count"].as_u64().unwrap_or(0);
    assert!(count >= 2, "--size should force multiple parts; {v}");
    assert!(dir.path().join("part_00.mp4").exists(), "stem_%02d naming");
}

#[test]
fn countdown_shows_digits_then_clears() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let out = dir.path().join("c.mp4");
    let v = run_json(&[
        "countdown",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--from",
        "2",
        "--each",
        "0.3",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    // Center-crop diff vs the source at n=3 (inside count) vs n=28 (after it).
    let diff = |n: u32| -> u64 {
        let px = |f: &Path| -> Vec<u8> {
            Command::new("ffmpeg")
                .args(["-i"])
                .arg(f)
                .args([
                    "-vf",
                    &format!("select=eq(n\\,{n}),crop=120:80:100:80"),
                    "-frames:v",
                    "1",
                    "-f",
                    "rawvideo",
                    "-pix_fmt",
                    "gray",
                    "-",
                ])
                .output()
                .unwrap()
                .stdout
        };
        px(&src)
            .iter()
            .zip(px(&out).iter())
            .map(|(a, b)| a.abs_diff(*b) as u64)
            .sum()
    };
    let (early, late) = (diff(3), diff(28));
    assert!(
        early > 50000 && early > late * 4,
        "digit burns early then clears: {early} vs {late}; {v}"
    );
}

#[test]
fn mix_merges_two_sources() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let mk = |name: &str, freq: u32, dur: f64| -> PathBuf {
        let p = dir.path().join(name);
        let ok = Command::new("ffmpeg")
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
            ])
            .arg(format!("sine=frequency={freq}:duration={dur}"))
            .arg(&p)
            .status()
            .unwrap()
            .success();
        assert!(ok);
        p
    };
    let a = mk("a.wav", 440, 1.0);
    let b = mk("b.wav", 880, 0.5);
    let out = dir.path().join("m.wav");
    let v = run_json(&[
        "mix",
        a.to_str().unwrap(),
        b.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    assert_eq!(v["extra"]["duration_mode"], "first");
    let mean = |p: &Path| -> f64 {
        let o = Command::new("ffmpeg")
            .arg("-i")
            .arg(p)
            .args(["-af", "volumedetect", "-vn", "-f", "null", "-"])
            .output()
            .unwrap();
        String::from_utf8_lossy(&o.stderr)
            .lines()
            .find_map(|l| {
                l.split("mean_volume:")
                    .nth(1)
                    .and_then(|r| r.split_whitespace().next())
                    .and_then(|x| x.parse::<f64>().ok())
            })
            .unwrap_or(0.0)
    };
    let (solo, merged) = (mean(&a).max(mean(&b)), mean(&out));
    assert!(
        merged > solo + 1.5,
        "summed mix should run hotter than either source ({solo} -> {merged}); {v}"
    );
    let d = v["probe"]["duration"].as_f64().unwrap_or(0.0);
    assert!((d - 1.0).abs() < 0.3, "duration=first ≈ A's 1s: {d}");
}

#[test]
fn grade_gamma_lifts_mids() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("mid.mp4");
    let ok = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
        ])
        .arg("color=c=0x777777:s=320x240:d=0.5:rate=30")
        .args(["-pix_fmt", "yuv420p", "-c:v", "libx264"])
        .arg(&src)
        .status()
        .unwrap()
        .success();
    assert!(ok);
    let grab = |name: &str, gamma: &str| -> f64 {
        let o = dir.path().join(name);
        let v = run_json(&[
            "grade",
            src.to_str().unwrap(),
            "-o",
            o.to_str().unwrap(),
            "--gamma",
            gamma,
        ]);
        assert_eq!(v["status"], "ok", "{v}");
        let png = dir.path().join(format!("{name}.png"));
        let ok = Command::new("ffmpeg")
            .args(["-hide_banner", "-loglevel", "error", "-y", "-i"])
            .arg(&o)
            .args(["-frames:v", "1"])
            .arg(&png)
            .status()
            .unwrap()
            .success();
        assert!(ok);
        let (r, g, b) = mean_rgb(&png);
        (r + g + b) / 3.0
    };
    let hi = grab("hi.mp4", "2.2");
    let lo = grab("lo.mp4", "0.6");
    assert!(
        hi > lo + 10.0,
        "gamma 2.2 should out-lift gamma 0.6 ({lo} -> {hi})"
    );
}

#[test]
fn caption_position_top_keeps_text_up() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("blue.mp4");
    let ok = Command::new("ffmpeg")
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
        .unwrap()
        .success();
    assert!(ok);
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
        "--position".into(),
        "top".into(),
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
    assert_eq!(v["extra"]["position"], "top", "{v}");
    let png = dir.path().join("t.png");
    let ok = Command::new("ffmpeg")
        .args(["-hide_banner", "-loglevel", "error", "-y", "-i"])
        .arg(&out)
        .args(["-frames:v", "1"])
        .arg(&png)
        .status()
        .unwrap()
        .success();
    assert!(ok);
    let img = image::open(&png).expect("frame").to_rgb8();
    let (w, h) = img.dimensions();
    let (mut top, mut bottom_half) = (0u32, 0u32);
    for y in 0..h {
        for x in 0..w {
            let p = img.get_pixel(x, y);
            if p[0] > 230 && p[1] > 230 && p[2] > 230 {
                if y * 2 < h {
                    top += 1;
                } else {
                    bottom_half += 1;
                }
            }
        }
    }
    assert!(top > 20, "top caption glyphs visible: {top}");
    assert_eq!(
        bottom_half, 0,
        "top captions keep the lower half clean: {bottom_half}"
    );
}

#[test]
fn mute_drops_audio_keeps_video() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let out = dir.path().join("m.mp4");
    let v = run_json(&["mute", src.to_str().unwrap(), "-o", out.to_str().unwrap()]);
    assert_eq!(v["status"], "ok", "{v}");
    assert_eq!(v["probe"]["has_video"], true);
    assert_eq!(v["probe"]["has_audio"], false, "{v}");
}

#[test]
fn hls_writes_playlist_and_segments() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let out = dir.path().join("web");
    let v = run_json(&[
        "hls",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--seg",
        "0.5",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    assert!(out.join("index.m3u8").is_file(), "playlist written");
    let segs = v["extra"]["segments"].as_u64().unwrap_or(0);
    assert!(segs >= 1, "{v}");
    assert!(out.join("seg_000.ts").is_file(), "seg_000.ts exists");
}

#[test]
fn timer_burns_counter_in_corner() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let out = dir.path().join("t.mp4");
    let v = run_json(&[
        "timer",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--position",
        "bottom-right",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    // Corner crop of source vs output differs; center is untouched.
    let px = |f: &Path, crop: &str| -> Vec<u8> {
        Command::new("ffmpeg")
            .args(["-i"])
            .arg(f)
            .args([
                "-vf",
                &format!("select=eq(n\\,15),{crop}"),
                "-frames:v",
                "1",
                "-f",
                "rawvideo",
                "-pix_fmt",
                "gray",
                "-",
            ])
            .output()
            .unwrap()
            .stdout
    };
    let corner = px(&src, "crop=140:60:160:170")
        .iter()
        .zip(px(&out, "crop=140:60:160:170").iter())
        .map(|(a, b)| a.abs_diff(*b) as u64)
        .sum::<u64>();
    let center = px(&src, "crop=140:60:90:90")
        .iter()
        .zip(px(&out, "crop=140:60:90:90").iter())
        .map(|(a, b)| a.abs_diff(*b) as u64)
        .sum::<u64>();
    assert!(corner > 20000, "timer digits burn in the corner: {corner}");
    assert_eq!(center, 0, "frame center untouched: {center}");
}

#[test]
fn qa_reports_psnr_and_ssim() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let b = dir.path().join("b.mp4");
    let g = run_json(&[
        "grade",
        src.to_str().unwrap(),
        "-o",
        b.to_str().unwrap(),
        "--gamma",
        "2.2",
    ]);
    assert_eq!(g["status"], "ok", "{g}");
    let v = run_json(&["qa", src.to_str().unwrap(), b.to_str().unwrap()]);
    assert_eq!(v["status"], "ok", "{v}");
    let psnr = v["extra"]["psnr"].as_f64().unwrap_or(0.0);
    let ssim = v["extra"]["ssim"].as_f64().unwrap_or(0.0);
    assert!(
        psnr > 5.0 && psnr.is_finite(),
        "gamma-graded vs src: {psnr}"
    );
    assert!(ssim > 0.3 && ssim < 1.0, "ssim band: {ssim}");
}

#[test]
fn conform_normalizes_spec() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let out = dir.path().join("c.mp4");
    let v = run_json(&[
        "conform",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--fps",
        "15",
        "--size",
        "160x160",
        "--lufs",
        "-14",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    assert_eq!(v["probe"]["fps"].as_f64().unwrap_or(0.0), 15.0, "{v}");
    let w = v["probe"]["width"].as_u64().unwrap_or(0);
    assert!(w <= 160 && w % 2 == 0, "fit inside 160x160: {w}");
    assert_eq!(
        v["probe"]["sample_rate"].as_u64().unwrap_or(0),
        48000,
        "loudnorm tail must be resampled back: {v}"
    );
}

#[test]
fn overlay_mode_screen_brightens() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let leak = dir.path().join("leak.mp4");
    let ok = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
        ])
        .arg("color=c=0x662200:s=320x240:d=1:rate=30")
        .args(["-pix_fmt", "yuv420p", "-c:v", "libx264"])
        .arg(&leak)
        .status()
        .unwrap()
        .success();
    assert!(ok);
    let out = dir.path().join("o.mp4");
    let v = run_json(&[
        "overlay",
        src.to_str().unwrap(),
        "--video",
        leak.to_str().unwrap(),
        "--mode",
        "screen",
        "-o",
        out.to_str().unwrap(),
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    // screen blend of an orange wash warms/brightens the frame.
    let mean = |f: &Path| -> f64 {
        let png = dir
            .path()
            .join(format!("m{}.png", f.file_stem().unwrap().to_string_lossy()));
        let ok = Command::new("ffmpeg")
            .args(["-hide_banner", "-loglevel", "error", "-y", "-i"])
            .arg(f)
            .args(["-frames:v", "1"])
            .arg(&png)
            .status()
            .unwrap()
            .success();
        assert!(ok);
        let (r, g, b) = mean_rgb(&png);
        (r + g + b) / 3.0
    };
    let (s, o) = (mean(&src), mean(&out));
    assert!(
        o > s + 3.0,
        "screen blend should lift the frame ({s} -> {o})"
    );
}

#[test]
fn sync_shifts_audio_late_and_early() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let late = dir.path().join("late.mp4");
    let v = run_json(&[
        "sync",
        src.to_str().unwrap(),
        "-o",
        late.to_str().unwrap(),
        "--ms",
        "300",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let d = v["probe"]["duration"].as_f64().unwrap_or(0.0);
    assert!((d - 1.3).abs() < 0.15, "+300ms pad extends container: {d}");
    // Audio is silent for the first ~0.3s of the padded output.
    let o = Command::new("ffmpeg")
        .args(["-i"])
        .arg(&late)
        .args([
            "-ss",
            "0",
            "-t",
            "0.2",
            "-af",
            "volumedetect",
            "-vn",
            "-f",
            "null",
            "-",
        ])
        .output()
        .unwrap();
    let s = String::from_utf8_lossy(&o.stderr);
    let mean = s
        .lines()
        .find_map(|l| {
            l.split("mean_volume:")
                .nth(1)?
                .split_whitespace()
                .next()?
                .parse::<f64>()
                .ok()
        })
        .unwrap_or(0.0);
    assert!(
        mean < -60.0 || mean == -91.0 || mean.abs() < 1e-9 || mean < -55.0,
        "delay pad should be near-silent: {mean}"
    );

    let early = dir.path().join("early.mp4");
    let v2 = run_json(&[
        "sync",
        src.to_str().unwrap(),
        "-o",
        early.to_str().unwrap(),
        "--ms",
        "-300",
    ]);
    assert_eq!(v2["status"], "ok", "{v2}");
    let d2 = v2["probe"]["duration"].as_f64().unwrap_or(0.0);
    assert!(d2 <= 1.05, "trim path doesn't grow duration: {d2}");
}

#[test]
fn crop_anchor_top_keeps_top_third() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    // Top-red / bottom-blue fixture.
    let src = dir.path().join("tb.mp4");
    let ok = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
        ])
        .arg("color=c=red:s=320x480:d=0.4:rate=30,drawbox=x=0:y=240:w=320:h=240:c=blue:t=fill")
        .args(["-pix_fmt", "yuv420p", "-c:v", "libx264"])
        .arg(&src)
        .status()
        .unwrap()
        .success();
    assert!(ok);
    let out = dir.path().join("sq.mp4");
    let v = run_json(&[
        "crop",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--aspect",
        "1:1",
        "--anchor",
        "top",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let png = dir.path().join("f.png");
    let ok = Command::new("ffmpeg")
        .args(["-hide_banner", "-loglevel", "error", "-y", "-i"])
        .arg(&out)
        .args(["-frames:v", "1"])
        .arg(&png)
        .status()
        .unwrap()
        .success();
    assert!(ok);
    let (r, _g, b) = mean_rgb(&png);
    assert!(
        r > b + 30.0,
        "top-anchored 1:1 keeps the red top (r={r} b={b})"
    );
}

#[test]
fn art_attaches_cover_stream() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let aud = dir.path().join("a.wav");
    let ok = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
        ])
        .arg("sine=frequency=440:duration=0.5")
        .arg(&aud)
        .status()
        .unwrap()
        .success();
    assert!(ok);
    let img = dir.path().join("c.png");
    let ok = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
        ])
        .arg("color=c=red:s=300x300")
        .args(["-frames:v", "1"])
        .arg(&img)
        .status()
        .unwrap()
        .success();
    assert!(ok);
    let out = dir.path().join("o.mp3");
    let v = run_json(&[
        "art",
        aud.to_str().unwrap(),
        "--image",
        img.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    // Two streams in the mp3: audio + mjpeg cover.
    let o = Command::new("ffprobe")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-show_streams",
            "-of",
            "json",
        ])
        .arg(&out)
        .output()
        .unwrap();
    let j: Value = serde_json::from_slice(&o.stdout).unwrap();
    let kinds: Vec<&str> = j["streams"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|s| s["codec_type"].as_str())
        .collect();
    assert!(
        kinds.contains(&"audio") && kinds.contains(&"video"),
        "{kinds:?}"
    );
}

#[test]
fn thumb_grabs_single_frame() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let out = dir.path().join("cover.png");
    let v = run_json(&[
        "thumb",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--at",
        "0.5",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    assert!(out.is_file());
    let (r, g, b) = mean_rgb(&out);
    assert!(
        r > 1.0 || g > 1.0 || b > 1.0,
        "cover not black: {r},{g},{b}"
    );
}

#[test]
fn subs_burn_renders_caption() {
    if !has_ffmpeg() || !has_filter("subtitles") {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let srt = dir.path().join("t.srt");
    std::fs::write(&srt, "1\n00:00:00,000 --> 00:00:00,900\nHELLO SUB\n").unwrap();
    let out = dir.path().join("burned.mp4");
    let v = run_json(&[
        "subs",
        src.to_str().unwrap(),
        "--burn",
        srt.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let png = dir.path().join("f.png");
    let ok = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-ss",
            "0.4",
            "-i",
        ])
        .arg(&out)
        .args(["-frames:v", "1"])
        .arg(&png)
        .status()
        .unwrap()
        .success();
    assert!(ok);
    let o = Command::new("ffmpeg")
        .args(["-i"])
        .arg(&png)
        .args([
            "-vf",
            "crop=w=iw:h=ih/4:x=0:y=3*ih/4,signalstats,metadata=print",
            "-f",
            "null",
            "-",
        ])
        .output()
        .unwrap();
    let s = String::from_utf8_lossy(&o.stderr);
    let ymax: f64 = s
        .lines()
        .find_map(|l| l.split("YMAX=").nth(1)?.trim().parse().ok())
        .unwrap_or(0.0);
    assert!(
        ymax > 200.0,
        "burned caption should add white pixels: YMAX={ymax}"
    );
}

#[test]
fn split_parts_n_equal_chunks() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let out_tpl = dir.path().join("p_%d.mp4");
    let v = run_json(&[
        "split",
        src.to_str().unwrap(),
        "-o",
        out_tpl.to_str().unwrap(),
        "--parts",
        "2",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let p0 = dir.path().join("p_0.mp4");
    let p1 = dir.path().join("p_1.mp4");
    assert!(p0.is_file() && p1.is_file(), "expected p_0 + p_1");
}

#[test]
fn title_fade_terminates_and_renders() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let out = dir.path().join("faded.mp4");
    let v = run_json(&[
        "title",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--text",
        "FADE ME",
        "--duration",
        "0.9",
        "--fade",
        "0.3",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    // Infinite looped secondary must not stretch the output past the input.
    let d = v["probe"]["duration"].as_f64().unwrap_or(99.0);
    assert!(d < 1.3, "fade title should end with the source: {d}");
}

#[test]
fn grade_hue_rotates_colors() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    // Pure red frame: hue=120 should turn it green-ish.
    let src = dir.path().join("red.mp4");
    let ok = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
        ])
        .arg("color=c=red:s=320x240:d=0.3:rate=30")
        .args(["-pix_fmt", "yuv420p", "-c:v", "libx264"])
        .arg(&src)
        .status()
        .unwrap()
        .success();
    assert!(ok);
    let out = dir.path().join("rot.mp4");
    let v = run_json(&[
        "grade",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--hue",
        "120",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let png = dir.path().join("f.png");
    let ok = Command::new("ffmpeg")
        .args(["-hide_banner", "-loglevel", "error", "-y", "-i"])
        .arg(&out)
        .args(["-frames:v", "1"])
        .arg(&png)
        .status()
        .unwrap()
        .success();
    assert!(ok);
    let (r, g, _b) = mean_rgb(&png);
    assert!(
        g > r + 20.0,
        "hue=120 turns a red frame green-ish (r={r} g={g})"
    );
}

#[test]
fn silence_end_appends_quiet_tail() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let aud = dir.path().join("a.wav");
    let ok = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
        ])
        .arg("sine=frequency=440:duration=1")
        .arg(&aud)
        .status()
        .unwrap()
        .success();
    assert!(ok);
    let out = dir.path().join("pad.wav");
    let v = run_json(&[
        "silence",
        aud.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--end",
        "--dur",
        "1",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let d = v["probe"]["duration"].as_f64().unwrap_or(0.0);
    assert!(
        (d - 2.0).abs() < 0.1,
        "--end appends 1s of silence to a 1s clip: {d}"
    );
}

#[test]
fn overlay_fade_terminates_and_writes() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let img = dir.path().join("logo.png");
    let ok = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
        ])
        .arg("color=c=white:s=80x80")
        .args(["-frames:v", "1"])
        .arg(&img)
        .status()
        .unwrap()
        .success();
    assert!(ok);
    let out = dir.path().join("ov.mp4");
    let v = run_json(&[
        "overlay",
        src.to_str().unwrap(),
        "--image",
        img.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--fade",
        "0.3",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let d = v["probe"]["duration"].as_f64().unwrap_or(99.0);
    assert!(d < 1.3, "looped still must not stretch output: {d}");
}

#[test]
fn subs_shift_moves_all_cues() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let srt = dir.path().join("in.srt");
    std::fs::write(
        &srt,
        "1\n00:00:01,000 --> 00:00:02,000\nHELLO\n\n2\n00:00:05,000 --> 00:00:06,500\nWORLD\n",
    )
    .unwrap();
    let out = dir.path().join("out.srt");
    let v = run_json(&[
        "subs",
        srt.to_str().unwrap(),
        "--shift",
        "2.5",
        "-o",
        out.to_str().unwrap(),
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let txt = std::fs::read_to_string(&out).unwrap();
    assert!(
        txt.contains("00:00:03,500 --> 00:00:04,500"),
        "cue1 shifted +2.5: {txt}"
    );
    assert!(
        txt.contains("00:00:07,500 --> 00:00:09,000"),
        "cue2 shifted +2.5: {txt}"
    );
}

#[test]
fn meta_album_genre_track_tags() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let out = dir.path().join("tagged.mp4");
    let v = run_json(&[
        "meta",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--album",
        "My Album",
        "--genre",
        "Podcast",
        "--track",
        "3",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let o = Command::new("ffprobe")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-show_entries",
            "format_tags",
            "-of",
            "default=noprint_wrappers=1",
        ])
        .arg(&out)
        .output()
        .unwrap();
    let tags = String::from_utf8_lossy(&o.stdout);
    assert!(tags.contains("album=My Album"), "{tags}");
    assert!(tags.contains("genre=Podcast"), "{tags}");
    assert!(tags.contains("track=3"), "{tags}");
}

#[test]
fn cut_ranges_joins_kept_segments() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path()); // 1s fixture
    let out = dir.path().join("kept.mp4");
    let v = run_json(&[
        "cut",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--ranges",
        "0-0.4,0.6-1",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let d = v["probe"]["duration"].as_f64().unwrap_or(0.0);
    assert!(
        (d - 0.8).abs() < 0.15,
        "0-0.4 + 0.6-1 should give ~0.8s: {d}"
    );
}

#[test]
fn solid_generates_color_clip() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("bg.mp4");
    let v = run_json(&[
        "solid",
        "-o",
        out.to_str().unwrap(),
        "--color",
        "ff0000",
        "--dur",
        "0.5",
        "--size",
        "320x240",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let d = v["probe"]["duration"].as_f64().unwrap_or(0.0);
    assert!((d - 0.5).abs() < 0.15, "duration: {d}");
    assert_eq!(v["probe"]["width"].as_u64(), Some(320));
    // Red-dominant frame + silent stereo track.
    let png = dir.path().join("f.png");
    let ok = Command::new("ffmpeg")
        .args(["-hide_banner", "-loglevel", "error", "-y", "-i"])
        .arg(&out)
        .args(["-frames:v", "1"])
        .arg(&png)
        .status()
        .unwrap()
        .success();
    assert!(ok);
    let (r, g, b) = mean_rgb(&png);
    assert!(r > 150.0 && g < 60.0 && b < 60.0, "red frame: {r},{g},{b}");
}

#[test]
fn volume_limit_caps_peak() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    // Hot 1kHz tone: +12dB would clip without the limiter.
    let aud = dir.path().join("hot.wav");
    let ok = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
        ])
        .arg("sine=frequency=1000:duration=0.4")
        .arg(&aud)
        .status()
        .unwrap()
        .success();
    assert!(ok);
    let out = dir.path().join("lim.wav");
    let v = run_json(&[
        "volume",
        aud.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--db",
        "12",
        "--limit",
        "-1",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let o = Command::new("ffmpeg")
        .args(["-i"])
        .arg(&out)
        .args(["-af", "volumedetect", "-f", "null", "-"])
        .output()
        .unwrap();
    let s = String::from_utf8_lossy(&o.stderr);
    let peak = s
        .lines()
        .find_map(|l| {
            l.split("max_volume:")
                .nth(1)?
                .split_whitespace()
                .next()?
                .parse::<f64>()
                .ok()
        })
        .unwrap_or(0.0);
    assert!(
        peak <= -0.5,
        "limiter -1 dBTP should cap the peak: max_volume={peak}"
    );
}

#[test]
fn cut_drop_removes_middle_segment() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path()); // 1s
    let out = dir.path().join("dropped.mp4");
    let v = run_json(&[
        "cut",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--drop",
        "0.3-0.6",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let d = v["probe"]["duration"].as_f64().unwrap_or(0.0);
    assert!(
        (d - 0.7).abs() < 0.15,
        "1s minus 0.3-0.6 should give ~0.7s: {d}"
    );
}

#[test]
fn title_outline_keeps_dark_stroke_around_text() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let out = dir.path().join("t_out.mp4");
    let v = run_json(&[
        "title",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--text",
        "HI",
        "--duration",
        "0.3",
        "--color",
        "ffffff",
        "--outline",
        "000000",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
}

#[test]
fn fit_pad_color_fills_bars() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path()); // 16:9 src
    let out = dir.path().join("fit_c.mp4");
    let v = run_json(&[
        "fit",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--aspect",
        "1:1",
        "--color",
        "ff0000",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    // top-left pad bar should be red, not black
    let png = dir.path().join("fitc.png");
    let ok = Command::new("ffmpeg")
        .args(["-hide_banner", "-loglevel", "error", "-y", "-i"])
        .arg(&out)
        .args(["-frames:v", "1"])
        .arg(&png)
        .status()
        .unwrap()
        .success();
    assert!(ok);
    let img = image::open(&png).unwrap().to_rgb8();
    let (w, _h) = img.dimensions();
    let p = img.get_pixel(w / 2, 4);
    assert!(
        p[0] > 150 && p[1] < 60 && p[2] < 60,
        "top pad bar should be red: {p:?}"
    );
}

#[test]
fn subs_burn_takes_style_overrides() {
    if !has_ffmpeg() || !has_filter("subtitles") {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let srt = dir.path().join("s.srt");
    std::fs::write(&srt, "1\n00:00:00,000 --> 00:00:00,800\nHI\n").unwrap();
    let out = dir.path().join("burn.mp4");
    let v = run_json(&[
        "subs",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--burn",
        srt.to_str().unwrap(),
        "--size",
        "30",
        "--color",
        "ff0000",
        "--top",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
}

#[test]
fn audiogram_bg_replaces_default_backdrop() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let aud = dir.path().join("a.wav");
    let ok = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
        ])
        .arg("sine=frequency=440:duration=0.5")
        .arg(&aud)
        .status()
        .unwrap()
        .success();
    assert!(ok);
    let out = dir.path().join("ag.mp4");
    let v = run_json(&[
        "audiogram",
        aud.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--bg",
        "ff0000",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
}

#[test]
fn overlay_opacity_blends_image() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let logo = dir.path().join("logo.png");
    let ok = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
        ])
        .arg("color=c=0xff0000:s=64x64:d=1")
        .args(["-frames:v", "1"])
        .arg(&logo)
        .status()
        .unwrap()
        .success();
    assert!(ok);
    let out = dir.path().join("ov.mp4");
    let v = run_json(&[
        "overlay",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--image",
        logo.to_str().unwrap(),
        "--opacity",
        "0.3",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
}

#[test]
fn split_silence_cuts_at_gap_midpoints() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    // tone - silence - tone so silencedetect finds one gap near 1.0s
    let aud = dir.path().join("gap.wav");
    let ok = Command::new("ffmpeg")
        .args(["-hide_banner", "-loglevel", "error", "-y"])
        .args(["-f", "lavfi", "-i", "sine=frequency=440:duration=1"])
        .args(["-f", "lavfi", "-t", "0.6", "-i", "anullsrc=r=44100:cl=mono"])
        .args(["-f", "lavfi", "-i", "sine=frequency=440:duration=1"])
        .args([
            "-filter_complex",
            "[0][1][2]concat=n=3:v=0:a=1[a]",
            "-map",
            "[a]",
            "-t",
            "2.6",
        ])
        .arg(&aud)
        .status()
        .unwrap()
        .success();
    assert!(ok);
    let tmpl = dir.path().join("part_%02d.wav");
    let v = run_json(&[
        "split",
        aud.to_str().unwrap(),
        "-o",
        tmpl.to_str().unwrap(),
        "--silence=-30",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    assert!(dir.path().join("part_00.wav").exists());
    assert!(dir.path().join("part_01.wav").exists());
}

#[test]
fn music_fade_does_not_break_bed() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let bed = dir.path().join("bed.wav");
    let ok = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
        ])
        .arg("sine=frequency=220:duration=2")
        .arg(&bed)
        .status()
        .unwrap()
        .success();
    assert!(ok);
    let out = dir.path().join("mus.mp4");
    let v = run_json(&[
        "music",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--track",
        bed.to_str().unwrap(),
        "--fade",
        "0.3",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
}

#[test]
fn eq_preset_fills_zero_bands() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let aud = dir.path().join("a.wav");
    let ok = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
        ])
        .arg("sine=frequency=3000:duration=0.4")
        .arg(&aud)
        .status()
        .unwrap()
        .success();
    assert!(ok);
    let out = dir.path().join("eq.wav");
    let v = run_json(&[
        "eq",
        aud.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--preset",
        "podcast",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
}

#[test]
fn broll_fade_softens_cutaway_edges() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let ins = dir.path().join("ins.mp4");
    let ok = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
        ])
        .arg("color=c=0x00ff00:s=320x240:d=1")
        .arg(&ins)
        .status()
        .unwrap()
        .success();
    assert!(ok);
    let out = dir.path().join("br.mp4");
    let v = run_json(&[
        "broll",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--insert",
        ins.to_str().unwrap(),
        "--at",
        "0.2",
        "--duration",
        "0.6",
        "--fade",
        "0.15",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let d = v["probe"]["duration"].as_f64().unwrap_or(0.0);
    assert!((d - 1.0).abs() < 0.15, "A-roll length pinned: {d}");
}

#[test]
fn frames_at_grabs_listed_timestamps() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let out = dir.path().join("shot.png");
    let v = run_json(&[
        "frames",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--at",
        "0.2,0.7",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    assert!(dir.path().join("shot_001.png").exists());
    assert!(dir.path().join("shot_002.png").exists());
}

#[test]
fn audiogram_size_sets_canvas() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let aud = dir.path().join("a.wav");
    let ok = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
        ])
        .arg("sine=frequency=440:duration=0.5")
        .arg(&aud)
        .status()
        .unwrap()
        .success();
    assert!(ok);
    let out = dir.path().join("ag.mp4");
    let v = run_json(&[
        "audiogram",
        aud.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--size",
        "1280x720",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    assert_eq!(v["probe"]["width"].as_u64(), Some(1280));
    assert_eq!(v["probe"]["height"].as_u64(), Some(720));
}

#[test]
fn audiogram_text_overlays_top_title() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let aud = dir.path().join("a.wav");
    let ok = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
        ])
        .arg("sine=frequency=440:duration=0.5")
        .arg(&aud)
        .status()
        .unwrap()
        .success();
    assert!(ok);
    let out = dir.path().join("ag.mp4");
    let v = run_json(&[
        "audiogram",
        aud.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--text",
        "EP 12",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
}

#[test]
fn replace_fade_keeps_duration() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let aud = dir.path().join("a.wav");
    let ok = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
        ])
        .arg("sine=frequency=440:duration=1.5")
        .arg(&aud)
        .status()
        .unwrap()
        .success();
    assert!(ok);
    let out = dir.path().join("rep.mp4");
    let v = run_json(&[
        "replace",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--audio",
        aud.to_str().unwrap(),
        "--fade",
        "0.2",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let d = v["probe"]["duration"].as_f64().unwrap_or(0.0);
    assert!((d - 1.0).abs() < 0.15, "video length pinned: {d}");
}

#[test]
fn thumb_width_scales_still() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let out = dir.path().join("t.png");
    let v = run_json(&[
        "thumb",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--width",
        "160",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    assert_eq!(v["probe"]["width"].as_u64(), Some(160));
}

#[test]
fn invert_at_windows_the_negation() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let out = dir.path().join("inv.mp4");
    let v = run_json(&[
        "invert",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--at",
        "0.3",
        "--dur",
        "0.4",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
}

#[test]
fn blur_at_windows_the_defocus() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let out = dir.path().join("blr.mp4");
    let v = run_json(&[
        "blur",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--at",
        "0.3",
        "--dur",
        "0.4",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
}

#[test]
fn title_corner_position_places_text() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let out = dir.path().join("tp.mp4");
    let v = run_json(&[
        "title",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--text",
        "HI",
        "--position",
        "top-right",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
}

#[test]
fn bw_at_windows_desaturation() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let out = dir.path().join("bw.mp4");
    let v = run_json(&[
        "bw",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--at",
        "0.3",
        "--dur",
        "0.4",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
}

#[test]
fn sharpen_at_windows_unsharp() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let out = dir.path().join("sh.mp4");
    let v = run_json(&[
        "sharpen",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--at",
        "0.3",
        "--dur",
        "0.4",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
}

#[test]
fn meta_clear_strips_tags() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let out = dir.path().join("mc.mp4");
    let v = run_json(&[
        "meta",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--clear",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
}

#[test]
fn vignette_at_windows_effect() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let out = dir.path().join("vig.mp4");
    let v = run_json(&[
        "vignette",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--at",
        "0.3",
        "--dur",
        "0.4",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
}

#[test]
fn grade_at_windows_look() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let out = dir.path().join("gr.mp4");
    let v = run_json(&[
        "grade",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--saturation",
        "1.8",
        "--at",
        "0.3",
        "--dur",
        "0.4",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
}

#[test]
fn broll_audio_mixes_insert_sound() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let out = dir.path().join("bra.mp4");
    let v = run_json(&[
        "broll",
        src.to_str().unwrap(),
        "--insert",
        src.to_str().unwrap(),
        "--at",
        "0.3",
        "--duration",
        "0.4",
        "--audio",
        "-o",
        out.to_str().unwrap(),
    ]);
    assert_eq!(v["status"], "ok", "{v}");
}

#[test]
fn progress_at_windows_bar() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let out = dir.path().join("pr.mp4");
    let v = run_json(&[
        "progress",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--at",
        "0.3",
        "--dur",
        "0.4",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
}

#[test]
fn audiogram_position_top() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let tone = dir.path().join("t.m4a");
    std::process::Command::new("ffmpeg")
        .args([
            "-v",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=1",
            "-c:a",
            "aac",
        ])
        .arg(&tone)
        .output()
        .unwrap();
    let out = dir.path().join("ag.mp4");
    let v = run_json(&[
        "audiogram",
        tone.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--position",
        "top",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
}

#[test]
fn subs_burn_font_flag() {
    if !has_ffmpeg() || !has_filter("subtitles") {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let srt = dir.path().join("s.srt");
    std::fs::write(&srt, "1\n00:00:00,0 --> 00:00:00,8\nHola\n").unwrap();
    let out = dir.path().join("sub.mp4");
    let v = run_json(&[
        "subs",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--burn",
        srt.to_str().unwrap(),
        "--font",
        "Helvetica",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
}

#[test]
fn zoom_out_reveals() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let out = dir.path().join("zo.mp4");
    let v = run_json(&[
        "zoom",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--out",
        "--factor",
        "2",
        "--at",
        "0.2",
        "--dur",
        "0.5",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
}

#[test]
fn title_shadow_renders() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let out = dir.path().join("sh.mp4");
    let v = run_json(&[
        "title",
        src.to_str().unwrap(),
        "--text",
        "SHADOW",
        "-o",
        out.to_str().unwrap(),
        "--shadow",
        "12",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
}

#[test]
fn extract_width_scales_still() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let out = dir.path().join("ex.png");
    let v = run_json(&[
        "extract",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--width",
        "160",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
}

#[test]
fn countdown_beep_adds_tone() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let out = dir.path().join("cd.mp4");
    let v = run_json(&[
        "countdown",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--beep",
        "--from",
        "2",
        "--each",
        "0.3",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
}

#[test]
fn leveler_preset_voice() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let tone = dir.path().join("t.m4a");
    std::process::Command::new("ffmpeg")
        .args([
            "-v",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=1",
            "-c:a",
            "aac",
        ])
        .arg(&tone)
        .output()
        .unwrap();
    let out = dir.path().join("lv.m4a");
    let v = run_json(&[
        "leveler",
        tone.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--preset",
        "voice",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
}

#[test]
fn spectrogram_color_scheme() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let tone = dir.path().join("t.m4a");
    std::process::Command::new("ffmpeg")
        .args([
            "-v",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=1",
            "-c:a",
            "aac",
        ])
        .arg(&tone)
        .output()
        .unwrap();
    let out = dir.path().join("sp.png");
    let v = run_json(&[
        "spectrogram",
        tone.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--color",
        "magma",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
}

#[test]
fn music_at_delays_bed() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let tone = dir.path().join("t.m4a");
    std::process::Command::new("ffmpeg")
        .args([
            "-v",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=1",
            "-c:a",
            "aac",
        ])
        .arg(&tone)
        .output()
        .unwrap();
    let out = dir.path().join("mu.mp4");
    let v = run_json(&[
        "music",
        src.to_str().unwrap(),
        "--track",
        tone.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--at",
        "0.3",
        "--dur",
        "0.4",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
}

#[test]
fn channel_invert_flips_side() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let tone = dir.path().join("st.m4a");
    std::process::Command::new("ffmpeg")
        .args([
            "-v",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=1[a];[a]pan=stereo|c0=c0|c1=c0",
            "-c:a",
            "aac",
        ])
        .arg(&tone)
        .output()
        .unwrap();
    let out = dir.path().join("ch.wav");
    let v = run_json(&[
        "channel",
        tone.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--mode",
        "invert",
        "--side",
        "left",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
}

#[test]
fn sheet_pad_margin() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let out = dir.path().join("sh.png");
    let v = run_json(&[
        "sheet",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--pad",
        "12",
        "--margin",
        "20",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
}

#[test]
fn overlay_angle_rotates_watermark() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let logo = dir.path().join("logo.png");
    std::process::Command::new("ffmpeg")
        .args([
            "-v",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "color=c=red:s=80x80:d=1",
            "-frames:v",
            "1",
        ])
        .arg(&logo)
        .output()
        .unwrap();
    let out = dir.path().join("ang.mp4");
    let v = run_json(&[
        "overlay",
        src.to_str().unwrap(),
        "--image",
        logo.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--angle",
        "25",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
}

#[test]
fn waveform_scale_log() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let tone = dir.path().join("t.m4a");
    std::process::Command::new("ffmpeg")
        .args([
            "-v",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=1",
            "-c:a",
            "aac",
        ])
        .arg(&tone)
        .output()
        .unwrap();
    let out = dir.path().join("wf.png");
    let v = run_json(&[
        "waveform",
        tone.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--scale",
        "log",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
}

#[test]
fn fx_tremolo_wobbles() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let tone = dir.path().join("t.m4a");
    std::process::Command::new("ffmpeg")
        .args([
            "-v",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=1",
            "-c:a",
            "aac",
        ])
        .arg(&tone)
        .output()
        .unwrap();
    let out = dir.path().join("fx.m4a");
    let v = run_json(&[
        "fx",
        tone.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--kind",
        "tremolo",
        "--strength",
        "0.8",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    assert_eq!(v["extra"]["effect"], "tremolo");
}

#[test]
fn boomerang_times_repeats_cycle() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let out = dir.path().join("bo.mp4");
    let v = run_json(&[
        "boomerang",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--times",
        "2",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let dur = v["probe"]["duration"].as_f64().unwrap_or(0.0);
    assert!(
        dur > 3.5,
        "boomerang --times 2 = 2 full fwd+rev cycles, got {dur}"
    );
}

#[test]
fn fx_window_ducks_dry_and_adds_wet() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let tone = dir.path().join("t.m4a");
    std::process::Command::new("ffmpeg")
        .args([
            "-v",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=1",
            "-c:a",
            "aac",
        ])
        .arg(&tone)
        .output()
        .unwrap();
    let out = dir.path().join("fxw.m4a");
    let v = run_json(&[
        "fx",
        tone.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--kind",
        "chorus",
        "--at",
        "0.3",
        "--dur",
        "0.4",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    assert_eq!(v["extra"]["effect"], "chorus");
}

#[test]
fn transcode_hevc_encodes() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let out = dir.path().join("h.mp4");
    let v = run_json(&[
        "transcode",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--preset",
        "hevc",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
}

#[test]
fn gate_preset_voice() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let tone = dir.path().join("t.m4a");
    std::process::Command::new("ffmpeg")
        .args([
            "-v",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=1",
            "-c:a",
            "aac",
        ])
        .arg(&tone)
        .output()
        .unwrap();
    let out = dir.path().join("g.m4a");
    let v = run_json(&[
        "gate",
        tone.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--preset",
        "voice",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    assert_eq!(v["extra"]["threshold_db"], -40.0);
}

#[test]
fn reverb_at_windows_the_tail() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let tone = dir.path().join("t.m4a");
    std::process::Command::new("ffmpeg")
        .args([
            "-v",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=1",
            "-c:a",
            "aac",
        ])
        .arg(&tone)
        .output()
        .unwrap();
    let out = dir.path().join("rv.m4a");
    let v = run_json(&[
        "reverb",
        tone.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--wet",
        "0.6",
        "--at",
        "0.3",
        "--dur",
        "0.4",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
}

#[test]
fn eq_at_windows_the_boost() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let tone = dir.path().join("t.m4a");
    std::process::Command::new("ffmpeg")
        .args([
            "-v",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=1",
            "-c:a",
            "aac",
        ])
        .arg(&tone)
        .output()
        .unwrap();
    let out = dir.path().join("eq.m4a");
    let v = run_json(&[
        "eq",
        tone.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--bass",
        "8",
        "--at",
        "0.3",
        "--dur",
        "0.4",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
}

#[test]
fn loop_section_repeats_only_the_middle() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let out = dir.path().join("lp.mp4");
    let v = run_json(&[
        "loop",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--from",
        "0.2",
        "--to",
        "0.6",
        "--times",
        "3",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    let dur = v["probe"]["duration"].as_f64().unwrap_or(0.0);
    // 0.2 head + 0.4x3 section + 0.4 tail = 1.8s from the 1s fixture
    assert!(dur > 1.5, "looped section should make ~1.8s, got {dur}");
}

#[test]
fn subs_mux_soft_subtitles() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let srt = dir.path().join("c.srt");
    std::fs::write(&srt, "1\n00:00:00,000 --> 00:00:00,800\nhello\n").unwrap();
    let out = dir.path().join("m.mp4");
    let v = run_json(&[
        "subs",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--mux",
        srt.to_str().unwrap(),
        "--lang",
        "spa",
    ]);
    assert_eq!(v["status"], "ok", "{v}");
    assert_eq!(v["extra"]["codec"], "mov_text");
}

#[test]
fn audiogram_custom_font() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let tone = dir.path().join("t.m4a");
    std::process::Command::new("ffmpeg")
        .args([
            "-v",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=1",
            "-c:a",
            "aac",
        ])
        .arg(&tone)
        .output()
        .unwrap();
    let out = dir.path().join("ag.mp4");
    let mut cmd = vec![
        "audiogram".to_string(),
        tone.to_string_lossy().to_string(),
        "-o".to_string(),
        out.to_string_lossy().to_string(),
        "--text".to_string(),
        "EP 1".to_string(),
    ];
    if std::path::Path::new("/System/Library/Fonts/Helvetica.ttc").exists() {
        cmd.push("--font".to_string());
        cmd.push("/System/Library/Fonts/Helvetica.ttc".to_string());
    }
    let refs: Vec<&str> = cmd.iter().map(String::as_str).collect();
    let v = run_json(&refs);
    assert_eq!(v["status"], "ok", "{v}");
}

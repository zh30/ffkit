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

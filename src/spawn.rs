use std::ffi::{OsStr, OsString};
use std::io::Read;
use std::process::{Command, Stdio};
use std::time::Duration;

use wait_timeout::ChildExt;

use crate::error::Error;

#[derive(Clone, Debug)]
pub struct Argv {
    pub program: String,
    pub args: Vec<OsString>,
}

impl Argv {
    pub fn ffmpeg() -> Self {
        Self {
            program: "ffmpeg".into(),
            args: vec![os("-hide_banner"), os("-nostdin")],
        }
    }

    pub fn ffprobe() -> Self {
        Self {
            program: "ffprobe".into(),
            args: vec![os("-hide_banner")],
        }
    }

    pub fn push(&mut self, a: impl AsRef<OsStr>) {
        self.args.push(a.as_ref().to_owned());
    }

    pub fn extend<I, S>(&mut self, iter: I)
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        for a in iter {
            self.push(a);
        }
    }

    pub fn display(&self) -> Vec<String> {
        let mut out = Vec::with_capacity(self.args.len() + 1);
        out.push(self.program.clone());
        for a in &self.args {
            out.push(a.to_string_lossy().into_owned());
        }
        out
    }
}

pub fn os(s: impl AsRef<OsStr>) -> OsString {
    s.as_ref().to_owned()
}

pub struct Spawned {
    pub status_ok: bool,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

pub fn run(argv: &Argv, timeout: Duration, progress: bool) -> Result<Spawned, Error> {
    let mut cmd = Command::new(&argv.program);
    cmd.args(&argv.args);
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    if progress {
        cmd.stderr(Stdio::inherit());
    } else {
        cmd.stderr(Stdio::piped());
    }

    let mut child = cmd.spawn().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            Error::missing_tool(format!(
                "{} not found on PATH; install ffmpeg (macOS: brew install ffmpeg)",
                argv.program
            ))
        } else {
            Error::ffmpeg(format!("failed to spawn {}: {e}", argv.program))
        }
    })?;

    let mut stdout_pipe = child.stdout.take();
    let mut stderr_pipe = child.stderr.take();

    let status = match child.wait_timeout(timeout).map_err(Error::from)? {
        Some(st) => st,
        None => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(Error::timeout(format!(
                "{} exceeded {}s",
                argv.program,
                timeout.as_secs()
            )));
        }
    };

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    if let Some(mut p) = stdout_pipe.take() {
        let _ = p.read_to_end(&mut stdout);
    }
    if let Some(mut p) = stderr_pipe.take() {
        let _ = p.read_to_end(&mut stderr);
    }

    Ok(Spawned {
        status_ok: status.success(),
        stdout,
        stderr,
    })
}

pub fn require_ok(argv: &Argv, spawned: Spawned) -> Result<Spawned, Error> {
    if spawned.status_ok {
        return Ok(spawned);
    }
    let err = String::from_utf8_lossy(&spawned.stderr);
    let tail = tail_lines(&err, 24);
    let msg = if tail.is_empty() {
        format!("{} exited non-zero", argv.program)
    } else {
        format!("{}: {tail}", argv.program)
    };
    Err(Error::ffmpeg(msg))
}

pub fn stdout_str(spawned: &Spawned) -> Result<&str, Error> {
    std::str::from_utf8(&spawned.stdout).map_err(|_| Error::ffmpeg("tool wrote non-utf8 stdout"))
}

pub fn stderr_str(spawned: &Spawned) -> String {
    String::from_utf8_lossy(&spawned.stderr).into_owned()
}

fn tail_lines(s: &str, n: usize) -> String {
    let lines: Vec<&str> = s.lines().filter(|l| !l.trim().is_empty()).collect();
    let start = lines.len().saturating_sub(n);
    lines[start..].join("\n")
}

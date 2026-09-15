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

use crate::cli::{CutArgs, Globals};
use crate::contract::Contract;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::time::{fmt_time, parse_time};

pub fn run(args: CutArgs, g: &Globals) -> Result<Contract, Error> {
    let start = args
        .start
        .as_deref()
        .map(parse_time)
        .transpose()?
        .unwrap_or(0.0);
    let end = args.end.as_deref().map(parse_time).transpose()?;
    let duration = args.duration.as_deref().map(parse_time).transpose()?;

    if args.start.is_none() && args.end.is_none() && args.duration.is_none() {
        return Err(Error::input("cut needs --start, --end, and/or --duration"));
    }

    let dur = match (end, duration) {
        (Some(e), Some(d)) => {
            if (e - start - d).abs() > 0.02 {
                return Err(Error::input(
                    "--end and --duration disagree; pass only one of them",
                ));
            }
            Some(d)
        }
        (Some(e), None) => {
            if e <= start {
                return Err(Error::input("--end must be after --start"));
            }
            Some(e - start)
        }
        (None, Some(d)) => Some(d),
        (None, None) => None,
    };

    let mut argv = ffmpeg_base(g.progress);
    if args.accurate {
        argv.push("-i");
        argv.push(&args.input);
        if start > 0.0 {
            argv.extend(["-ss", &fmt_time(start)]);
        }
        if let Some(d) = dur {
            argv.extend(["-t", &fmt_time(d)]);
        }
        argv.extend([
            "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-c:a", "aac",
        ]);
    } else {
        if start > 0.0 {
            argv.extend(["-ss", &fmt_time(start)]);
        }
        argv.push("-i");
        argv.push(&args.input);
        if let Some(d) = dur {
            argv.extend(["-t", &fmt_time(d)]);
        }
        argv.extend(["-c", "copy", "-avoid_negative_ts", "make_zero"]);
    }
    argv.push(&args.output);

    engine::write_job("cut", &[&args.input], &args.output, vec![argv], g)
}

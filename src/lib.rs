pub mod batch;
pub mod cli;
pub mod contract;
pub mod doctor;
pub mod embed;
pub mod engine;
pub mod error;
pub mod ffmpeg_raw;
pub mod font;
pub mod graph;
pub mod install;
pub mod look;
pub mod paths;
pub mod pipeline;
pub mod probe;
pub mod raster;
pub mod silence;
pub mod spawn;
pub mod srt;
pub mod time;
pub mod verbs;
pub mod version;

use clap::CommandFactory;

use crate::cli::{Cli, Cmd, Globals};
use crate::contract::Contract;
use crate::error::Error;

pub fn run(cli: Cli) -> Result<Contract, Error> {
    let g = Globals::from(&cli);
    match cli.cmd {
        Cmd::Doctor => doctor::run(&g),
        Cmd::Probe { input } => {
            let p = probe::probe(&input, g.timeout)?;
            Ok(Contract::ok("probe", Some(paths::display(&input)), Some(p)))
        }
        Cmd::Look(args) => look::run(args, &g),
        Cmd::Cut(args) => verbs::cut::run(args, &g),
        Cmd::Concat(args) => verbs::concat::run(args, &g),
        Cmd::Split(args) => verbs::split::run(args, &g),
        Cmd::Key(args) => verbs::key::run(args, &g),
        Cmd::Grid(args) => verbs::grid::run(args, &g),
        Cmd::Progress(args) => verbs::progress::run(args, &g),
        Cmd::Freeze(args) => verbs::freeze::run(args, &g),
        Cmd::Censor(args) => verbs::censor::run(args, &g),
        Cmd::Boomerang(args) => verbs::boomerang::run(args, &g),
        Cmd::Chapter(args) => verbs::chapter::run(args, &g),
        Cmd::Autocrop(args) => verbs::autocrop::run(args, &g),
        Cmd::Sheet(args) => verbs::sheet::run(args, &g),
        Cmd::Pitch(args) => verbs::pitch::run(args, &g),
        Cmd::Cutsil(args) => verbs::cutsil::run(args, &g),
        Cmd::Channel(args) => verbs::channel::run(args, &g),
        Cmd::Fit(args) => verbs::fit::run(args, &g),
        Cmd::Extract(args) => verbs::extract::run(args, &g),
        Cmd::Overlay(args) => verbs::overlay::run(args, &g),
        Cmd::Broll(args) => verbs::broll::run(args, &g),
        Cmd::Caption(args) => verbs::caption::run(args, &g),
        Cmd::Loudnorm(args) => verbs::loudnorm::run(args, &g),
        Cmd::Denoise(args) => verbs::denoise::run(args, &g),
        Cmd::Transcode(args) => verbs::transcode::run(args, &g),
        Cmd::Compress(args) => verbs::compress::run(args, &g),
        Cmd::Deliver(args) => verbs::deliver::run(args, &g),
        Cmd::Audiogram(args) => verbs::audiogram::run(args, &g),
        Cmd::Speed(args) => verbs::speed::run(args, &g),
        Cmd::Music(args) => verbs::music::run(args, &g),
        Cmd::Replace(args) => verbs::replace::run(args, &g),
        Cmd::Slideshow(args) => verbs::slideshow::run(args, &g),
        Cmd::Jumpcut(args) => verbs::jumpcut::run(args, &g),
        Cmd::Rough(args) => verbs::rough::run(args, &g),
        Cmd::Cover(args) => verbs::cover::run(args, &g),
        Cmd::Fade(args) => verbs::fade::run(args, &g),
        Cmd::Title(args) => verbs::title::run(args, &g),
        Cmd::Loop(args) => verbs::r#loop::run(args, &g),
        Cmd::Stabilize(args) => verbs::stabilize::run(args, &g),
        Cmd::Reverse(args) => verbs::reverse::run(args, &g),
        Cmd::Grade(args) => verbs::grade::run(args, &g),
        Cmd::Zoom(args) => verbs::zoom::run(args, &g),
        Cmd::Sharpen(args) => verbs::sharpen::run(args, &g),
        Cmd::Vignette(args) => verbs::vignette::run(args, &g),
        Cmd::Bw(args) => verbs::bw::run(args, &g),
        Cmd::Volume(args) => verbs::volume::run(args, &g),
        Cmd::Blur(args) => verbs::blur::run(args, &g),
        Cmd::Batch(args) => batch::run(args, &g),
        Cmd::Graph { plan } => graph::run(plan, &g),
        Cmd::Pipeline { plan } => pipeline::run(plan, &g),
        Cmd::Ffmpeg { because, args } => ffmpeg_raw::run(because, args, &g),
        Cmd::InstallSkill => install::run(g.dry_run),
        Cmd::Version { check } => version::run(check),
    }
}

pub fn clap_command() -> clap::Command {
    Cli::command()
}

pub fn verb_names() -> Vec<String> {
    clap_command()
        .get_subcommands()
        .map(|c| c.get_name().to_string())
        .collect()
}

pub mod batch;
pub mod cli;
pub mod color;
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
pub mod scene;
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
        Cmd::Insert(args) => verbs::insert::run(args, &g),
        Cmd::Progress(args) => verbs::progress::run(args, &g),
        Cmd::Freeze(args) => verbs::freeze::run(args, &g),
        Cmd::Censor(args) => verbs::censor::run(args, &g),
        Cmd::Crop(args) => verbs::crop::run(args, &g),
        Cmd::Vdenoise(args) => verbs::vdenoise::run(args, &g),
        Cmd::Waveform(args) => verbs::waveform::run(args, &g),
        Cmd::Meter(args) => verbs::meter::run(args, &g),
        Cmd::Spectrogram(args) => verbs::spectrogram::run(args, &g),
        Cmd::Dehum(args) => verbs::dehum::run(args, &g),
        Cmd::Tempo(args) => verbs::tempo::run(args, &g),
        Cmd::Silence(args) => verbs::silence::run(args, &g),
        Cmd::Vocal(args) => verbs::vocal::run(args, &g),
        Cmd::Remux(args) => verbs::remux::run(args, &g),
        Cmd::Meme(args) => verbs::meme::run(args, &g),
        Cmd::Voice(args) => verbs::voice::run(args, &g),
        Cmd::Deinterlace(args) => verbs::deinterlace::run(args, &g),
        Cmd::Crossfade(args) => verbs::crossfade::run(args, &g),
        Cmd::Strip(args) => verbs::strip::run(args, &g),
        Cmd::Frames(args) => verbs::frames::run(args, &g),
        Cmd::Invert(args) => verbs::invert::run(args, &g),
        Cmd::Countdown(args) => verbs::countdown::run(args, &g),
        Cmd::Mix(args) => verbs::mix::run(args, &g),
        Cmd::Timer(args) => verbs::timer::run(args, &g),
        Cmd::Multicam(args) => verbs::multicam::run(args, &g),
        Cmd::Mute(args) => verbs::mute::run(args, &g),
        Cmd::Hls(args) => verbs::hls::run(args, &g),
        Cmd::Qa(args) => verbs::qa::run(args, &g),
        Cmd::Conform(args) => verbs::conform::run(args, &g),
        Cmd::Scroll(args) => verbs::scroll::run(args, &g),
        Cmd::Sync(args) => verbs::sync::run(args, &g),
        Cmd::Art(args) => verbs::art::run(args, &g),
        Cmd::Leveler(args) => verbs::leveler::run(args, &g),
        Cmd::Gate(args) => verbs::gate::run(args, &g),
        Cmd::Boomerang(args) => verbs::boomerang::run(args, &g),
        Cmd::Fx(args) => verbs::fx::run(args, &g),
        Cmd::Align(args) => verbs::align::run(args, &g),
        Cmd::Chapter(args) => verbs::chapter::run(args, &g),
        Cmd::Autocrop(args) => verbs::autocrop::run(args, &g),
        Cmd::Sheet(args) => verbs::sheet::run(args, &g),
        Cmd::Sprite(args) => verbs::sprite::run(args, &g),
        Cmd::Pitch(args) => verbs::pitch::run(args, &g),
        Cmd::Cutsil(args) => verbs::cutsil::run(args, &g),
        Cmd::Channel(args) => verbs::channel::run(args, &g),
        Cmd::Eq(args) => verbs::eq::run(args, &g),
        Cmd::Reverb(args) => verbs::reverb::run(args, &g),
        Cmd::Bleep(args) => verbs::bleep::run(args, &g),
        Cmd::Rotate(args) => verbs::rotate::run(args, &g),
        Cmd::Delogo(args) => verbs::delogo::run(args, &g),
        Cmd::Meta(args) => verbs::meta::run(args, &g),
        Cmd::Subs(args) => verbs::subs::run(args, &g),
        Cmd::Thumb(args) => verbs::thumb::run(args, &g),
        Cmd::Solid(args) => verbs::solid::run(args, &g),
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
        Cmd::Trail(args) => verbs::trail::run(args, &g),
        Cmd::Glitch(args) => verbs::glitch::run(args, &g),
        Cmd::Bars(args) => verbs::bars::run(args, &g),
        Cmd::Scope(args) => verbs::scope::run(args, &g),
        Cmd::Desqueeze(args) => verbs::desqueeze::run(args, &g),
        Cmd::Solarize(args) => verbs::solarize::run(args, &g),
        Cmd::Pulse(args) => verbs::pulse::run(args, &g),
        Cmd::Deflicker(args) => verbs::deflicker::run(args, &g),
        Cmd::Emboss(args) => verbs::emboss::run(args, &g),
        Cmd::Tilt(args) => verbs::tilt::run(args, &g),
        Cmd::Sway(args) => verbs::sway::run(args, &g),
        Cmd::Rack(args) => verbs::rack::run(args, &g),
        Cmd::Outline(args) => verbs::outline::run(args, &g),
        Cmd::Night(args) => verbs::night::run(args, &g),
        Cmd::Snow(args) => verbs::snow::run(args, &g),
        Cmd::Impact(args) => verbs::impact::run(args, &g),
        Cmd::Wave(args) => verbs::wave::run(args, &g),
        Cmd::Spin(args) => verbs::spin::run(args, &g),
        Cmd::Iris(args) => verbs::iris::run(args, &g),
        Cmd::Burst(args) => verbs::burst::run(args, &g),
        Cmd::Thump(args) => verbs::accent::thump(args, &g),
        Cmd::Riser(args) => verbs::accent::riser(args, &g),
        Cmd::Whoosh(args) => verbs::accent::whoosh(args, &g),
        Cmd::Deesser(args) => verbs::deesser::run(args, &g),
        Cmd::Declip(args) => verbs::declip::run(args, &g),
        Cmd::Deband(args) => verbs::deband::run(args, &g),
        Cmd::Dedup(args) => verbs::dedup::run(args, &g),
        Cmd::Equalize(args) => verbs::equalize::run(args, &g),
        Cmd::Scan(args) => verbs::scan::run(args, &g),
        Cmd::Smooth(args) => verbs::smooth::run(args, &g),
        Cmd::Upscale(args) => verbs::upscale::run(args, &g),
        Cmd::Pick(args) => verbs::pick::run(args, &g),
        Cmd::Diff(args) => verbs::diff::run(args, &g),
        Cmd::Selective(args) => verbs::selective::run(args, &g),
        Cmd::Amplify(args) => verbs::amplify::run(args, &g),
        Cmd::Cartoon(args) => verbs::cartoon::run(args, &g),
        Cmd::Heat(args) => verbs::heat::run(args, &g),
        Cmd::Kaleido(args) => verbs::kaleido::run(args, &g),
        Cmd::Strobe(args) => verbs::strobe::run(args, &g),
        Cmd::Edge(args) => verbs::edge::run(args, &g),
        Cmd::Lens(args) => verbs::lens::run(args, &g),
        Cmd::Perspective(args) => verbs::perspective::run(args, &g),
        Cmd::Gen(args) => verbs::gen::run(args, &g),
        Cmd::Shear(args) => verbs::shear::run(args, &g),
        Cmd::V360(args) => verbs::v360::run(args, &g),
        Cmd::Wb(args) => verbs::wb::run(args, &g),
        Cmd::Mirror(args) => verbs::mirror::run(args, &g),
        Cmd::Pix(args) => verbs::pix::run(args, &g),
        Cmd::Flip(args) => verbs::flip::run(args, &g),
        Cmd::Poster(args) => verbs::poster::run(args, &g),
        Cmd::Duotone(args) => verbs::duotone::run(args, &g),
        Cmd::Glow(args) => verbs::glow::run(args, &g),
        Cmd::Vhs(args) => verbs::vhs::run(args, &g),
        Cmd::MotionBlur(args) => verbs::motionblur::run(args, &g),
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

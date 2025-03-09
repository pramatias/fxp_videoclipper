pub use clap::clap_derive::Subcommand;

// An enum for all possible modes
#[derive(Subcommand, Debug)]
pub enum Modes {
    Exporter,
    Merger,
    Sampler,
    Clutter,
    Clipper,
    Gmicer,
}

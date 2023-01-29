mod setup;
mod namespace;
mod cargo;
mod cargo_image;
mod resource;
mod version;
mod state;

pub use setup::*;
pub use namespace::*;
pub use cargo::*;
pub use cargo_image::*;
pub use resource::*;
pub use version::*;
pub use state::*;

use clap::{Parser, Subcommand};

/// A self-sufficient hybrid-cloud manager
#[derive(Debug, Parser)]
#[clap(about, version, name = "nanocl")]
pub struct Cli {
  /// Nanocld host
  #[clap(long, short = 'H', default_value = "unix://run/nanocl/nanocl.sock")]
  pub host: String,
  /// Commands
  #[clap(subcommand)]
  pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
  /// Manage namespaces
  Namespace(NamespaceArgs),
  /// Manage cargoes
  Cargo(CargoArgs),
  /// Manage resources
  Resource(ResourceArgs),
  /// Watch daemon events
  Events,
  /// Setup nanocl
  Setup(SetupArgs),
  /// Apply or Reverse a state from a configuration file
  State(StateArgs),
  /// Show nanocl version information
  Version(VersionArgs),
  // TODO: shell completion
  // Completion {
  //   /// Shell to generate completion for
  //   #[clap(arg_enum)]
  //   shell: Shell,
  // },
}

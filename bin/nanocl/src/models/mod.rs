use clap::{Parser, Subcommand, ValueEnum};
use nanocld_client::stubs::process::ProcessLogQuery;
use serde::{Deserialize, Serialize};

mod backup;
mod cargo;
mod context;
mod event;
mod generic;
mod install;
mod job;
mod metric;
mod namespace;
mod node;
mod process;
mod resource;
mod secret;
mod state;
mod uninstall;
mod version;
mod vm;
mod vm_image;

pub use backup::*;
pub use cargo::*;
pub use context::*;
pub use event::*;
pub use generic::*;
pub use install::*;
pub use job::*;
pub use metric::*;
pub use namespace::*;
pub use node::*;
pub use process::*;
pub use resource::*;
pub use secret::*;
pub use state::*;
pub use uninstall::*;
pub use vm::*;
pub use vm_image::*;

/// Cli available options and commands
#[derive(Parser)]
#[clap(about, version, name = "nanocl")]
pub struct Cli {
  /// Nanocld host default: unix://run/nanocl/nanocl.sock
  #[clap(long, short = 'H')]
  pub host: Option<String>,
  /// Commands
  #[clap(subcommand)]
  pub command: Command,
}

/// `nanocl logs` available options
#[derive(Clone, Parser)]
pub struct LogsOpts {
  /// Name of process to show logs
  pub name: String,
  /// Only include logs since unix timestamp
  #[clap(short = 's')]
  pub since: Option<i64>,
  /// Only include logs until unix timestamp
  #[clap(short = 'u')]
  pub until: Option<i64>,
  /// If integer only return last n logs, if "all" returns all logs
  #[clap(short = 't')]
  pub tail: Option<String>,
  /// Bool, if set include timestamp to ever log line
  #[clap(long = "timestamps")]
  pub timestamps: bool,
  /// Bool, if set open the log as stream
  #[clap(short = 'f')]
  pub follow: bool,
}

impl From<LogsOpts> for ProcessLogQuery {
  fn from(opts: LogsOpts) -> Self {
    Self {
      namespace: None,
      since: opts.since,
      until: opts.until,
      tail: opts.tail,
      timestamps: Some(opts.timestamps),
      follow: Some(opts.follow),
      stderr: Some(true),
      stdout: Some(true),
    }
  }
}

/// Nanocl available commands
#[derive(Subcommand)]
pub enum Command {
  /// Manage namespaces
  Namespace(NamespaceArg),
  /// Manage secrets
  Secret(SecretArg),
  /// Manage jobs
  Job(JobArg),
  /// Manage cargoes
  Cargo(CargoArg),
  /// Manage virtual machines
  Vm(VmArg),
  /// Manage resources
  Resource(ResourceArg),
  /// Manage metrics
  Metric(MetricArg),
  /// Manage contexts
  Context(ContextArg),
  /// Manage nodes (experimental)
  Node(NodeArg),
  /// Apply or Remove a Statefile
  State(StateArg),
  /// Show or watch events
  Event(EventArg),
  /// Show processes
  Ps(GenericListOpts<ProcessFilter>),
  /// Show nanocl host information
  Info,
  /// Get logs of a process
  Logs(LogsOpts),
  /// Show nanocl version information
  Version,
  /// Install components
  Install(InstallOpts),
  /// Uninstall components
  Uninstall(UninstallOpts),
  /// Backup the current state
  Backup(BackupOpts),
  // TODO: shell completion
  // Completion {
  //   /// Shell to generate completion for
  //   #[clap(arg_enum)]
  //   shell: Shell,
  // },
}

/// `nanocl` available display formats `yaml` by default
#[derive(Default, Clone, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "PascalCase")]
pub enum DisplayFormat {
  #[default]
  Yaml,
  Toml,
  Json,
}

impl std::fmt::Display for DisplayFormat {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let data = match self {
      Self::Yaml => "yaml",
      Self::Toml => "toml",
      Self::Json => "json",
    };
    write!(f, "{data}")
  }
}

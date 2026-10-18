use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "oxide",
    version,
    about = "A fast, simple system package manager with a git-backed registry"
)]
pub struct Cli {
    /// Override the registry URL or local path
    #[arg(long, global = true, env = "OXIDE_REGISTRY")]
    pub registry: Option<String>,

    /// Operate under an alternate root (for testing; default: /)
    #[arg(long, global = true, env = "OXIDE_ROOT")]
    pub root: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Sync the package registry from the repo
    Sync,

    /// Install packages
    #[command(alias = "i")]
    Install { packages: Vec<String> },

    /// Detain (uninstall) packages
    #[command(alias = "remove", alias = "rm", alias = "uninstall")]
    Detain {
        packages: Vec<String>,

        /// Detain even if other packages depend on them
        #[arg(long, short)]
        force: bool,
    },

    /// Upgrade installed packages
    #[command(alias = "upgrade", alias = "u")]
    Update {
        packages: Vec<String>,

        /// Sync the registry before updating
        #[arg(long)]
        sync: bool,
    },

    /// Search the registry
    #[command(alias = "s")]
    Search { query: String },

    /// Show package details
    Info { package: String },

    /// List installed packages
    #[command(alias = "l")]
    List,

    /// List installed packages with updates available
    Outdated,

    /// List files owned by a package
    Files { package: String },

    /// Verify integrity of installed packages
    Verify { packages: Vec<String> },

    /// Explain why a package is installed
    Why { package: String },

    /// Show a package's dependency tree (from the registry)
    Deps { package: String },

    /// List orphaned packages (dependencies nothing needs anymore)
    Orphans,

    /// Hold packages — skip them during `oxide update`
    Hold { packages: Vec<String> },

    /// Release held packages
    Unhold { packages: Vec<String> },

    /// Show install history
    Log,

    /// Show disk usage per package
    Stats,

    /// Print explicitly installed packages (for backup)
    Export,

    /// Install every package listed in a file (from `oxide export`)
    Import { file: PathBuf },

    /// Check installed packages for consistency problems
    Check,

    /// Remove cached data
    Clean,
}

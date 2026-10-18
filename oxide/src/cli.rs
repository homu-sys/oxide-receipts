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
    /// Sync the package registry
    Sync,

    /// Install packages
    #[command(alias = "i")]
    Install { packages: Vec<String> },

    /// Remove installed packages
    #[command(alias = "rm")]
    Remove {
        packages: Vec<String>,

        /// Remove even if other packages depend on them
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

    /// Remove cached data
    Clean,
}

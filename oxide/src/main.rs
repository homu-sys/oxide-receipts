mod cli;
mod config;
mod extract;
mod install;
mod manifest;
mod paths;
mod query;
mod receipt;
mod registry;
mod remove;
mod term;
mod update;
mod util;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, ColorChoice, Command};
use paths::Paths;
use std::process::Command as SysCommand;

/// Translate pacman-style flags into oxide subcommands wherever they appear.
fn translate_argv(args: Vec<String>) -> Vec<String> {
    if args.len() < 2 {
        return args;
    }
    // Find the first single-dash token (global options use --long form).
    let Some(idx) = args
        .iter()
        .skip(1)
        .position(|a| a.starts_with('-') && !a.starts_with("--"))
        .map(|p| p + 1)
    else {
        return args;
    };
    let flag = args[idx].clone();
    let rest: Vec<String> = args[idx + 1..].to_vec();
    let mut out = args[..idx].to_vec();
    let mut map = |sub: &str, extra: &[&str]| {
        out.push(sub.to_string());
        out.extend(extra.iter().map(|s| s.to_string()));
        out.extend(rest.iter().cloned());
    };
    match flag.as_str() {
        "-S" => map("install", &[]),
        "-Sy" => map("sync", &[]),
        "-Syu" => map("update", &["--sync"]),
        "-Su" => map("update", &[]),
        "-Ss" => map("search", &[]),
        "-Sl" => map("request", &[]),
        "-Si" => map("info", &[]),
        "-Sc" => map("clean", &[]),
        "-R" | "-Rn" | "-Rns" => map("detain", &[]),
        "-Q" => map("list", &[]),
        "-Qi" => map("info", &[]),
        "-Ql" => map("files", &[]),
        "-Qu" => map("outdated", &[]),
        _ => return args,
    }
    out
}

fn needs_root(command: &Command) -> bool {
    matches!(
        command,
        Command::Sync { .. }
            | Command::Install { .. }
            | Command::Detain { .. }
            | Command::Update { .. }
            | Command::Hold { .. }
            | Command::Unhold { .. }
            | Command::Import { .. }
            | Command::Clean { .. }
    )
}

fn reexec_with_sudo() -> ! {
    let exe = std::env::current_exe().expect("current exe");
    let args: Vec<String> = std::env::args().skip(1).collect();
    let status = SysCommand::new("sudo")
        .arg(exe)
        .args(args)
        .status()
        .expect("failed to run sudo — install it or run oxide as root");
    std::process::exit(status.code().unwrap_or(1));
}

fn run(cli: &Cli) -> Result<()> {
    let paths = Paths::new(cli.root.clone());

    if needs_root(&cli.command) && paths.is_system() && util::euid() != 0 {
        eprintln!("{} re-running with sudo...", term::yellow("root required —"));
        reexec_with_sudo();
    }

    let registry_url =
        config::resolve_registry_url(cli.registry.as_deref(), &paths.config_file());

    match &cli.command {
        Command::Sync => registry::Registry::sync(&paths, &registry_url),

        Command::Install { packages } => {
            if packages.is_empty() {
                anyhow::bail!("no packages specified");
            }
            let registry = registry::Registry::load(&paths)?;
            install::install_packages(&paths, &registry, packages)
        }

        Command::Detain { packages, force } => {
            if packages.is_empty() {
                anyhow::bail!("no packages specified");
            }
            remove::remove_packages(&paths, packages, *force)
        }

        Command::Update { packages, sync } => {
            if *sync {
                registry::Registry::sync(&paths, &registry_url)?;
            }
            let registry = registry::Registry::load(&paths)?;
            update::update_packages(&paths, &registry, packages)
        }

        Command::Search { query } => {
            let registry = registry::Registry::load(&paths)?;
            query::search(&registry, query)
        }

        Command::Request => {
            let registry = registry::Registry::load(&paths)?;
            query::request(&registry)
        }

        Command::Info { package } => {
            let registry = registry::Registry::load(&paths)?;
            query::info(&paths, &registry, package)
        }

        Command::List => query::list_installed(&paths),

        Command::Outdated => {
            let registry = registry::Registry::load(&paths)?;
            query::outdated(&paths, &registry)
        }

        Command::Files { package } => query::files(&paths, package),

        Command::Owner { path } => query::owner(&paths, path),

        Command::Verify { packages } => query::verify(&paths, packages),

        Command::Why { package } => query::why(&paths, package),

        Command::Deps { package } => {
            let registry = registry::Registry::load(&paths)?;
            query::deps(&registry, package)
        }

        Command::Tree { package } => query::tree(&paths, package.as_deref()),

        Command::Orphans => query::orphans(&paths),

        Command::Hold { packages } => {
            if packages.is_empty() {
                anyhow::bail!("no packages specified");
            }
            query::set_hold(&paths, packages, true)
        }

        Command::Unhold { packages } => {
            if packages.is_empty() {
                anyhow::bail!("no packages specified");
            }
            query::set_hold(&paths, packages, false)
        }

        Command::Log => query::log(&paths),

        Command::Stats => query::stats(&paths),

        Command::Export => query::export(&paths),

        Command::Import { file } => {
            let registry = registry::Registry::load(&paths)?;
            query::import(&paths, &registry, file)
        }

        Command::Check => query::check(&paths),

        Command::Clean => query::clean(&paths),

        Command::Doctor => query::doctor(&paths),

        Command::Completions { shell } => {
            cli::print_completions(*shell);
            Ok(())
        }
    }
}

/// Let `--color` also steer clap's own `--help`/error coloring, which is
/// rendered before our owo-colors override is installed.
fn prescan_color(args: &[String]) {
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        let value = if a == "--color" {
            i += 1;
            args.get(i).map(String::as_str)
        } else {
            a.strip_prefix("--color=")
        };
        match value {
            Some("never") => std::env::set_var("NO_COLOR", "1"),
            Some("always") => {
                std::env::remove_var("NO_COLOR");
                std::env::set_var("CLICOLOR_FORCE", "1");
            }
            _ => {}
        }
        i += 1;
    }
}

fn main() {
    let args = translate_argv(std::env::args().collect());
    prescan_color(&args);
    let cli = Cli::parse_from(args);

    match cli.color {
        ColorChoice::Always => owo_colors::set_override(true),
        ColorChoice::Never => owo_colors::set_override(false),
        ColorChoice::Auto => owo_colors::unset_override(),
    }

    if let Err(err) = run(&cli) {
        eprintln!("{} {err:#}", term::error("error:"));
        std::process::exit(1);
    }
}

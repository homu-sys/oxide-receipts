use anyhow::{Context, Result};
use owo_colors::OwoColorize;

use crate::paths::Paths;
use crate::receipt::Receipt;
use crate::registry::Registry;
use crate::util;

pub fn list_installed(paths: &Paths) -> Result<()> {
    let receipts = Receipt::load_all(paths)?;
    if receipts.is_empty() {
        println!("no packages installed");
        return Ok(());
    }
    let width = receipts
        .iter()
        .map(|r| r.package.name.len())
        .max()
        .unwrap_or(0);
    for r in &receipts {
        let padded = format!("{:width$}", r.package.name, width = width);
        println!("{}  {}", padded.bold(), r.package.version);
    }
    println!("\n{} package(s) installed", receipts.len());
    Ok(())
}

pub fn outdated(paths: &Paths, registry: &Registry) -> Result<()> {
    let installed = Receipt::load_all(paths)?;
    let mut found = false;
    for r in &installed {
        if let Ok(m) = registry.get(&r.package.name) {
            if util::version_gt(&m.package.version, &r.package.version) {
                println!(
                    "{:width$}  {} → {}",
                    r.package.name,
                    r.package.version,
                    m.package.version.green(),
                    width = r.package.name.len().max(12)
                );
                found = true;
            }
        }
    }
    if !found {
        println!("everything is up to date");
    }
    Ok(())
}

pub fn search(registry: &Registry, query: &str) -> Result<()> {
    let query = query.to_lowercase();
    let mut matches: Vec<_> = registry
        .all()
        .filter(|m| {
            m.package.name.to_lowercase().contains(&query)
                || m.package
                    .description
                    .as_deref()
                    .is_some_and(|d| d.to_lowercase().contains(&query))
        })
        .collect();
    matches.sort_by(|a, b| a.package.name.cmp(&b.package.name));

    for m in matches {
        println!(
            "{} {} {}",
            m.package.name.bold(),
            m.package.version,
            m.package.description.as_deref().unwrap_or("").dimmed()
        );
    }
    Ok(())
}

pub fn info(paths: &Paths, registry: &Registry, name: &str) -> Result<()> {
    let manifest = registry.get(name)?;
    let p = &manifest.package;

    println!("{}", p.name.bold());
    println!("  version:      {}", p.version);
    if let Some(desc) = &p.description {
        println!("  description:  {desc}");
    }
    if let Some(home) = &p.homepage {
        println!("  homepage:     {home}");
    }
    if let Some(license) = &p.license {
        println!("  license:      {license}");
    }
    if !p.arch.is_empty() {
        println!("  architectures: {}", p.arch.join(", "));
    }
    if !manifest.dependencies.depends.is_empty() {
        println!("  depends on:   {}", manifest.dependencies.depends.join(", "));
    }
    println!("  archive:      {}", manifest.archive.file);

    match Receipt::load(paths, name) {
        Ok(r) => println!(
            "  status:       {} ({})",
            "installed".green(),
            r.package.version
        ),
        Err(_) => println!("  status:       {}", "not installed".dimmed()),
    }
    Ok(())
}

pub fn files(paths: &Paths, name: &str) -> Result<()> {
    let receipt = Receipt::load(paths, name)?;
    for link in &receipt.links {
        println!("{} {}", "link".cyan(), link.path);
    }
    for file in &receipt.files {
        println!("{} {}/{}", "file".dimmed(), receipt.prefix, file.path);
    }
    Ok(())
}

pub fn verify(paths: &Paths, names: &[String]) -> Result<()> {
    let installed = Receipt::load_all(paths)?;
    let targets: Vec<&Receipt> = if names.is_empty() {
        installed.iter().collect()
    } else {
        names
            .iter()
            .map(|n| {
                installed
                    .iter()
                    .find(|r| &r.package.name == n)
                    .with_context(|| format!("package '{n}' is not installed"))
            })
            .collect::<Result<Vec<_>>>()?
    };

    let mut problems = 0usize;
    for receipt in targets {
        let mut ok = true;
        for file in &receipt.files {
            let path = std::path::Path::new(&receipt.prefix).join(&file.path);
            match util::sha256_file(&path) {
                Ok(hash) if hash == file.sha256 => {}
                Ok(_) => {
                    println!(
                        "  {} {name}: modified: {}",
                        "!!".red(),
                        file.path,
                        name = receipt.package.name
                    );
                    ok = false;
                }
                Err(_) => {
                    println!(
                        "  {} {name}: missing: {}",
                        "!!".red(),
                        file.path,
                        name = receipt.package.name
                    );
                    ok = false;
                }
            }
        }
        for link in &receipt.links {
            let target = std::fs::read_link(&link.path);
            match target {
                Ok(t) if t.to_string_lossy() == link.target => {}
                _ => {
                    println!(
                        "  {} {name}: broken link: {}",
                        "!!".red(),
                        link.path,
                        name = receipt.package.name
                    );
                    ok = false;
                }
            }
        }
        if ok {
            println!(
                "{} {} {}",
                "ok".green().bold(),
                receipt.package.name,
                receipt.package.version
            );
        } else {
            problems += 1;
        }
    }

    if problems > 0 {
        anyhow::bail!("{problems} package(s) failed verification");
    }
    Ok(())
}

pub fn clean(paths: &Paths) -> Result<()> {
    let staging = paths.staging();
    if staging.exists() {
        std::fs::remove_dir_all(&staging)?;
        println!("cleaned {}", staging.display());
    } else {
        println!("nothing to clean");
    }
    Ok(())
}

use anyhow::{Context, Result};
use owo_colors::OwoColorize;
use std::path::Path;

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

pub fn why(paths: &Paths, name: &str) -> Result<()> {
    let receipts = Receipt::load_all(paths)?;
    let receipt = receipts
        .iter()
        .find(|r| r.package.name == name)
        .with_context(|| format!("package '{name}' is not installed"))?;

    let dependents: Vec<&Receipt> = receipts
        .iter()
        .filter(|r| r.depends.iter().any(|d| d == name))
        .collect();

    if !dependents.is_empty() {
        println!("'{name}' is required by:");
        for d in &dependents {
            println!("  {} {} {}", d.package.name.bold(), d.package.version, "← explicit dependency".dimmed());
        }
    } else if receipt.explicit {
        println!("'{name}' was explicitly installed");
    } else {
        println!(
            "'{name}' was installed as a dependency, but nothing depends on it anymore (orphan)"
        );
    }
    if !receipt.depends.is_empty() {
        println!("'{name}' itself depends on: {}", receipt.depends.join(", "));
    }
    Ok(())
}

pub fn deps(registry: &Registry, name: &str) -> Result<()> {
    let manifest = registry.get(name)?;
    println!("{}", manifest.package.name.bold());
    print_deps(registry, name, "  ", &mut Vec::new())?;
    Ok(())
}

fn print_deps(registry: &Registry, name: &str, indent: &str, seen: &mut Vec<String>) -> Result<()> {
    if seen.iter().any(|s| s == name) {
        println!("{indent}{} (cycle)", name.dimmed());
        return Ok(());
    }
    seen.push(name.to_string());
    let manifest = registry.get(name)?;
    for dep in &manifest.dependencies.depends {
        let m = registry.get(dep)?;
        println!("{indent}{dep} {}", m.package.version);
        print_deps(registry, dep, &format!("{indent}  "), seen)?;
    }
    seen.pop();
    Ok(())
}

pub fn orphans(paths: &Paths) -> Result<()> {
    let receipts = Receipt::load_all(paths)?;
    let mut found = false;
    for r in &receipts {
        if r.explicit {
            continue;
        }
        let required = receipts
            .iter()
            .any(|x| x.depends.iter().any(|d| d == &r.package.name));
        if !required {
            println!(
                "{} {} {}",
                r.package.name.bold(),
                r.package.version,
                "(orphan)".dimmed()
            );
            found = true;
        }
    }
    if !found {
        println!("no orphaned packages");
    }
    Ok(())
}

pub fn set_hold(paths: &Paths, names: &[String], hold: bool) -> Result<()> {
    for name in names {
        let mut receipt = Receipt::load(paths, name)?;
        receipt.hold = hold;
        Receipt::save(paths, &receipt)?;
        if hold {
            println!(
                "{} {} {} — skipped by `oxide update`",
                "held".yellow().bold(),
                name,
                receipt.package.version
            );
        } else {
            println!(
                "{} {} {}",
                "unheld".green(),
                name,
                receipt.package.version
            );
        }
    }
    Ok(())
}

pub fn log(paths: &Paths) -> Result<()> {
    let mut receipts = Receipt::load_all(paths)?;
    receipts.sort_by(|a, b| a.package.installed_at.cmp(&b.package.installed_at));
    if receipts.is_empty() {
        println!("no install history");
        return Ok(());
    }
    for r in &receipts {
        let held = if r.hold { " [held]" } else { "" };
        let kind = if r.explicit { "" } else { " (dep)" };
        println!(
            "{}  {} {}{}{}",
            r.package.installed_at.dimmed(),
            r.package.name.bold(),
            r.package.version,
            kind,
            held
        );
    }
    Ok(())
}

fn human_size(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    let b = bytes as f64;
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KiB", b / KIB)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MiB", b / KIB / KIB)
    } else {
        format!("{:.2} GiB", b / KIB / KIB / KIB)
    }
}

pub fn stats(paths: &Paths) -> Result<()> {
    use walkdir::WalkDir;

    let receipts = Receipt::load_all(paths)?;
    let mut sizes: Vec<(String, String, u64)> = Vec::new();
    let mut total = 0u64;
    for r in &receipts {
        let mut size = 0u64;
        for entry in WalkDir::new(&r.prefix).follow_links(false).into_iter().flatten() {
            if entry.file_type().is_file() {
                size += entry.metadata().map(|m| m.len()).unwrap_or(0);
            }
        }
        total += size;
        sizes.push((r.package.name.clone(), r.package.version.clone(), size));
    }
    sizes.sort_by(|a, b| b.2.cmp(&a.2));
    let width = sizes.iter().map(|(n, ..)| n.len()).max().unwrap_or(0);
    for (name, version, size) in &sizes {
        println!(
            "{:width$}  {version:>10}  {:>10}",
            name.bold(),
            human_size(*size),
            width = width
        );
    }
    println!(
        "{}{}  {:>10}",
        "total".bold(),
        " ".repeat(width.saturating_sub(5)),
        human_size(total)
    );
    Ok(())
}

pub fn export(paths: &Paths) -> Result<()> {
    let receipts = Receipt::load_all(paths)?;
    for r in receipts.iter().filter(|r| r.explicit) {
        println!("{}", r.package.name);
    }
    Ok(())
}

pub fn import(paths: &Paths, registry: &Registry, file: &Path) -> Result<()> {
    let content = std::fs::read_to_string(file)
        .with_context(|| format!("reading {}", file.display()))?;
    let names: Vec<String> = content
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| l.to_string())
        .collect();
    if names.is_empty() {
        anyhow::bail!("no packages listed in {}", file.display());
    }
    println!("importing {} package(s)", names.len());
    crate::install::install_packages(paths, registry, &names)
}

pub fn check(paths: &Paths) -> Result<()> {
    let installed = Receipt::load_all(paths)?;
    let mut problems = 0usize;

    for receipt in &installed {
        let mut pkg_problems = 0usize;

        for dep in &receipt.depends {
            if !installed.iter().any(|r| &r.package.name == dep) {
                println!(
                    "  {} {}: missing dependency '{dep}'",
                    "!!".red(),
                    receipt.package.name
                );
                pkg_problems += 1;
            }
        }
        for file in &receipt.files {
            let path = std::path::Path::new(&receipt.prefix).join(&file.path);
            match util::sha256_file(&path) {
                Ok(hash) if hash == file.sha256 => {}
                _ => pkg_problems += 1,
            }
        }
        for link in &receipt.links {
            let ok = std::fs::read_link(&link.path)
                .is_ok_and(|t| t.to_string_lossy() == link.target);
            if !ok {
                pkg_problems += 1;
            }
        }

        if pkg_problems > 0 {
            println!(
                "{} {} {}: {pkg_problems} problem(s)",
                "!!".red(),
                receipt.package.name,
                receipt.package.version
            );
            problems += pkg_problems;
        }
    }

    if problems > 0 {
        anyhow::bail!("{problems} problem(s) found — try reinstalling affected packages");
    }
    println!(
        "{} {} package(s) consistent",
        "ok".green().bold(),
        installed.len()
    );
    Ok(())
}

use anyhow::{bail, Context, Result};
use owo_colors::OwoColorize;
use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::extract;
use crate::manifest::Manifest;
use crate::receipt::{FileEntry, LinkEntry, Receipt, ReceiptPackage};
use crate::registry::Registry;
use crate::paths::Paths;
use crate::util;
use walkdir::WalkDir;

pub fn install_packages(paths: &Paths, registry: &Registry, names: &[String]) -> Result<()> {
    let installed: HashMap<String, Receipt> = Receipt::load_all(paths)?
        .into_iter()
        .map(|r| (r.package.name.clone(), r))
        .collect();

    let plan = resolve_plan(registry, names, &installed)?;
    if plan.is_empty() {
        println!("nothing to do — all requested packages are up to date");
        return Ok(());
    }

    for name in &plan {
        let manifest = registry.get(name)?;
        if !manifest.supports_current_arch() {
            bail!(
                "package '{name}' does not support architecture {}",
                util::current_arch()
            );
        }
        match installed.get(name) {
            Some(receipt) if receipt.package.version == manifest.package.version => {
                println!(" {} {name} is up to date", "=" .dimmed());
            }
            Some(receipt) => {
                println!(
                    "upgrading {} {} → {}",
                    name,
                    receipt.package.version,
                    manifest.package.version
                );
                install_one(paths, registry, manifest)?;
            }
            None => {
                install_one(paths, registry, manifest)?;
            }
        }
    }
    Ok(())
}

fn resolve_plan(
    registry: &Registry,
    targets: &[String],
    installed: &HashMap<String, Receipt>,
) -> Result<Vec<String>> {
    let mut plan = Vec::new();
    let mut visited = HashSet::new();
    let mut visiting = HashSet::new();

    for target in targets {
        visit(registry, target, installed, &mut visiting, &mut visited, &mut plan)?;
    }
    Ok(plan)
}

fn visit(
    registry: &Registry,
    name: &str,
    installed: &HashMap<String, Receipt>,
    visiting: &mut HashSet<String>,
    visited: &mut HashSet<String>,
    plan: &mut Vec<String>,
) -> Result<()> {
    if visited.contains(name) {
        return Ok(());
    }
    if !visiting.insert(name.to_string()) {
        bail!("dependency cycle detected involving '{name}'");
    }

    let manifest = registry.get(name)?;
    for dep in &manifest.dependencies.depends {
        visit(registry, dep, installed, visiting, visited, plan)?;
    }

    visiting.remove(name);
    visited.insert(name.to_string());

    let up_to_date = installed
        .get(name)
        .is_some_and(|r| r.package.version == manifest.package.version);
    if !up_to_date {
        plan.push(name.to_string());
    }
    Ok(())
}

fn install_one(paths: &Paths, registry: &Registry, manifest: &Manifest) -> Result<()> {
    let name = &manifest.package.name;
    let archive_path = registry.package_dir(name).join(&manifest.archive.file);
    if !archive_path.exists() {
        bail!(
            "archive {} missing from registry — re-run `oxide sync`",
            manifest.archive.file
        );
    }

    let staging = paths.staging().join(name);
    if staging.exists() {
        std::fs::remove_dir_all(&staging)?;
    }
    std::fs::create_dir_all(&staging)?;

    extract::extract_archive(&archive_path, &staging, &manifest.archive.strip_prefix)
        .with_context(|| format!("extracting {name}"))?;

    let prefix = paths.prefix_for(name);
    let bindir = paths.bin_dir();
    std::fs::create_dir_all(&bindir).context("creating /usr/local/bin")?;

    let bins: Vec<String> = if manifest.install.bins.is_empty() {
        auto_detect_executables(&staging)?
    } else {
        manifest.install.bins.clone()
    };

    if let Ok(old) = Receipt::load(paths, name) {
        remove_receipt_artifacts(&old);
    }

    if prefix.exists() {
        std::fs::remove_dir_all(&prefix)?;
    }
    std::fs::rename(&staging, &prefix)
        .with_context(|| format!("moving {name} into {}", prefix.display()))?;

    let mut links = Vec::new();
    for bin in &bins {
        let src = prefix.join(bin);
        if !src.is_file() {
            bail!("manifest references missing binary '{bin}' in package {name}");
        }
        let link_path = bindir.join(src.file_name().unwrap());
        if link_path.symlink_metadata().is_ok() {
            let meta = link_path.symlink_metadata()?;
            if meta.file_type().is_symlink() {
                std::fs::remove_file(&link_path)?;
            } else {
                bail!(
                    "{} exists and is not a symlink managed by oxide",
                    link_path.display()
                );
            }
        }
        std::os::unix::fs::symlink(&src, &link_path)
            .with_context(|| format!("symlinking {}", link_path.display()))?;
        links.push(LinkEntry {
            path: link_path.to_string_lossy().into_owned(),
            target: src.to_string_lossy().into_owned(),
        });
    }

    let mut files = Vec::new();
    for entry in WalkDir::new(&prefix).follow_links(false) {
        let entry = entry?;
        if entry.file_type().is_file() {
            let rel = entry
                .path()
                .strip_prefix(&prefix)
                .unwrap()
                .to_string_lossy()
                .into_owned();
            let sha256 = util::sha256_file(entry.path())?;
            files.push(FileEntry { path: rel, sha256 });
        }
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));

    let receipt = Receipt {
        package: ReceiptPackage {
            name: name.clone(),
            version: manifest.package.version.clone(),
            description: manifest.package.description.clone(),
            installed_at: time::OffsetDateTime::now_utc()
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_default(),
        },
        prefix: prefix.to_string_lossy().into_owned(),
        depends: manifest.dependencies.depends.clone(),
        files,
        links,
    };
    Receipt::save(paths, &receipt)?;

    println!(
        "{} {name} {} ({} files)",
        "installed".green().bold(),
        manifest.package.version,
        receipt.files.len()
    );
    Ok(())
}

fn auto_detect_executables(staging: &Path) -> Result<Vec<String>> {
    let mut bins = Vec::new();
    for entry in WalkDir::new(staging).follow_links(false) {
        let entry = entry?;
        if entry.file_type().is_file() {
            let meta = entry.metadata()?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if meta.permissions().mode() & 0o111 != 0 {
                    bins.push(
                        entry
                            .path()
                            .strip_prefix(staging)
                            .unwrap()
                            .to_string_lossy()
                            .into_owned(),
                    );
                }
            }
        }
    }
    bins.sort();
    Ok(bins)
}

fn remove_receipt_artifacts(receipt: &Receipt) {
    for link in &receipt.links {
        let _ = std::fs::remove_file(&link.path);
    }
    let _ = std::fs::remove_dir_all(&receipt.prefix);
}

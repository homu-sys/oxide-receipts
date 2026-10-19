use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;

use crate::paths::Paths;
use crate::receipt::Receipt;
use crate::registry::Registry;
use crate::term;
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
        println!("{}  {}", term::bold(&padded), r.package.version);
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
                    term::green(&m.package.version),
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
            term::bold(&m.package.name),
            m.package.version,
            term::dim(m.package.description.as_deref().unwrap_or(""))
        );
    }
    Ok(())
}

pub fn request(registry: &Registry) -> Result<()> {
    let mut all: Vec<_> = registry.all().collect();
    if all.is_empty() {
        println!("registry is empty");
        return Ok(());
    }
    all.sort_by(|a, b| a.package.name.cmp(&b.package.name));
    let width = all.iter().map(|m| m.package.name.len()).max().unwrap_or(0);
    for m in &all {
        println!(
            "{} {} {}",
            term::bold(&format!("{:width$}", m.package.name, width = width)),
            m.package.version,
            term::dim(m.package.description.as_deref().unwrap_or(""))
        );
    }
    println!("\n{} package(s) available", all.len());
    Ok(())
}

pub fn info(paths: &Paths, registry: &Registry, name: &str) -> Result<()> {
    let manifest = registry.get(name)?;
    let p = &manifest.package;

    println!("{}", term::bold(&p.name));
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
            term::green("installed"),
            r.package.version
        ),
        Err(_) => println!("  status:       {}", term::dim("not installed")),
    }
    Ok(())
}

pub fn files(paths: &Paths, name: &str) -> Result<()> {
    let receipt = Receipt::load(paths, name)?;
    for link in &receipt.links {
        println!("{} {}", term::cyan("link"), link.path);
    }
    for file in &receipt.files {
        println!("{} {}/{}", term::dim("file"), receipt.prefix, file.path);
    }
    Ok(())
}

pub fn owner(paths: &Paths, path: &Path) -> Result<()> {
    let target = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let receipts = Receipt::load_all(paths)?;
    let mut found = false;

    for r in &receipts {
        let mut owned = r.links.iter().any(|link| {
            let lp = Path::new(&link.path);
            let lt = std::fs::canonicalize(lp).unwrap_or_else(|_| lp.to_path_buf());
            lt == target || lp == target
        });

        if !owned {
            let prefix = Path::new(&r.prefix);
            owned = r.files.iter().any(|file| {
                let p = prefix.join(&file.path);
                let pc = std::fs::canonicalize(&p).unwrap_or_else(|_| p.clone());
                p == target || pc == target
            });
        }

        if owned {
            println!(
                "{} is owned by {} {}",
                target.display(),
                term::bold(&r.package.name),
                r.package.version
            );
            found = true;
        }
    }

    if !found {
        println!("{}", term::yellow("no package owns this file"));
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
                        term::red("!!"),
                        file.path,
                        name = receipt.package.name
                    );
                    ok = false;
                }
                Err(_) => {
                    println!(
                        "  {} {name}: missing: {}",
                        term::red("!!"),
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
                        term::red("!!"),
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
                term::green_bold("ok"),
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
    let mut cleaned = false;
    for dir in [paths.staging(), paths.cache()] {
        if dir.exists() {
            std::fs::remove_dir_all(&dir)?;
            println!("cleaned {}", dir.display());
            cleaned = true;
        }
    }
    if !cleaned {
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
            println!(
                "  {} {} {}",
                term::bold(&d.package.name),
                d.package.version,
                term::dim("← explicit dependency")
            );
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
    println!("{}", term::bold(&manifest.package.name));
    print_deps(registry, name, "  ", &mut Vec::new())?;
    Ok(())
}

fn print_deps(registry: &Registry, name: &str, indent: &str, seen: &mut Vec<String>) -> Result<()> {
    if seen.iter().any(|s| s == name) {
        println!("{indent}{} (cycle)", term::dim(name));
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

pub fn tree(paths: &Paths, package: Option<&str>) -> Result<()> {
    let receipts = Receipt::load_all(paths)?;
    if receipts.is_empty() {
        println!("no packages installed");
        return Ok(());
    }
    let by_name: HashMap<&str, &Receipt> =
        receipts.iter().map(|r| (r.package.name.as_str(), r)).collect();

    let roots: Vec<&Receipt> = match package {
        Some(name) => vec![by_name
            .get(name)
            .copied()
            .with_context(|| format!("package '{name}' is not installed"))?],
        None => receipts.iter().filter(|r| r.explicit).collect(),
    };

    let mut seen: Vec<String> = Vec::new();
    for root in roots {
        if seen.iter().any(|s| s == &root.package.name) {
            continue;
        }
        seen.push(root.package.name.clone());
        println!(
            "{} {}",
            term::bold(&root.package.name),
            term::dim(&root.package.version)
        );
        print_dep_tree(&by_name, root, "", &mut seen);
    }
    Ok(())
}

fn print_dep_tree(
    by_name: &HashMap<&str, &Receipt>,
    receipt: &Receipt,
    prefix: &str,
    seen: &mut Vec<String>,
) {
    let n = receipt.depends.len();
    for (i, dep) in receipt.depends.iter().enumerate() {
        let last = i + 1 == n;
        let branch = if last { "└── " } else { "├── " };
        let cont = if last { "    " } else { "│   " };
        match by_name.get(dep.as_str()) {
            Some(r) => {
                if seen.iter().any(|s| s == dep) {
                    println!("{prefix}{branch}{}", term::dim(&r.package.name));
                } else {
                    seen.push(dep.clone());
                    println!("{prefix}{branch}{}", term::bold(&r.package.name));
                    print_dep_tree(by_name, r, &format!("{prefix}{cont}"), seen);
                }
            }
            None => println!("{prefix}{branch}{} {}", dep, term::red("(missing)")),
        }
    }
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
                term::bold(&r.package.name),
                r.package.version,
                term::dim("(orphan)")
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
                term::yellow_bold("held"),
                name,
                receipt.package.version
            );
        } else {
            println!(
                "{} {} {}",
                term::green("unheld"),
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
            term::dim(&r.package.installed_at),
            term::bold(&r.package.name),
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
        let padded = format!("{name:<width$}");
        println!(
            "{}  {version:>10}  {:>10}",
            term::bold(&padded),
            human_size(*size)
        );
    }
    let label = format!("{:<width$}", "total");
    println!(
        "{}  {:>10}  {:>10}",
        term::bold(&label),
        "",
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
                    term::red("!!"),
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
                term::red("!!"),
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
        term::green_bold("ok"),
        installed.len()
    );
    Ok(())
}

pub fn doctor(paths: &Paths) -> Result<()> {
    use std::process::Command;

    println!("{}", term::bold("oxide doctor"));
    let mut issues = 0usize;

    let check = |ok: bool, label: &str, detail: &str, issues: &mut usize| {
        if ok {
            println!("  {} {label} {}", term::green("✓"), term::dim(detail));
        } else {
            println!("  {} {label} {}", term::red("✗"), term::dim(detail));
            *issues += 1;
        }
    };

    let root = paths.root.display().to_string();
    check(true, "root", &format!("{root}"), &mut issues);

    // curl or wget is needed by `oxide sync`.
    let curl = Command::new("curl")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    let wget = Command::new("wget")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    let dl = if curl { "curl" } else if wget { "wget" } else { "missing — sync will fail" };
    check(curl || wget, "download", dl, &mut issues);

    // Registry index present?
    let index = paths.index();
    let registry_dir = paths.registry().join("packages");
    if index.is_file() {
        let count = crate::manifest::load_index(&index).map(|m| m.len()).unwrap_or(0);
        check(true, "registry", &format!("{count} package(s)"), &mut issues);
    } else if registry_dir.is_dir() {
        let count = std::fs::read_dir(&registry_dir)
            .map(|d| d.filter_map(|e| e.ok()).filter(|e| e.path().is_dir()).count())
            .unwrap_or(0);
        check(true, "registry", &format!("{count} package(s)"), &mut issues);
    } else {
        check(false, "registry", "not synced — run `oxide sync`", &mut issues);
    }

    // Receipts readable?
    let receipts = Receipt::load_all(paths)?;
    check(
        true,
        "receipts",
        &format!("{} package(s) installed", receipts.len()),
        &mut issues,
    );

    // orphans
    let orphans = receipts
        .iter()
        .filter(|r| {
            !r.explicit
                && !receipts
                    .iter()
                    .any(|x| x.depends.iter().any(|d| d == &r.package.name))
        })
        .count();
    if orphans > 0 {
        println!(
            "  {} orphans {}",
            term::yellow("!"),
            term::dim(&format!("{orphans} package(s) — see `oxide orphans`"))
        );
    }

    // /usr/local/bin on PATH?
    let path_ok = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).any(|d| d == Path::new("/usr/local/bin")))
        .unwrap_or(false);
    check(
        path_ok,
        "PATH",
        if path_ok { "contains /usr/local/bin" } else { "missing /usr/local/bin" },
        &mut issues,
    );

    if issues == 0 {
        println!("\n{}", term::green_bold("all good"));
    } else {
        println!(
            "\n{}",
            term::yellow_bold(&format!("{issues} issue(s) found"))
        );
    }
    Ok(())
}

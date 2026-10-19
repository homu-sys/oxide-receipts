use anyhow::Result;

use crate::install;
use crate::paths::Paths;
use crate::receipt::Receipt;
use crate::registry::Registry;
use crate::term;
use crate::util;

pub fn update_packages(paths: &Paths, registry: &Registry, names: &[String]) -> Result<()> {
    let installed = Receipt::load_all(paths)?;
    let full = names.is_empty();
    let requested: Vec<String> = if full {
        installed.iter().map(|r| r.package.name.clone()).collect()
    } else {
        names.to_vec()
    };

    let mut targets = Vec::new();
    for name in &requested {
        let Some(receipt) = installed.iter().find(|r| &r.package.name == name) else {
            println!("{} '{name}' is not installed", term::yellow("warning:"));
            continue;
        };
        if receipt.hold {
            println!(
                "{} {} {} (held)",
                term::dim("skipping"),
                name,
                receipt.package.version
            );
            continue;
        }
        targets.push(name.clone());
    }

    // Packages with a registry version newer than what is installed.
    let mut upgrades: Vec<(String, String, String)> = Vec::new();
    for name in &targets {
        let receipt = installed.iter().find(|r| &r.package.name == name).unwrap();
        if let Ok(manifest) = registry.get(name) {
            if util::version_gt(&manifest.package.version, &receipt.package.version) {
                upgrades.push((
                    name.clone(),
                    receipt.package.version.clone(),
                    manifest.package.version.clone(),
                ));
            }
        }
    }

    if upgrades.is_empty() {
        println!("{}", term::dim("there is nothing to do"));
        return Ok(());
    }

    println!(
        "{}",
        term::bold(if full {
            ":: Starting full system upgrade..."
        } else {
            ":: Upgrading packages..."
        })
    );
    print!(
        "{}",
        term::bold(&format!("Packages ({}):", upgrades.len()))
    );
    for (name, _, new) in &upgrades {
        print!(" {name}-{new}");
    }
    println!("\n");

    let targets: Vec<String> = upgrades.iter().map(|(n, _, _)| n.clone()).collect();
    install::install_packages(paths, registry, &targets)
}

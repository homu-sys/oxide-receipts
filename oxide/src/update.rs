use anyhow::Result;

use crate::install;
use crate::paths::Paths;
use crate::receipt::Receipt;
use crate::registry::Registry;
use crate::term;
use crate::util;

pub fn update_packages(paths: &Paths, registry: &Registry, names: &[String]) -> Result<()> {
    let installed = Receipt::load_all(paths)?;
    let requested: Vec<String> = if names.is_empty() {
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

    let mut updated = 0usize;
    for name in &targets {
        let receipt = installed.iter().find(|r| &r.package.name == name).unwrap();
        if let Ok(manifest) = registry.get(name) {
            if util::version_gt(&manifest.package.version, &receipt.package.version) {
                updated += 1;
            }
        }
    }

    if updated == 0 {
        println!("everything is up to date");
        return Ok(());
    }

    install::install_packages(paths, registry, &targets)
}

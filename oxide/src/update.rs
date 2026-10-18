use anyhow::Result;
use owo_colors::OwoColorize;

use crate::install;
use crate::paths::Paths;
use crate::receipt::Receipt;
use crate::registry::Registry;
use crate::util;

pub fn update_packages(paths: &Paths, registry: &Registry, names: &[String]) -> Result<()> {
    let installed = Receipt::load_all(paths)?;
    let targets: Vec<String> = if names.is_empty() {
        installed.iter().map(|r| r.package.name.clone()).collect()
    } else {
        names.to_vec()
    };

    let mut updated = 0usize;
    for name in &targets {
        let Some(receipt) = installed.iter().find(|r| &r.package.name == name) else {
            println!("{} '{name}' is not installed", "warning:".yellow());
            continue;
        };
        let Ok(manifest) = registry.get(name) else {
            continue;
        };
        if util::version_gt(&manifest.package.version, &receipt.package.version) {
            updated += 1;
        }
    }

    if updated == 0 {
        println!("everything is up to date");
        return Ok(());
    }

    install::install_packages(paths, registry, &targets)
}

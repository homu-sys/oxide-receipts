use anyhow::{bail, Result};

use crate::paths::Paths;
use crate::receipt::Receipt;
use crate::term;

pub fn remove_packages(paths: &Paths, names: &[String], force: bool) -> Result<()> {
    let all = Receipt::load_all(paths)?;

    for name in names {
        let receipt = all
            .iter()
            .find(|r| &r.package.name == name)
            .ok_or_else(|| anyhow::anyhow!("package '{name}' is not installed"))?;

        if !force {
            let dependents: Vec<&str> = all
                .iter()
                .filter(|r| r.depends.iter().any(|d| d == name))
                .map(|r| r.package.name.as_str())
                .collect();
            if !dependents.is_empty() {
                bail!(
                    "'{name}' is required by: {} — use --force to remove anyway",
                    dependents.join(", ")
                );
            }
        }

        for link in &receipt.links {
            let _ = std::fs::remove_file(&link.path);
        }
        std::fs::remove_dir_all(&receipt.prefix)?;
        Receipt::delete(paths, name)?;
        println!(
            "{} {name} {}",
            term::red_bold("removed"),
            receipt.package.version
        );
    }
    Ok(())
}

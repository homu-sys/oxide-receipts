use anyhow::{bail, Context, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

use crate::manifest::{self, Manifest};
use crate::paths::Paths;
use crate::term;

pub struct Registry {
    path: PathBuf,
    packages: HashMap<String, Manifest>,
}

impl Registry {
    pub fn sync(paths: &Paths, url: &str) -> Result<()> {
        let dest = paths.registry();
        let start = Instant::now();

        if dest.join(".git").exists() {
            let status = Command::new("git")
                .args(["-C", &dest.to_string_lossy(), "pull", "--ff-only", "--quiet"])
                .status()
                .context("running git pull — is git installed?")?;
            if !status.success() {
                bail!("git pull failed in {}", dest.display());
            }
            println!(
                "{} in {:.2}s",
                term::green("registry updated"),
                start.elapsed().as_secs_f64()
            );
        } else {
            if dest.exists() {
                std::fs::remove_dir_all(&dest)
                    .with_context(|| format!("clearing {}", dest.display()))?;
            }
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let status = Command::new("git")
                .args([
                    "clone",
                    "--depth",
                    "1",
                    "--quiet",
                    url,
                    &dest.to_string_lossy(),
                ])
                .status()
                .context("running git clone — is git installed?")?;
            if !status.success() {
                bail!("git clone of {url} failed");
            }
            println!(
                "{} from {url} in {:.2}s",
                term::green("registry synced"),
                start.elapsed().as_secs_f64()
            );
        }
        Ok(())
    }

    pub fn load(paths: &Paths) -> Result<Registry> {
        let path = paths.registry().join("packages");
        let packages = manifest::load_all(&path)?;
        Ok(Registry { path, packages })
    }

    pub fn package_dir(&self, name: &str) -> PathBuf {
        self.path.join(name)
    }

    pub fn get(&self, name: &str) -> Result<&Manifest> {
        self.packages
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("package '{name}' not found in registry"))
    }

    pub fn all(&self) -> impl Iterator<Item = &Manifest> {
        self.packages.values()
    }
}

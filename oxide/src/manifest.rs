use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct Manifest {
    pub package: Package,
    #[serde(default)]
    pub dependencies: Dependencies,
    pub archive: Archive,
    #[serde(default)]
    pub install: Install,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub license: Option<String>,
    #[serde(default)]
    pub arch: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Dependencies {
    #[serde(default)]
    pub depends: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Archive {
    pub file: String,
    #[serde(default)]
    pub strip_prefix: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Install {
    #[serde(default)]
    pub bins: Vec<String>,
}

impl Manifest {
    pub fn supports_current_arch(&self) -> bool {
        self.package.arch.is_empty() || self.package.arch.iter().any(|a| a == crate::util::current_arch())
    }
}

pub fn load_all(packages_dir: &Path) -> Result<HashMap<String, Manifest>> {
    let mut manifests = HashMap::new();
    if !packages_dir.is_dir() {
        bail!(
            "registry not synced at {} — run `oxide sync` first",
            packages_dir.display()
        );
    }
    let mut entries: Vec<_> = std::fs::read_dir(packages_dir)
        .context("reading registry packages directory")?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let manifest_path = entry.path().join("config.toml");
        if !manifest_path.exists() {
            continue;
        }
        let content = std::fs::read_to_string(&manifest_path)
            .with_context(|| format!("reading {}", manifest_path.display()))?;
        let manifest: Manifest = toml::from_str(&content)
            .with_context(|| format!("parsing {}", manifest_path.display()))?;
        manifests.insert(manifest.package.name.clone(), manifest);
    }
    Ok(manifests)
}

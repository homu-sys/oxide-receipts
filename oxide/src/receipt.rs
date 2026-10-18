use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::paths::Paths;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Receipt {
    pub package: ReceiptPackage,
    pub prefix: String,
    #[serde(default)]
    pub depends: Vec<String>,
    #[serde(default)]
    pub files: Vec<FileEntry>,
    #[serde(default)]
    pub links: Vec<LinkEntry>,
    /// True when the user installed it directly (not pulled in as a dependency).
    #[serde(default = "default_true")]
    pub explicit: bool,
    /// Held packages are skipped by `oxide update`.
    #[serde(default)]
    pub hold: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptPackage {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub installed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    /// Path relative to the package prefix.
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkEntry {
    /// Absolute path of the symlink (inside the install root).
    pub path: String,
    /// Absolute path the symlink points to (inside the install root).
    pub target: String,
}

impl Receipt {
    pub fn receipt_path(paths: &Paths, name: &str) -> PathBuf {
        paths.receipts().join(format!("{name}.toml"))
    }

    pub fn load(paths: &Paths, name: &str) -> Result<Receipt> {
        let path = Self::receipt_path(paths, name);
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("package '{name}' is not installed"))?;
        toml::from_str(&content).context("parsing receipt")
    }

    pub fn load_all(paths: &Paths) -> Result<Vec<Receipt>> {
        let dir = paths.receipts();
        if !dir.is_dir() {
            return Ok(Vec::new());
        }
        let mut receipts: Vec<Receipt> = Vec::new();
        for entry in std::fs::read_dir(&dir).context("reading receipts directory")? {
            let entry = entry?;
            if entry.path().extension().is_some_and(|e| e == "toml") {
                let content = std::fs::read_to_string(entry.path())?;
                receipts.push(toml::from_str(&content)?);
            }
        }
        receipts.sort_by(|a, b| a.package.name.cmp(&b.package.name));
        Ok(receipts)
    }

    pub fn save(paths: &Paths, receipt: &Receipt) -> Result<()> {
        std::fs::create_dir_all(paths.receipts())?;
        let content = toml::to_string_pretty(receipt).context("serializing receipt")?;
        std::fs::write(Self::receipt_path(paths, &receipt.package.name), content)?;
        Ok(())
    }

    pub fn delete(paths: &Paths, name: &str) -> Result<()> {
        let path = Self::receipt_path(paths, name);
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        Ok(())
    }
}

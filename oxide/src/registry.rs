use anyhow::{bail, Context, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

use crate::manifest::{self, Manifest};
use crate::paths::Paths;
use crate::term;
use crate::util;

pub struct Registry {
    path: PathBuf,
    packages: HashMap<String, Manifest>,
}

impl Registry {
    /// Sync the registry by fetching the single `index` file directly.
    pub fn sync(paths: &Paths, url: &str) -> Result<()> {
        let start = Instant::now();
        let index_url = index_url_for(url);
        let dest = paths.index();

        util::download(&index_url, &dest)
            .with_context(|| format!("fetching the index from {index_url}"))?;

        let packages = manifest::load_index(&dest)
            .context("downloaded index is not valid — check the registry URL")?;
        if packages.is_empty() {
            bail!("index at {index_url} contains no packages");
        }
        println!(
            "{} {} packages from {} in {:.2}s",
            term::green("registry synced:"),
            packages.len(),
            index_url,
            start.elapsed().as_secs_f64()
        );
        Ok(())
    }

    pub fn load(paths: &Paths) -> Result<Registry> {
        let path = paths.registry();
        let index = path.join("index");
        let packages = if index.is_file() {
            manifest::load_index(&index)?
        } else {
            manifest::load_all(&path.join("packages"))?
        };
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

/// Turn a registry URL into the URL of its single `index` file.
///
/// Accepts an explicit index URL, a GitHub repo (`https://github.com/o/r[.git]`
/// → `raw.githubusercontent.com/o/r/main/index`), any other base URL, or a
/// local directory (served via `file://`).
fn index_url_for(url: &str) -> String {
    let url = url.trim_end_matches('/');
    if url.ends_with("/index") {
        return url.to_string();
    }
    if let Some(rest) = url.strip_prefix("https://github.com/") {
        let rest = rest.trim_end_matches(".git");
        if rest.matches('/').count() == 1 {
            return format!("https://raw.githubusercontent.com/{rest}/refs/heads/main/index");
        }
    }
    if url.contains("://") {
        return format!("{url}/index");
    }
    format!("file://{url}/index")
}

use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct Paths {
    pub root: PathBuf,
}

impl Paths {
    pub fn new(root: Option<PathBuf>) -> Self {
        Self {
            root: root.unwrap_or_else(|| PathBuf::from("/")),
        }
    }

    pub fn is_system(&self) -> bool {
        self.root == Path::new("/")
    }

    pub fn registry(&self) -> PathBuf {
        self.root.join("var/lib/oxide/registry")
    }

    pub fn receipts(&self) -> PathBuf {
        self.root.join("var/lib/oxide/receipts")
    }

    pub fn staging(&self) -> PathBuf {
        self.root.join("var/lib/oxide/staging")
    }

    pub fn prefix_for(&self, name: &str) -> PathBuf {
        self.root.join("opt/oxide").join(name)
    }

    pub fn bin_dir(&self) -> PathBuf {
        self.root.join("usr/local/bin")
    }

    pub fn config_file(&self) -> PathBuf {
        self.root.join("etc/oxide/config.toml")
    }
}

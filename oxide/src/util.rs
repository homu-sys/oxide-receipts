use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn current_arch() -> &'static str {
    std::env::consts::ARCH
}

pub fn euid() -> u32 {
    unsafe { libc::geteuid() }
}

pub fn sha256_file(path: &Path) -> Result<String> {
    let mut file = std::fs::File::open(path)
        .with_context(|| format!("opening {} for hashing", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

pub fn normalize_version(v: &str) -> String {
    v.trim_start_matches('v').to_string()
}

pub fn version_gt(a: &str, b: &str) -> bool {
    match (
        semver::Version::parse(&normalize_version(a)),
        semver::Version::parse(&normalize_version(b)),
    ) {
        (Ok(a), Ok(b)) => a > b,
        _ => a != b,
    }
}

fn on_path(name: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|paths| {
        std::env::split_paths(&paths).any(|dir| dir.join(name).is_file())
    })
}

/// Download `url` to `dest`, following redirects, via curl (or wget).
pub fn download(url: &str, dest: &Path) -> Result<()> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let mut part = dest.as_os_str().to_os_string();
    part.push(".part");
    let part = PathBuf::from(part);

    let status = if on_path("curl") {
        Command::new("curl")
            .args(["-fsSL", "--retry", "3", "-o"])
            .arg(&part)
            .arg(url)
            .status()
            .context("running curl")?
    } else if on_path("wget") {
        Command::new("wget")
            .args(["-q", "-O"])
            .arg(&part)
            .arg(url)
            .status()
            .context("running wget")?
    } else {
        anyhow::bail!("need curl or wget to download {url}");
    };

    if !status.success() {
        let _ = std::fs::remove_file(&part);
        anyhow::bail!("download failed: {url}");
    }
    std::fs::rename(&part, dest)
        .with_context(|| format!("moving download into {}", dest.display()))?;
    Ok(())
}

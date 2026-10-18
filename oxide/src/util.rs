use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::Path;

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

use anyhow::{bail, Context, Result};
use std::fs::File;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

pub fn extract_archive(archive: &Path, dest: &Path, strip_prefix: &str) -> Result<()> {
    let name = archive
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let strip = strip_components(strip_prefix);

    if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
        let decoder = flate2::read::GzDecoder::new(File::open(archive)?);
        untar(decoder, dest, strip)
    } else if name.ends_with(".tar.xz") {
        let decoder = xz2::read::XzDecoder::new(File::open(archive)?);
        untar(decoder, dest, strip)
    } else if name.ends_with(".tar.zst") || name.ends_with(".tar.zstd") {
        let decoder = zstd::Decoder::new(File::open(archive)?).context("opening zstd stream")?;
        untar(decoder, dest, strip)
    } else if name.ends_with(".tar") {
        untar(File::open(archive)?, dest, strip)
    } else {
        bail!("unsupported archive format: {name}");
    }
}

fn strip_components(prefix: &str) -> usize {
    let prefix = prefix.trim_matches('/');
    if prefix.is_empty() {
        0
    } else {
        prefix.split('/').count()
    }
}

fn untar<R: Read>(reader: R, dest: &Path, strip: usize) -> Result<()> {
    let mut archive = tar::Archive::new(reader);
    for entry in archive.entries().context("reading tar entries")? {
        let mut entry = entry.context("reading tar entry")?;
        let path = entry.path()?.into_owned();

        if path
            .components()
            .any(|c| matches!(c, Component::ParentDir | Component::RootDir))
        {
            bail!("archive contains unsafe path: {}", path.display());
        }

        let components: Vec<Component> = path.components().collect();
        if components.len() <= strip {
            continue;
        }
        let rel: PathBuf = components[strip..]
            .iter()
            .map(|c| c.as_os_str().to_os_string())
            .collect();
        if rel.as_os_str().is_empty() {
            continue;
        }
        let out = dest.join(rel);

        if entry.header().entry_type().is_dir() {
            std::fs::create_dir_all(&out)?;
            continue;
        }
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent)?;
        }
        entry.unpack(&out)?;
    }
    Ok(())
}

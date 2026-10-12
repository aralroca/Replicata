//! Result bundles: a gzipped tar of the output directory.
//!
//! Tar metadata (mtime, uid, order) is normalised so the same directory always packs to the
//! same bytes, but the *identity* of a bundle is always the canonical hash of its files
//! ([`crate::hash`]), never the hash of the archive.

use std::io::{Cursor, Read};
use std::path::Path;

use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;

use crate::hash::{hash_files, read_dir_files, HashError, OutputHash};

/// Hard cap on the unpacked size of a bundle (defence against zip bombs).
pub const MAX_UNPACKED_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum BundleError {
    #[error(transparent)]
    Hash(#[from] HashError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("bundle entry is not a regular file or directory: {0}")]
    EntryType(String),
    #[error("bundle is larger than {MAX_UNPACKED_BYTES} bytes once unpacked")]
    TooLarge,
}

/// Packs every regular file under `root` into a deterministic `.tar.gz`.
pub fn pack_dir(root: &Path) -> Result<Vec<u8>, BundleError> {
    let mut files = read_dir_files(root)?;

    files.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
    pack_files(&files)
}

pub fn pack_files(files: &[(String, Vec<u8>)]) -> Result<Vec<u8>, BundleError> {
    let mut builder = tar::Builder::new(GzEncoder::new(Vec::new(), Compression::default()));

    for (name, bytes) in files {
        let mut header = tar::Header::new_gnu();
        header.set_size(bytes.len() as u64);
        header.set_mode(0o644);
        header.set_mtime(0);
        header.set_uid(0);
        header.set_gid(0);
        header.set_entry_type(tar::EntryType::Regular);
        builder.append_data(&mut header, name, bytes.as_slice())?;
    }
    Ok(builder.into_inner()?.finish()?)
}

/// Unpacks a bundle into memory, rejecting links/devices and oversized content.
pub fn unpack(bundle: &[u8]) -> Result<Vec<(String, Vec<u8>)>, BundleError> {
    let mut archive = tar::Archive::new(GzDecoder::new(Cursor::new(bundle)));
    let mut files = Vec::new();
    let mut total = 0u64;

    for entry in archive.entries()? {
        let mut entry = entry?;
        let name = entry.path()?.to_string_lossy().trim_end_matches('/').to_string();
        match entry.header().entry_type() {
            tar::EntryType::Directory => continue,
            tar::EntryType::Regular | tar::EntryType::Continuous => {}
            _ => return Err(BundleError::EntryType(name)),
        }
        total = total.saturating_add(entry.header().size()?);
        if total > MAX_UNPACKED_BYTES {
            return Err(BundleError::TooLarge);
        }
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes)?;
        files.push((name, bytes));
    }
    Ok(files)
}

/// Canonical output hash of a bundle's contents.
pub fn hash_bundle(bundle: &[u8]) -> Result<OutputHash, BundleError> {
    let files = unpack(bundle)?;

    Ok(hash_files(files.iter().map(|(n, b)| (n.as_str(), b.as_slice())))?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash::hash_dir;

    #[test]
    fn pack_is_deterministic_and_hash_survives_roundtrip() {
        let dir = tempfile::tempdir().unwrap();

        std::fs::create_dir_all(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("result.json"), b"{}").unwrap();
        std::fs::write(dir.path().join("sub/x.txt"), b"hello").unwrap();
        let a = pack_dir(dir.path()).unwrap();
        let b = pack_dir(dir.path()).unwrap();
        assert_eq!(a, b);
        assert_eq!(hash_bundle(&a).unwrap(), hash_dir(dir.path()).unwrap());
    }

    #[test]
    fn symlink_entries_are_rejected() {
        let mut builder = tar::Builder::new(GzEncoder::new(Vec::new(), Compression::default()));
        let mut header = tar::Header::new_gnu();

        header.set_entry_type(tar::EntryType::Symlink);
        header.set_size(0);
        builder.append_link(&mut header, "evil", "/etc/passwd").unwrap();
        let bytes = builder.into_inner().unwrap().finish().unwrap();
        assert!(matches!(unpack(&bytes), Err(BundleError::EntryType(_))));
    }

    #[test]
    fn traversal_names_fail_hashing() {
        let bytes = pack_files(&[("ok".into(), b"1".to_vec())]).unwrap();

        assert!(hash_bundle(&bytes).is_ok());
        assert!(hash_bundle(b"not a gzip").is_err());
    }
}

//! Canonical output hash.
//!
//! `H = sha256( concat( for (name, bytes) in sorted_by_name(files): name ‖ 0x00 ‖ sha256(bytes) ‖ 0x00 ) )`
//!
//! * `name` is the path relative to the output root, `/`-separated, UTF-8.
//! * Only regular files count; empty directories are ignored; symlinks are rejected
//!   (a symlink could point outside the sandbox and differs across machines).
//! * stdout/stderr are never part of `H`.

use std::fs;
use std::path::Path;

use sha2::{Digest, Sha256};

pub type OutputHash = [u8; 32];

#[derive(Debug, thiserror::Error)]
pub enum HashError {
    #[error("symlinks are not allowed in the output: {0}")]
    Symlink(String),
    #[error("unsupported file type in the output: {0}")]
    FileType(String),
    #[error("non UTF-8 or unsafe path in the output: {0}")]
    Path(String),
    #[error("duplicate path in the output: {0}")]
    Duplicate(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Hash of an in-memory set of `(relative_name, contents)` files.
pub fn hash_files<'a, I>(files: I) -> Result<OutputHash, HashError>
where
    I: IntoIterator<Item = (&'a str, &'a [u8])>,
{
    let mut entries: Vec<(&str, [u8; 32])> =
        files.into_iter().map(|(name, bytes)| (name, Sha256::digest(bytes).into())).collect();
    entries.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
    if let Some(pair) = entries.windows(2).find(|pair| pair[0].0 == pair[1].0) {
        return Err(HashError::Duplicate(pair[0].0.to_string()));
    }
    if let Some((name, _)) = entries.iter().find(|(name, _)| !crate::spec::is_safe_relative_path(name)) {
        return Err(HashError::Path(name.to_string()));
    }

    let mut hasher = Sha256::new();
    for (name, digest) in &entries {
        hasher.update(name.as_bytes());
        hasher.update([0u8]);
        hasher.update(digest);
        hasher.update([0u8]);
    }
    Ok(hasher.finalize().into())
}

/// Reads every regular file under `root` (recursively) as `(relative_name, contents)`.
pub fn read_dir_files(root: &Path) -> Result<Vec<(String, Vec<u8>)>, HashError> {
    let mut out = Vec::new();

    collect(root, root, &mut out)?;
    Ok(out)
}

fn collect(root: &Path, dir: &Path, out: &mut Vec<(String, Vec<u8>)>) -> Result<(), HashError> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        let display = path.display().to_string();
        if file_type.is_symlink() {
            return Err(HashError::Symlink(display));
        }
        if file_type.is_dir() {
            collect(root, &path, out)?;
            continue;
        }
        if !file_type.is_file() {
            return Err(HashError::FileType(display));
        }
        let name = relative_name(root, &path).ok_or(HashError::Path(display))?;
        out.push((name, fs::read(&path)?));
    }
    Ok(())
}

fn relative_name(root: &Path, path: &Path) -> Option<String> {
    let rel = path.strip_prefix(root).ok()?;
    let parts: Option<Vec<&str>> = rel.components().map(|c| c.as_os_str().to_str()).collect();

    Some(parts?.join("/"))
}

/// Canonical hash of a result directory.
pub fn hash_dir(root: &Path) -> Result<OutputHash, HashError> {
    let files = read_dir_files(root)?;

    hash_files(files.iter().map(|(name, bytes)| (name.as_str(), bytes.as_slice())))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(root: &Path, name: &str, bytes: &[u8]) {
        let path = root.join(name);

        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }

    #[test]
    fn dir_hash_matches_in_memory_hash_and_ignores_empty_dirs() {
        let dir = tempfile::tempdir().unwrap();

        write(dir.path(), "result.json", b"{\"pi\":3.14}");
        write(dir.path(), "plots/a.csv", b"1,2\n");
        fs::create_dir_all(dir.path().join("empty")).unwrap();
        let expected = hash_files([("plots/a.csv", &b"1,2\n"[..]), ("result.json", &b"{\"pi\":3.14}"[..])]).unwrap();
        assert_eq!(hash_dir(dir.path()).unwrap(), expected);
    }

    #[test]
    fn empty_output_has_a_stable_hash() {
        let dir = tempfile::tempdir().unwrap();

        assert_eq!(hash_dir(dir.path()).unwrap(), <[u8; 32]>::from(Sha256::digest([])));
    }

    #[cfg(unix)]
    #[test]
    fn symlinks_are_rejected() {
        let dir = tempfile::tempdir().unwrap();

        write(dir.path(), "a", b"x");
        std::os::unix::fs::symlink(dir.path().join("a"), dir.path().join("link")).unwrap();
        assert!(matches!(hash_dir(dir.path()), Err(HashError::Symlink(_))));
    }

    #[test]
    fn duplicates_and_unsafe_names_are_rejected() {
        assert!(matches!(hash_files([("a", &b"1"[..]), ("a", &b"2"[..])]), Err(HashError::Duplicate(_))));
        assert!(matches!(hash_files([("../a", &b"1"[..])]), Err(HashError::Path(_))));
    }
}

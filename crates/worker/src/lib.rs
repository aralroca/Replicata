//! Replicata worker: replicates experiments in a deterministic Docker sandbox, takes part
//! in the on-chain commit-reveal and gets paid in USDC.

pub mod api;
pub mod chain;
pub mod daemon;
pub mod inputs;
pub mod job;
pub mod pay;
pub mod sandbox;
pub mod state;

use std::path::{Path, PathBuf};

use anyhow::Context;
use solana_keypair::Keypair;

/// `$REPLICATA_HOME` or `~/.replicata`.
pub fn home() -> PathBuf {
    std::env::var_os("REPLICATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".replicata"))
}

pub fn read_keypair(path: &Path) -> anyhow::Result<Keypair> {
    let bytes: Vec<u8> =
        serde_json::from_str(&std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?)?;

    Keypair::try_from(bytes.as_slice()).context("invalid keypair file")
}

/// Loads the keypair, creating it (Solana CLI JSON format) if it does not exist.
pub fn load_or_create_keypair(path: &Path) -> anyhow::Result<(Keypair, bool)> {
    if path.exists() {
        return Ok((read_keypair(path)?, false));
    }
    let keypair = Keypair::new();
    std::fs::create_dir_all(path.parent().unwrap_or(Path::new(".")))?;
    std::fs::write(path, serde_json::to_string(&keypair.to_bytes().to_vec())?)?;
    #[cfg(unix)]
    std::os::unix::fs::PermissionsExt::set_mode(&mut std::fs::metadata(path)?.permissions(), 0o600);

    Ok((keypair, true))
}

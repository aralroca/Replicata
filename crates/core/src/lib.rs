//! Shared building blocks for every Replicata component (program tests, coordinator,
//! worker and — through `vectors/` — the web app).
//!
//! * [`spec`]: the experiment `JobSpec` and its canonical hash (`spec_hash` on-chain).
//! * [`hash`]: the canonical output hash `H` of a result directory or bundle.
//! * [`commit`]: the commit-reveal commitment and the bundle upload message.
//! * [`bundle`]: tar.gz packing/unpacking of a result directory.
//! * [`net`]: cluster constants (program id, USDC mint, RPC, CAIP-2 chain id).

pub mod bundle;
pub mod commit;
pub mod hash;
pub mod net;
pub mod spec;

pub use commit::{bundle_upload_message, commitment};
pub use hash::{hash_dir, hash_files, OutputHash};
pub use spec::{Input, JobSpec, SpecError};

/// Lowercase hex encoding of a 32-byte hash.
pub fn to_hex(bytes: &[u8; 32]) -> String {
    hex::encode(bytes)
}

/// Parses 64 lowercase/uppercase hex chars into 32 bytes.
pub fn from_hex(s: &str) -> Option<[u8; 32]> {
    let mut out = [0u8; 32];

    hex::decode_to_slice(s, &mut out).ok()?;
    Some(out)
}

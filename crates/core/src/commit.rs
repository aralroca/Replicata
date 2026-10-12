//! Commit-reveal and bundle-upload messages.
//!
//! The commitment binds the worker's public key, so a worker cannot copy another
//! worker's (public, on-chain) commitment and later reveal someone else's result.
//! The on-chain program computes exactly the same bytes in `reveal`.

use sha2::{Digest, Sha256};

/// `sha256(output_hash ‖ salt ‖ worker_pubkey)`.
pub fn commitment(output_hash: &[u8; 32], salt: &[u8; 32], worker: &[u8; 32]) -> [u8; 32] {
    let mut hasher = Sha256::new();

    hasher.update(output_hash);
    hasher.update(salt);
    hasher.update(worker);
    hasher.finalize().into()
}

/// Message a worker signs (ed25519) to upload its result bundle to the coordinator:
/// `"replicata:bundle:v1:" ‖ experiment_pubkey ‖ output_hash`.
pub fn bundle_upload_message(experiment: &[u8; 32], output_hash: &[u8; 32]) -> Vec<u8> {
    [b"replicata:bundle:v1:".as_slice(), experiment, output_hash].concat()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commitment_binds_every_part() {
        let base = commitment(&[1; 32], &[2; 32], &[3; 32]);

        assert_ne!(base, commitment(&[9; 32], &[2; 32], &[3; 32]));
        assert_ne!(base, commitment(&[1; 32], &[9; 32], &[3; 32]));
        assert_ne!(base, commitment(&[1; 32], &[2; 32], &[9; 32]));
        assert_eq!(base, commitment(&[1; 32], &[2; 32], &[3; 32]));
    }

    #[test]
    fn upload_message_layout() {
        let msg = bundle_upload_message(&[7; 32], &[8; 32]);

        assert_eq!(msg.len(), 20 + 64);
        assert!(msg.starts_with(b"replicata:bundle:v1:"));
    }
}

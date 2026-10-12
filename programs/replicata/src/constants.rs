use anchor_lang::prelude::*;

#[constant]
pub const EXPERIMENT_SEED: &[u8] = b"experiment";

#[constant]
pub const CLAIM_SEED: &[u8] = b"claim";

#[constant]
pub const WORKER_SEED: &[u8] = b"worker";

/// Upper bound on claims per experiment. `finalize` pays every revealed claim in a single
/// transaction (3 accounts each), which keeps the transaction under the 1232-byte limit.
#[constant]
pub const MAX_CLAIMS: u8 = 5;

#[constant]
pub const BPS_DENOMINATOR: u16 = 10_000;

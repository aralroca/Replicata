use anchor_lang::prelude::*;

use crate::constants::MAX_CLAIMS;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug, InitSpace)]
pub enum ExperimentStatus {
    /// Accepting claims, commits and reveals.
    Open,
    /// A strict majority of revealed hashes agreed; reward paid out.
    Replicated,
    /// No quorum; reward refunded to the author, bonds refundable.
    Unresolved,
    /// Cancelled by the author before anyone claimed it.
    Cancelled,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug, InitSpace)]
pub enum ClaimState {
    Claimed,
    Committed,
    Revealed,
    /// Agreed with the majority: bond returned plus a share of the reward.
    Paid,
    /// Disagreed with the majority: bond confiscated.
    Slashed,
    /// Bond returned (unresolved experiment).
    Refunded,
}

#[account]
#[derive(InitSpace)]
pub struct Experiment {
    pub authority: Pubkey,
    pub mint: Pubkey,
    pub vault: Pubkey,
    pub nonce: u64,
    /// sha256 of the canonical JobSpec JSON (see `replicata-core`).
    pub spec_hash: [u8; 32],
    /// Raw sha256 of the container image digest.
    pub image_digest: [u8; 32],
    pub reward: u64,
    pub bond: u64,
    pub k: u8,
    pub max_claims: u8,
    pub claims_count: u8,
    pub commits_count: u8,
    pub reveals_count: u8,
    pub commit_deadline: i64,
    pub reveal_deadline: i64,
    pub status: ExperimentStatus,
    pub result_hash: [u8; 32],
    pub agree_count: u8,
    /// Workers whose result matched the majority (first `agree_count` entries).
    pub replicators: [Pubkey; MAX_CLAIMS as usize],
    /// Price (token base units) of one x402 access to the verified result.
    pub access_price: u64,
    pub author_share_bps: u16,
    /// Tokens in the vault that are owed (reward + bonds). Anything above is royalties.
    pub escrowed: u64,
    pub royalties_paid: u64,
    pub created_at: i64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct Claim {
    pub experiment: Pubkey,
    pub worker: Pubkey,
    pub commitment: [u8; 32],
    pub revealed_hash: [u8; 32],
    pub salt: [u8; 32],
    pub state: ClaimState,
    pub claimed_at: i64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct WorkerProfile {
    pub worker: Pubkey,
    pub claims: u32,
    pub ok: u32,
    pub bad: u32,
    pub earned: u64,
    pub bump: u8,
}

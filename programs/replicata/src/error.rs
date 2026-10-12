use anchor_lang::prelude::*;

#[error_code]
pub enum ReplicataError {
    #[msg("Reward must be greater than zero")]
    ZeroReward,
    #[msg("Invalid replication parameters: need 1 <= k <= max_claims <= MAX_CLAIMS")]
    InvalidReplicationParams,
    #[msg("Author share must be at most 10000 basis points")]
    InvalidAuthorShare,
    #[msg("Deadlines must satisfy now < commit_deadline < reveal_deadline")]
    InvalidDeadlines,
    #[msg("The experiment is not open")]
    ExperimentNotOpen,
    #[msg("The experiment already has claims and cannot be cancelled")]
    HasClaims,
    #[msg("All claim slots are taken")]
    ClaimsFull,
    #[msg("The commit window is closed")]
    CommitWindowClosed,
    #[msg("The reveal window is not open")]
    RevealWindowClosed,
    #[msg("The claim is not in the expected state")]
    InvalidClaimState,
    #[msg("The revealed hash and salt do not match the commitment")]
    CommitmentMismatch,
    #[msg("The experiment cannot be finalized yet")]
    NotReadyToFinalize,
    #[msg("Remaining accounts do not match the revealed claims")]
    InvalidRemainingAccounts,
    #[msg("The experiment has not been replicated")]
    NotReplicated,
    #[msg("There are no royalties to distribute")]
    NoRoyalties,
    #[msg("Bonds can only be withdrawn from unresolved experiments")]
    BondNotWithdrawable,
    #[msg("Arithmetic overflow")]
    Overflow,
}

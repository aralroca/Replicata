use anchor_lang::prelude::*;

use crate::constants::CLAIM_SEED;
use crate::error::ReplicataError;
use crate::state::{Claim, ClaimState, Experiment, ExperimentStatus};

#[derive(Accounts)]
pub struct RevealResult<'info> {
    pub worker: Signer<'info>,
    #[account(mut)]
    pub experiment: Account<'info, Experiment>,
    #[account(
        mut,
        seeds = [CLAIM_SEED, experiment.key().as_ref(), worker.key().as_ref()],
        bump = claim.bump,
        has_one = worker,
        has_one = experiment,
    )]
    pub claim: Account<'info, Claim>,
}

/// Reveals open once the commit window has closed, or as soon as every slot is taken and
/// committed (nobody else can join), and close at `reveal_deadline`.
pub fn reveal_window_open(experiment: &Experiment, now: i64) -> bool {
    let all_committed =
        experiment.claims_count == experiment.max_claims && experiment.commits_count == experiment.claims_count;

    now <= experiment.reveal_deadline && (now > experiment.commit_deadline || all_committed)
}

pub fn commitment(output_hash: &[u8; 32], salt: &[u8; 32], worker: &Pubkey) -> [u8; 32] {
    solana_sha256_hasher::hashv(&[output_hash, salt, worker.as_ref()]).to_bytes()
}

pub fn handle_reveal(ctx: Context<RevealResult>, output_hash: [u8; 32], salt: [u8; 32]) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let experiment = &mut ctx.accounts.experiment;
    let claim = &mut ctx.accounts.claim;
    require!(experiment.status == ExperimentStatus::Open, ReplicataError::ExperimentNotOpen);
    require!(reveal_window_open(experiment, now), ReplicataError::RevealWindowClosed);
    require!(claim.state == ClaimState::Committed, ReplicataError::InvalidClaimState);
    require!(commitment(&output_hash, &salt, &claim.worker) == claim.commitment, ReplicataError::CommitmentMismatch);

    claim.revealed_hash = output_hash;
    claim.salt = salt;
    claim.state = ClaimState::Revealed;
    experiment.reveals_count += 1;
    Ok(())
}

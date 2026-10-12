use anchor_lang::prelude::*;

use crate::constants::CLAIM_SEED;
use crate::error::ReplicataError;
use crate::state::{Claim, ClaimState, Experiment, ExperimentStatus};

#[derive(Accounts)]
pub struct CommitResult<'info> {
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

/// Stores `sha256(output_hash ‖ salt ‖ worker)` without revealing the output hash.
pub fn handle_commit(ctx: Context<CommitResult>, commitment: [u8; 32]) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let experiment = &mut ctx.accounts.experiment;
    let claim = &mut ctx.accounts.claim;
    require!(experiment.status == ExperimentStatus::Open, ReplicataError::ExperimentNotOpen);
    require!(now <= experiment.commit_deadline, ReplicataError::CommitWindowClosed);
    require!(claim.state == ClaimState::Claimed, ReplicataError::InvalidClaimState);

    claim.commitment = commitment;
    claim.state = ClaimState::Committed;
    experiment.commits_count += 1;
    Ok(())
}

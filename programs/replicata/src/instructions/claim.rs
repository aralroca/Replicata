use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

use crate::constants::{CLAIM_SEED, WORKER_SEED};
use crate::error::ReplicataError;
use crate::state::{Claim, ClaimState, Experiment, ExperimentStatus, WorkerProfile};
use crate::utils::pay_into_vault;

#[derive(Accounts)]
pub struct ClaimExperiment<'info> {
    #[account(mut)]
    pub worker: Signer<'info>,
    #[account(mut, has_one = mint, has_one = vault)]
    pub experiment: Account<'info, Experiment>,
    #[account(
        init,
        payer = worker,
        space = 8 + Claim::INIT_SPACE,
        seeds = [CLAIM_SEED, experiment.key().as_ref(), worker.key().as_ref()],
        bump,
    )]
    pub claim: Account<'info, Claim>,
    #[account(
        init_if_needed,
        payer = worker,
        space = 8 + WorkerProfile::INIT_SPACE,
        seeds = [WORKER_SEED, worker.key().as_ref()],
        bump,
    )]
    pub worker_profile: Account<'info, WorkerProfile>,
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(mut)]
    pub vault: InterfaceAccount<'info, TokenAccount>,
    #[account(mut, token::mint = mint, token::authority = worker, token::token_program = token_program)]
    pub worker_token_account: InterfaceAccount<'info, TokenAccount>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

/// A worker reserves one replication slot by locking `bond` in the vault.
pub fn handle_claim(ctx: Context<ClaimExperiment>) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let experiment = &ctx.accounts.experiment;
    require!(experiment.status == ExperimentStatus::Open, ReplicataError::ExperimentNotOpen);
    require!(now <= experiment.commit_deadline, ReplicataError::CommitWindowClosed);
    require!(experiment.claims_count < experiment.max_claims, ReplicataError::ClaimsFull);
    let bond = experiment.bond;

    pay_into_vault(
        &ctx.accounts.token_program.to_account_info(),
        &ctx.accounts.worker_token_account,
        &ctx.accounts.vault,
        &ctx.accounts.mint,
        &ctx.accounts.worker,
        bond,
    )?;

    let experiment = &mut ctx.accounts.experiment;
    experiment.claims_count += 1;
    experiment.escrowed = experiment.escrowed.checked_add(bond).ok_or(ReplicataError::Overflow)?;

    ctx.accounts.claim.set_inner(Claim {
        experiment: experiment.key(),
        worker: ctx.accounts.worker.key(),
        commitment: [0; 32],
        revealed_hash: [0; 32],
        salt: [0; 32],
        state: ClaimState::Claimed,
        claimed_at: now,
        bump: ctx.bumps.claim,
    });

    let profile = &mut ctx.accounts.worker_profile;
    profile.worker = ctx.accounts.worker.key();
    profile.bump = ctx.bumps.worker_profile;
    profile.claims = profile.claims.saturating_add(1);
    Ok(())
}

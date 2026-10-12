use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

use crate::constants::CLAIM_SEED;
use crate::error::ReplicataError;
use crate::state::{Claim, ClaimState, Experiment, ExperimentStatus};
use crate::utils::Vault;

#[derive(Accounts)]
pub struct WithdrawBond<'info> {
    pub worker: Signer<'info>,
    #[account(mut, has_one = mint, has_one = vault)]
    pub experiment: Account<'info, Experiment>,
    #[account(
        mut,
        seeds = [CLAIM_SEED, experiment.key().as_ref(), worker.key().as_ref()],
        bump = claim.bump,
        has_one = worker,
        has_one = experiment,
    )]
    pub claim: Account<'info, Claim>,
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(mut)]
    pub vault: InterfaceAccount<'info, TokenAccount>,
    #[account(mut, token::mint = mint, token::authority = worker, token::token_program = token_program)]
    pub worker_token_account: InterfaceAccount<'info, TokenAccount>,
    pub token_program: Interface<'info, TokenInterface>,
}

/// After an `Unresolved` outcome, a worker that never revealed gets its bond back.
pub fn handle_withdraw_bond(ctx: Context<WithdrawBond>) -> Result<()> {
    let experiment = &ctx.accounts.experiment;
    require!(experiment.status == ExperimentStatus::Unresolved, ReplicataError::BondNotWithdrawable);
    require!(
        matches!(ctx.accounts.claim.state, ClaimState::Claimed | ClaimState::Committed),
        ReplicataError::InvalidClaimState
    );
    let bond = experiment.bond;
    let token_program = ctx.accounts.token_program.to_account_info();

    Vault { experiment, vault: &ctx.accounts.vault, mint: &ctx.accounts.mint, token_program: &token_program }
        .pay(&ctx.accounts.worker_token_account.to_account_info(), bond)?;

    ctx.accounts.claim.state = ClaimState::Refunded;
    let experiment = &mut ctx.accounts.experiment;
    experiment.escrowed = experiment.escrowed.checked_sub(bond).ok_or(ReplicataError::Overflow)?;
    Ok(())
}

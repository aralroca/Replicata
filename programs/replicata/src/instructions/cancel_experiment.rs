use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

use crate::error::ReplicataError;
use crate::state::{Experiment, ExperimentStatus};
use crate::utils::Vault;

#[derive(Accounts)]
pub struct CancelExperiment<'info> {
    pub authority: Signer<'info>,
    #[account(mut, has_one = authority, has_one = mint, has_one = vault)]
    pub experiment: Account<'info, Experiment>,
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(mut)]
    pub vault: InterfaceAccount<'info, TokenAccount>,
    #[account(mut, token::mint = mint, token::authority = authority, token::token_program = token_program)]
    pub authority_token_account: InterfaceAccount<'info, TokenAccount>,
    pub token_program: Interface<'info, TokenInterface>,
}

/// The author can take the reward back only while nobody has claimed the experiment.
pub fn handle_cancel_experiment(ctx: Context<CancelExperiment>) -> Result<()> {
    let experiment = &ctx.accounts.experiment;
    require!(experiment.status == ExperimentStatus::Open, ReplicataError::ExperimentNotOpen);
    require!(experiment.claims_count == 0, ReplicataError::HasClaims);
    let refund = experiment.escrowed;

    Vault {
        experiment,
        vault: &ctx.accounts.vault,
        mint: &ctx.accounts.mint,
        token_program: &ctx.accounts.token_program.to_account_info(),
    }
    .pay(&ctx.accounts.authority_token_account.to_account_info(), refund)?;

    let experiment = &mut ctx.accounts.experiment;
    experiment.escrowed = 0;
    experiment.status = ExperimentStatus::Cancelled;
    Ok(())
}

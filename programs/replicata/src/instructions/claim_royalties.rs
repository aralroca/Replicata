use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

use crate::constants::BPS_DENOMINATOR;
use crate::error::ReplicataError;
use crate::state::{Experiment, ExperimentStatus};
use crate::utils::{load_token_account, Vault};

/// Permissionless: distributes everything in the vault above `escrowed` (x402 access
/// payments land in the vault because `payTo` is the experiment PDA).
///
/// `remaining_accounts`: one token account per replicator, in `experiment.replicators` order.
#[derive(Accounts)]
pub struct ClaimRoyalties<'info> {
    #[account(mut, has_one = mint, has_one = vault)]
    pub experiment: Account<'info, Experiment>,
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(mut)]
    pub vault: InterfaceAccount<'info, TokenAccount>,
    #[account(
        mut,
        token::mint = mint,
        token::authority = experiment.authority,
        token::token_program = token_program,
    )]
    pub authority_token_account: InterfaceAccount<'info, TokenAccount>,
    pub token_program: Interface<'info, TokenInterface>,
}

/// `(author_amount, per_replicator)`; the indivisible remainder goes to the author.
pub fn split_royalties(available: u64, author_share_bps: u16, replicators: u8) -> (u64, u64) {
    let author = (available as u128 * author_share_bps as u128 / BPS_DENOMINATOR as u128) as u64;
    let rest = available - author;
    let per = rest / replicators as u64;

    (author + rest % replicators as u64, per)
}

pub fn handle_claim_royalties<'info>(ctx: Context<'info, ClaimRoyalties<'info>>) -> Result<()> {
    let experiment = &ctx.accounts.experiment;
    require!(experiment.status == ExperimentStatus::Replicated, ReplicataError::NotReplicated);
    let agree = experiment.agree_count;
    require!(ctx.remaining_accounts.len() == agree as usize, ReplicataError::InvalidRemainingAccounts);
    let available = ctx.accounts.vault.amount.saturating_sub(experiment.escrowed);
    require!(available > 0, ReplicataError::NoRoyalties);
    let (author_amount, per) = split_royalties(available, experiment.author_share_bps, agree);
    let token_program = ctx.accounts.token_program.to_account_info();
    let payer =
        Vault { experiment, vault: &ctx.accounts.vault, mint: &ctx.accounts.mint, token_program: &token_program };

    for (info, worker) in ctx.remaining_accounts.iter().zip(experiment.replicators.iter()) {
        load_token_account(info, worker, &experiment.mint)?;
        payer.pay(info, per)?;
    }
    payer.pay(&ctx.accounts.authority_token_account.to_account_info(), author_amount)?;

    let experiment = &mut ctx.accounts.experiment;
    experiment.royalties_paid = experiment.royalties_paid.checked_add(available).ok_or(ReplicataError::Overflow)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::split_royalties;

    #[test]
    fn splits_exactly() {
        assert_eq!(split_royalties(1_000_000, 7_000, 3), (700_000, 100_000));
        assert_eq!(split_royalties(1_000_001, 7_000, 3), (700_000 + 1, 100_000));
        assert_eq!(split_royalties(10, 0, 3), (1, 3));
        assert_eq!(split_royalties(10, 10_000, 3), (10, 0));
        assert_eq!(split_royalties(u64::MAX, 5_000, 1).0 + split_royalties(u64::MAX, 5_000, 1).1, u64::MAX);
    }
}

//! Replicata: an escrowed bounty for replicating a computational experiment.
//!
//! Flow: `create_experiment` (reward into the vault) → workers `claim` (bond) → `commit`
//! (hidden output hash) → `reveal` → permissionless `finalize` (strict majority pays the
//! agreeing workers and confiscates the rest) → `claim_royalties` distributes x402 access
//! payments that land in the vault. `cancel_experiment` and `withdraw_bond` cover the
//! unhappy paths.

pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;
pub mod utils;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("G9Ssnp8rjg7gpi4Sm3EZLoC73z9pdXd9etJzUDH39Wj9");

#[program]
pub mod replicata {
    use super::*;

    pub fn create_experiment(ctx: Context<CreateExperiment>, args: CreateExperimentArgs) -> Result<()> {
        instructions::create_experiment::handle_create_experiment(ctx, args)
    }

    pub fn cancel_experiment(ctx: Context<CancelExperiment>) -> Result<()> {
        instructions::cancel_experiment::handle_cancel_experiment(ctx)
    }

    pub fn claim(ctx: Context<ClaimExperiment>) -> Result<()> {
        instructions::claim::handle_claim(ctx)
    }

    pub fn commit(ctx: Context<CommitResult>, commitment: [u8; 32]) -> Result<()> {
        instructions::commit::handle_commit(ctx, commitment)
    }

    pub fn reveal(ctx: Context<RevealResult>, output_hash: [u8; 32], salt: [u8; 32]) -> Result<()> {
        instructions::reveal::handle_reveal(ctx, output_hash, salt)
    }

    pub fn finalize<'info>(ctx: Context<'info, Finalize<'info>>) -> Result<()> {
        instructions::finalize::handle_finalize(ctx)
    }

    pub fn claim_royalties<'info>(ctx: Context<'info, ClaimRoyalties<'info>>) -> Result<()> {
        instructions::claim_royalties::handle_claim_royalties(ctx)
    }

    pub fn withdraw_bond(ctx: Context<WithdrawBond>) -> Result<()> {
        instructions::withdraw_bond::handle_withdraw_bond(ctx)
    }
}

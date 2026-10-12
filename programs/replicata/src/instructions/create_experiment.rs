use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

use crate::constants::{BPS_DENOMINATOR, EXPERIMENT_SEED, MAX_CLAIMS};
use crate::error::ReplicataError;
use crate::state::{Experiment, ExperimentStatus};
use crate::utils::pay_into_vault;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct CreateExperimentArgs {
    pub nonce: u64,
    pub spec_hash: [u8; 32],
    pub image_digest: [u8; 32],
    pub reward: u64,
    pub bond: u64,
    pub k: u8,
    pub max_claims: u8,
    pub commit_deadline: i64,
    pub reveal_deadline: i64,
    pub access_price: u64,
    pub author_share_bps: u16,
}

#[derive(Accounts)]
#[instruction(args: CreateExperimentArgs)]
pub struct CreateExperiment<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        init,
        payer = authority,
        space = 8 + Experiment::INIT_SPACE,
        seeds = [EXPERIMENT_SEED, authority.key().as_ref(), &args.nonce.to_le_bytes()],
        bump,
    )]
    pub experiment: Account<'info, Experiment>,
    #[account(mint::token_program = token_program)]
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(
        init,
        payer = authority,
        associated_token::mint = mint,
        associated_token::authority = experiment,
        associated_token::token_program = token_program,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,
    #[account(mut, token::mint = mint, token::authority = authority, token::token_program = token_program)]
    pub authority_token_account: InterfaceAccount<'info, TokenAccount>,
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

fn validate(args: &CreateExperimentArgs, now: i64) -> Result<()> {
    require!(args.reward > 0, ReplicataError::ZeroReward);
    require!(
        args.k >= 1 && args.k <= args.max_claims && args.max_claims <= MAX_CLAIMS,
        ReplicataError::InvalidReplicationParams
    );
    require!(args.author_share_bps <= BPS_DENOMINATOR, ReplicataError::InvalidAuthorShare);
    require!(
        now < args.commit_deadline && args.commit_deadline < args.reveal_deadline,
        ReplicataError::InvalidDeadlines
    );
    Ok(())
}

pub fn handle_create_experiment(ctx: Context<CreateExperiment>, args: CreateExperimentArgs) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    validate(&args, now)?;

    ctx.accounts.experiment.set_inner(Experiment {
        authority: ctx.accounts.authority.key(),
        mint: ctx.accounts.mint.key(),
        vault: ctx.accounts.vault.key(),
        nonce: args.nonce,
        spec_hash: args.spec_hash,
        image_digest: args.image_digest,
        reward: args.reward,
        bond: args.bond,
        k: args.k,
        max_claims: args.max_claims,
        claims_count: 0,
        commits_count: 0,
        reveals_count: 0,
        commit_deadline: args.commit_deadline,
        reveal_deadline: args.reveal_deadline,
        status: ExperimentStatus::Open,
        result_hash: [0; 32],
        agree_count: 0,
        replicators: [Pubkey::default(); MAX_CLAIMS as usize],
        access_price: args.access_price,
        author_share_bps: args.author_share_bps,
        escrowed: args.reward,
        royalties_paid: 0,
        created_at: now,
        bump: ctx.bumps.experiment,
    });

    pay_into_vault(
        &ctx.accounts.token_program.to_account_info(),
        &ctx.accounts.authority_token_account,
        &ctx.accounts.vault,
        &ctx.accounts.mint,
        &ctx.accounts.authority,
        args.reward,
    )
}

use anchor_lang::prelude::*;
use anchor_lang::AccountsExit;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

use crate::error::ReplicataError;
use crate::state::{Claim, ClaimState, Experiment, ExperimentStatus, WorkerProfile};
use crate::utils::{load_token_account, Vault};

/// Permissionless: anyone (usually the coordinator's crank) can settle an experiment.
///
/// `remaining_accounts` must contain, for **every** revealed claim, the triple
/// `[claim (mut), worker token account (mut), worker profile (mut)]`.
#[derive(Accounts)]
pub struct Finalize<'info> {
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

struct Revealed<'info> {
    claim: Account<'info, Claim>,
    token_account: &'info AccountInfo<'info>,
    profile: Account<'info, WorkerProfile>,
}

/// Ready when the reveal window is over, or when every claim has revealed and no new
/// claim can arrive (all slots taken, or the commit window closed).
pub fn ready_to_finalize(experiment: &Experiment, now: i64) -> bool {
    let all_revealed = experiment.reveals_count == experiment.claims_count;
    let no_more_claims = experiment.claims_count == experiment.max_claims || now > experiment.commit_deadline;

    now > experiment.reveal_deadline || (all_revealed && no_more_claims && experiment.claims_count > 0)
}

fn load_revealed<'info>(
    experiment: &Experiment,
    key: &Pubkey,
    remaining: &'info [AccountInfo<'info>],
) -> Result<Vec<Revealed<'info>>> {
    require!(remaining.len() == 3 * experiment.reveals_count as usize, ReplicataError::InvalidRemainingAccounts);
    let mut out: Vec<Revealed<'info>> = Vec::with_capacity(experiment.reveals_count as usize);

    for triple in remaining.chunks(3) {
        let claim = Account::<Claim>::try_from(&triple[0])?;
        require_keys_eq!(claim.experiment, *key, ReplicataError::InvalidRemainingAccounts);
        require!(claim.state == ClaimState::Revealed, ReplicataError::InvalidRemainingAccounts);
        require!(out.iter().all(|r| r.claim.key() != claim.key()), ReplicataError::InvalidRemainingAccounts);
        load_token_account(&triple[1], &claim.worker, &experiment.mint)?;
        let profile = Account::<WorkerProfile>::try_from(&triple[2])?;
        require_keys_eq!(profile.worker, claim.worker, ReplicataError::InvalidRemainingAccounts);
        out.push(Revealed { claim, token_account: &triple[1], profile });
    }
    Ok(out)
}

/// Most frequent revealed hash and how many claims revealed it.
fn majority(revealed: &[Revealed]) -> ([u8; 32], u8) {
    revealed
        .iter()
        .map(|r| {
            let count = revealed.iter().filter(|o| o.claim.revealed_hash == r.claim.revealed_hash).count();
            (r.claim.revealed_hash, count as u8)
        })
        .max_by_key(|(_, count)| *count)
        .unwrap_or(([0; 32], 0))
}

pub fn handle_finalize<'info>(ctx: Context<'info, Finalize<'info>>) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let experiment = &ctx.accounts.experiment;
    require!(experiment.status == ExperimentStatus::Open, ReplicataError::ExperimentNotOpen);
    require!(ready_to_finalize(experiment, now), ReplicataError::NotReadyToFinalize);
    let mut revealed = load_revealed(experiment, &experiment.key(), ctx.remaining_accounts)?;
    let (hash, agree) = majority(&revealed);
    let quorum = experiment.reveals_count >= experiment.k && (agree as u16) * 2 > experiment.reveals_count as u16;

    if quorum {
        settle_replicated(
            &mut ctx.accounts.experiment,
            &ctx.accounts.vault,
            &ctx.accounts.mint,
            &ctx.accounts.token_program,
            &ctx.accounts.authority_token_account,
            &mut revealed,
            hash,
            agree,
        )?;
    } else {
        settle_unresolved(
            &mut ctx.accounts.experiment,
            &ctx.accounts.vault,
            &ctx.accounts.mint,
            &ctx.accounts.token_program,
            &ctx.accounts.authority_token_account,
            &mut revealed,
        )?;
    }
    revealed.iter().try_for_each(|r| {
        r.claim.exit(&crate::ID)?;
        r.profile.exit(&crate::ID)
    })
}

/// Agreeing workers get their bond back plus an equal share of
/// `reward + every other bond` (disagreeing and unrevealed bonds are confiscated).
/// The indivisible remainder goes back to the author.
#[allow(clippy::too_many_arguments)]
fn settle_replicated<'info>(
    experiment: &mut Account<'info, Experiment>,
    vault: &InterfaceAccount<'info, TokenAccount>,
    mint: &InterfaceAccount<'info, Mint>,
    token_program: &Interface<'info, TokenInterface>,
    author: &InterfaceAccount<'info, TokenAccount>,
    revealed: &mut [Revealed<'info>],
    hash: [u8; 32],
    agree: u8,
) -> Result<()> {
    let bond = experiment.bond;
    let returned_bonds = bond.checked_mul(agree as u64).ok_or(ReplicataError::Overflow)?;
    let distributable = experiment.escrowed.checked_sub(returned_bonds).ok_or(ReplicataError::Overflow)?;
    let share = distributable / agree as u64;
    let dust = distributable % agree as u64;
    let token_program = token_program.to_account_info();
    let payer = Vault { experiment, vault, mint, token_program: &token_program };
    let mut replicators = experiment.replicators;
    let mut slot = 0usize;

    for r in revealed.iter_mut() {
        if r.claim.revealed_hash != hash {
            r.claim.state = ClaimState::Slashed;
            r.profile.bad = r.profile.bad.saturating_add(1);
            continue;
        }
        payer.pay(r.token_account, bond + share)?;
        r.claim.state = ClaimState::Paid;
        r.profile.ok = r.profile.ok.saturating_add(1);
        r.profile.earned = r.profile.earned.saturating_add(share);
        replicators[slot] = r.claim.worker;
        slot += 1;
    }
    payer.pay(&author.to_account_info(), dust)?;

    experiment.replicators = replicators;
    experiment.result_hash = hash;
    experiment.agree_count = agree;
    experiment.escrowed = 0;
    experiment.status = ExperimentStatus::Replicated;
    Ok(())
}

/// No quorum: refund the reward and every revealed bond. Bonds of claims that never
/// revealed stay escrowed until their worker calls `withdraw_bond`.
fn settle_unresolved<'info>(
    experiment: &mut Account<'info, Experiment>,
    vault: &InterfaceAccount<'info, TokenAccount>,
    mint: &InterfaceAccount<'info, Mint>,
    token_program: &Interface<'info, TokenInterface>,
    author: &InterfaceAccount<'info, TokenAccount>,
    revealed: &mut [Revealed<'info>],
) -> Result<()> {
    let token_program = token_program.to_account_info();
    let payer = Vault { experiment, vault, mint, token_program: &token_program };
    let bond = experiment.bond;
    let reward = experiment.reward;

    payer.pay(&author.to_account_info(), reward)?;
    for r in revealed.iter_mut() {
        payer.pay(r.token_account, bond)?;
        r.claim.state = ClaimState::Refunded;
    }
    let refunded =
        bond.checked_mul(revealed.len() as u64).and_then(|b| b.checked_add(reward)).ok_or(ReplicataError::Overflow)?;

    experiment.escrowed = experiment.escrowed.checked_sub(refunded).ok_or(ReplicataError::Overflow)?;
    experiment.status = ExperimentStatus::Unresolved;
    Ok(())
}

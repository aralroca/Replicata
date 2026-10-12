//! Thin Rust client for the Replicata program, built on the program crate's own
//! Anchor-generated `accounts::*` / `instruction::*` (no hand-written instruction layouts).
//! Used by the coordinator, the worker and the LiteSVM tests.

use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_lang::{AccountDeserialize, InstructionData, ToAccountMetas};
use spl_associated_token_account_interface::address::get_associated_token_address_with_program_id;

pub use anchor_lang::prelude::Pubkey;
pub use anchor_lang::solana_program::instruction::Instruction as ProgramInstruction;
pub use replicata::state::{Claim, ClaimState, Experiment, ExperimentStatus, WorkerProfile};
pub use replicata::{CreateExperimentArgs, ID as PROGRAM_ID, MAX_CLAIMS};

/// Classic SPL Token (USDC on devnet is a classic mint).
pub const TOKEN_PROGRAM_ID: Pubkey = spl_token_interface::ID;

pub fn experiment_pda(authority: &Pubkey, nonce: u64) -> Pubkey {
    Pubkey::find_program_address(&[replicata::EXPERIMENT_SEED, authority.as_ref(), &nonce.to_le_bytes()], &PROGRAM_ID).0
}

pub fn claim_pda(experiment: &Pubkey, worker: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[replicata::CLAIM_SEED, experiment.as_ref(), worker.as_ref()], &PROGRAM_ID).0
}

pub fn profile_pda(worker: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[replicata::WORKER_SEED, worker.as_ref()], &PROGRAM_ID).0
}

pub fn ata(owner: &Pubkey, mint: &Pubkey) -> Pubkey {
    get_associated_token_address_with_program_id(owner, mint, &TOKEN_PROGRAM_ID)
}

/// Idempotent "create ATA" (used so a payout destination always exists).
pub fn create_ata_idempotent(payer: &Pubkey, owner: &Pubkey, mint: &Pubkey) -> Instruction {
    spl_associated_token_account_interface::instruction::create_associated_token_account_idempotent(
        payer,
        owner,
        mint,
        &TOKEN_PROGRAM_ID,
    )
}

fn ix(accounts: impl ToAccountMetas, data: impl InstructionData) -> Instruction {
    Instruction { program_id: PROGRAM_ID, accounts: accounts.to_account_metas(None), data: data.data() }
}

/// Returns the experiment address and the instruction.
pub fn create_experiment(authority: &Pubkey, mint: &Pubkey, args: CreateExperimentArgs) -> (Pubkey, Instruction) {
    let experiment = experiment_pda(authority, args.nonce);
    let accounts = replicata::accounts::CreateExperiment {
        authority: *authority,
        experiment,
        mint: *mint,
        vault: ata(&experiment, mint),
        authority_token_account: ata(authority, mint),
        token_program: TOKEN_PROGRAM_ID,
        associated_token_program: spl_associated_token_account_interface::program::ID,
        system_program: anchor_lang::system_program::ID,
    };

    (experiment, ix(accounts, replicata::instruction::CreateExperiment { args }))
}

pub fn cancel_experiment(experiment: &Pubkey, exp: &Experiment) -> Instruction {
    let accounts = replicata::accounts::CancelExperiment {
        authority: exp.authority,
        experiment: *experiment,
        mint: exp.mint,
        vault: exp.vault,
        authority_token_account: ata(&exp.authority, &exp.mint),
        token_program: TOKEN_PROGRAM_ID,
    };

    ix(accounts, replicata::instruction::CancelExperiment {})
}

pub fn claim(worker: &Pubkey, experiment: &Pubkey, exp: &Experiment) -> Instruction {
    let accounts = replicata::accounts::ClaimExperiment {
        worker: *worker,
        experiment: *experiment,
        claim: claim_pda(experiment, worker),
        worker_profile: profile_pda(worker),
        mint: exp.mint,
        vault: exp.vault,
        worker_token_account: ata(worker, &exp.mint),
        token_program: TOKEN_PROGRAM_ID,
        system_program: anchor_lang::system_program::ID,
    };

    ix(accounts, replicata::instruction::Claim {})
}

pub fn commit(worker: &Pubkey, experiment: &Pubkey, commitment: [u8; 32]) -> Instruction {
    let accounts = replicata::accounts::CommitResult { worker: *worker, experiment: *experiment, claim: claim_pda(experiment, worker) };

    ix(accounts, replicata::instruction::Commit { commitment })
}

pub fn reveal(worker: &Pubkey, experiment: &Pubkey, output_hash: [u8; 32], salt: [u8; 32]) -> Instruction {
    let accounts = replicata::accounts::RevealResult { worker: *worker, experiment: *experiment, claim: claim_pda(experiment, worker) };

    ix(accounts, replicata::instruction::Reveal { output_hash, salt })
}

/// `revealed_workers` must list **every** worker whose claim is in `Revealed` state.
pub fn finalize(experiment: &Pubkey, exp: &Experiment, revealed_workers: &[Pubkey]) -> Instruction {
    let accounts = replicata::accounts::Finalize {
        experiment: *experiment,
        mint: exp.mint,
        vault: exp.vault,
        authority_token_account: ata(&exp.authority, &exp.mint),
        token_program: TOKEN_PROGRAM_ID,
    };
    let mut instruction = ix(accounts, replicata::instruction::Finalize {});

    instruction.accounts.extend(revealed_workers.iter().flat_map(|w| {
        [
            AccountMeta::new(claim_pda(experiment, w), false),
            AccountMeta::new(ata(w, &exp.mint), false),
            AccountMeta::new(profile_pda(w), false),
        ]
    }));
    instruction
}

pub fn claim_royalties(experiment: &Pubkey, exp: &Experiment) -> Instruction {
    let accounts = replicata::accounts::ClaimRoyalties {
        experiment: *experiment,
        mint: exp.mint,
        vault: exp.vault,
        authority_token_account: ata(&exp.authority, &exp.mint),
        token_program: TOKEN_PROGRAM_ID,
    };
    let mut instruction = ix(accounts, replicata::instruction::ClaimRoyalties {});

    instruction
        .accounts
        .extend(replicators(exp).iter().map(|w| AccountMeta::new(ata(w, &exp.mint), false)));
    instruction
}

pub fn withdraw_bond(worker: &Pubkey, experiment: &Pubkey, exp: &Experiment) -> Instruction {
    let accounts = replicata::accounts::WithdrawBond {
        worker: *worker,
        experiment: *experiment,
        claim: claim_pda(experiment, worker),
        mint: exp.mint,
        vault: exp.vault,
        worker_token_account: ata(worker, &exp.mint),
        token_program: TOKEN_PROGRAM_ID,
    };

    ix(accounts, replicata::instruction::WithdrawBond {})
}

pub fn replicators(exp: &Experiment) -> &[Pubkey] {
    &exp.replicators[..exp.agree_count as usize]
}

/// Decodes any Replicata account (checks the Anchor discriminator).
pub fn decode<T: AccountDeserialize>(data: &[u8]) -> Option<T> {
    T::try_deserialize(&mut &data[..]).ok()
}

/// Anchor account discriminators, for `getProgramAccounts` memcmp filters.
pub mod discriminator {
    use anchor_lang::Discriminator;

    pub fn experiment() -> &'static [u8] {
        super::Experiment::DISCRIMINATOR
    }

    pub fn claim() -> &'static [u8] {
        super::Claim::DISCRIMINATOR
    }

    pub fn worker_profile() -> &'static [u8] {
        super::WorkerProfile::DISCRIMINATOR
    }
}

/// Phase of an open experiment at time `now` (mirrors the on-chain windows).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Claiming,
    Revealing,
    Finalizable,
    Settled,
}

pub fn phase(exp: &Experiment, now: i64) -> Phase {
    if exp.status != ExperimentStatus::Open {
        return Phase::Settled;
    }
    if replicata::instructions::finalize::ready_to_finalize(exp, now) {
        return Phase::Finalizable;
    }
    if replicata::instructions::reveal::reveal_window_open(exp, now) {
        return Phase::Revealing;
    }
    Phase::Claiming
}

//! LiteSVM harness for the Replicata program.
//!
//! Anchor 1.2 and LiteSVM 0.18 share the same `solana-address`/`solana-instruction` types
//! (semver trick), so Anchor-built instructions are sent to LiteSVM directly.

use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_lang::{AccountDeserialize, InstructionData, ToAccountMetas};
use litesvm::types::FailedTransactionMetadata;
use litesvm::LiteSVM;
use litesvm_token::{CreateAssociatedTokenAccount, CreateMint, MintTo};
use replicata::state::{Claim, Experiment, WorkerProfile};
use replicata::CreateExperimentArgs;
use solana_clock::Clock;
use solana_instruction_error::InstructionError;
use solana_keypair::Keypair;
use solana_program_pack::Pack;
use solana_signer::Signer;
use solana_transaction::Transaction;
use solana_transaction_error::TransactionError;
use spl_associated_token_account_interface::address::get_associated_token_address;

pub use replicata::error::ReplicataError;

pub const START: i64 = 1_700_000_000;
pub const USDC: u64 = 1_000_000;

const PROGRAM_SO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../target/deploy/replicata.so");

pub type TxResult = Result<(), FailedTransactionMetadata>;

/// Anchor custom error code of a program error.
pub fn code(err: ReplicataError) -> u32 {
    err as u32 + anchor_lang::error::ERROR_CODE_OFFSET
}

/// Custom error code returned by a failed transaction, if any.
pub fn custom_code(result: &TxResult) -> Option<u32> {
    match result {
        Err(FailedTransactionMetadata { err: TransactionError::InstructionError(_, InstructionError::Custom(c)), .. }) => Some(*c),
        _ => None,
    }
}

#[track_caller]
pub fn assert_program_err(result: TxResult, err: ReplicataError) {
    assert_eq!(custom_code(&result), Some(code(err)), "expected {err:?}, got {result:?}");
}

#[derive(Clone, Debug)]
pub struct Params {
    pub reward: u64,
    pub bond: u64,
    pub k: u8,
    pub max_claims: u8,
    pub commit_in: i64,
    pub reveal_in: i64,
    pub access_price: u64,
    pub author_share_bps: u16,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            reward: 3 * USDC,
            bond: USDC,
            k: 3,
            max_claims: 3,
            commit_in: 100,
            reveal_in: 200,
            access_price: 10_000,
            author_share_bps: 7_000,
        }
    }
}

pub struct Env {
    pub svm: LiteSVM,
    pub mint: Pubkey,
    pub mint_authority: Keypair,
    pub author: Keypair,
    pub now: i64,
    nonce: u64,
}

impl Default for Env {
    fn default() -> Self {
        Self::new()
    }
}

impl Env {
    pub fn new() -> Self {
        let mut svm = LiteSVM::new();
        svm.add_program_from_file(replicata::ID, PROGRAM_SO)
            .expect("target/deploy/replicata.so missing: run `anchor build` first");
        let mint_authority = Keypair::new();
        svm.airdrop(&mint_authority.pubkey(), 100_000_000_000).unwrap();
        let mint = CreateMint::new(&mut svm, &mint_authority).decimals(6).send().unwrap();
        let author = Keypair::new();
        let mut env = Self { svm, mint, mint_authority, author, now: START, nonce: 0 };

        env.set_time(START);
        let author = env.author.insecure_clone();
        env.fund(&author, 1_000 * USDC);
        env
    }

    pub fn set_time(&mut self, unix_timestamp: i64) {
        let mut clock: Clock = self.svm.get_sysvar();
        clock.unix_timestamp = unix_timestamp;
        self.svm.set_sysvar(&clock);
        self.now = unix_timestamp;
    }

    pub fn advance(&mut self, seconds: i64) {
        self.set_time(self.now + seconds);
    }

    /// Airdrops SOL, creates the ATA and mints `tokens` to `owner`.
    pub fn fund(&mut self, owner: &Keypair, tokens: u64) {
        self.svm.airdrop(&owner.pubkey(), 10_000_000_000).unwrap();
        let ata = CreateAssociatedTokenAccount::new(&mut self.svm, &self.mint_authority, &self.mint)
            .owner(&owner.pubkey())
            .send()
            .unwrap();
        if tokens > 0 {
            MintTo::new(&mut self.svm, &self.mint_authority, &self.mint, &ata, tokens).send().unwrap();
        }
    }

    pub fn user(&mut self, tokens: u64) -> Keypair {
        let user = Keypair::new();

        self.fund(&user, tokens);
        user
    }

    pub fn ata(&self, owner: &Pubkey) -> Pubkey {
        get_associated_token_address(owner, &self.mint)
    }

    pub fn balance(&self, owner: &Pubkey) -> u64 {
        self.token_amount(&self.ata(owner))
    }

    pub fn token_amount(&self, token_account: &Pubkey) -> u64 {
        let account = self.svm.get_account(token_account).expect("token account exists");

        spl_token_interface::state::Account::unpack(&account.data).unwrap().amount
    }

    /// Sends a raw token transfer (e.g. an x402 payment landing in the vault).
    pub fn transfer(&mut self, from: &Keypair, to_token_account: &Pubkey, amount: u64) {
        let ix = spl_token_interface::instruction::transfer(
            &spl_token_interface::ID,
            &self.ata(&from.pubkey()),
            to_token_account,
            &from.pubkey(),
            &[],
            amount,
        )
        .unwrap();
        self.send(ix, &[from]).unwrap();
    }

    pub fn send(&mut self, ix: Instruction, signers: &[&Keypair]) -> TxResult {
        self.send_many(&[ix], signers)
    }

    pub fn send_many(&mut self, ixs: &[Instruction], signers: &[&Keypair]) -> TxResult {
        self.svm.expire_blockhash();
        let tx = Transaction::new_signed_with_payer(ixs, Some(&signers[0].pubkey()), signers, self.svm.latest_blockhash());

        self.svm.send_transaction(tx).map(|_| ())
    }

    fn read<T: AccountDeserialize>(&self, key: &Pubkey) -> T {
        let account = self.svm.get_account(key).expect("account exists");

        T::try_deserialize(&mut account.data.as_slice()).unwrap()
    }

    pub fn experiment(&self, key: &Pubkey) -> Experiment {
        self.read(key)
    }

    pub fn claim_account(&self, experiment: &Pubkey, worker: &Pubkey) -> Claim {
        self.read(&claim_pda(experiment, worker))
    }

    pub fn profile(&self, worker: &Pubkey) -> WorkerProfile {
        self.read(&profile_pda(worker))
    }

    pub fn vault(&self, experiment: &Pubkey) -> Pubkey {
        self.ata(experiment)
    }

    // ---------------------------------------------------------------- instructions

    pub fn create_args(&mut self, params: &Params) -> CreateExperimentArgs {
        self.nonce += 1;
        CreateExperimentArgs {
            nonce: self.nonce,
            spec_hash: [7; 32],
            image_digest: [9; 32],
            reward: params.reward,
            bond: params.bond,
            k: params.k,
            max_claims: params.max_claims,
            commit_deadline: self.now + params.commit_in,
            reveal_deadline: self.now + params.reveal_in,
            access_price: params.access_price,
            author_share_bps: params.author_share_bps,
        }
    }

    pub fn create_ix(&self, authority: &Pubkey, args: CreateExperimentArgs) -> (Pubkey, Instruction) {
        let experiment = experiment_pda(authority, args.nonce);
        let accounts = replicata::accounts::CreateExperiment {
            authority: *authority,
            experiment,
            mint: self.mint,
            vault: self.ata(&experiment),
            authority_token_account: self.ata(authority),
            token_program: spl_token_interface::ID,
            associated_token_program: spl_associated_token_account_interface::program::ID,
            system_program: anchor_lang::system_program::ID,
        };

        (experiment, ix(accounts, replicata::instruction::CreateExperiment { args }))
    }

    pub fn try_create(&mut self, params: &Params) -> Result<Pubkey, FailedTransactionMetadata> {
        let args = self.create_args(params);
        let (experiment, ix) = self.create_ix(&self.author.pubkey(), args);
        let author = self.author.insecure_clone();

        self.send(ix, &[&author]).map(|_| experiment)
    }

    pub fn create(&mut self, params: &Params) -> Pubkey {
        self.try_create(params).unwrap()
    }

    pub fn cancel(&mut self, experiment: &Pubkey) -> TxResult {
        let accounts = replicata::accounts::CancelExperiment {
            authority: self.author.pubkey(),
            experiment: *experiment,
            mint: self.mint,
            vault: self.vault(experiment),
            authority_token_account: self.ata(&self.author.pubkey()),
            token_program: spl_token_interface::ID,
        };
        let author = self.author.insecure_clone();

        self.send(ix(accounts, replicata::instruction::CancelExperiment {}), &[&author])
    }

    pub fn claim(&mut self, experiment: &Pubkey, worker: &Keypair) -> TxResult {
        let accounts = replicata::accounts::ClaimExperiment {
            worker: worker.pubkey(),
            experiment: *experiment,
            claim: claim_pda(experiment, &worker.pubkey()),
            worker_profile: profile_pda(&worker.pubkey()),
            mint: self.mint,
            vault: self.vault(experiment),
            worker_token_account: self.ata(&worker.pubkey()),
            token_program: spl_token_interface::ID,
            system_program: anchor_lang::system_program::ID,
        };

        self.send(ix(accounts, replicata::instruction::Claim {}), &[worker])
    }

    pub fn commit_raw(&mut self, experiment: &Pubkey, worker: &Keypair, commitment: [u8; 32]) -> TxResult {
        let accounts = replicata::accounts::CommitResult {
            worker: worker.pubkey(),
            experiment: *experiment,
            claim: claim_pda(experiment, &worker.pubkey()),
        };

        self.send(ix(accounts, replicata::instruction::Commit { commitment }), &[worker])
    }

    /// Commits `hash` with a salt derived from the worker key (tests reuse it in `reveal`).
    pub fn commit(&mut self, experiment: &Pubkey, worker: &Keypair, hash: [u8; 32]) -> TxResult {
        let commitment = replicata_core::commitment(&hash, &salt_of(worker), &worker.pubkey().to_bytes());

        self.commit_raw(experiment, worker, commitment)
    }

    pub fn reveal_raw(&mut self, experiment: &Pubkey, worker: &Keypair, output_hash: [u8; 32], salt: [u8; 32]) -> TxResult {
        let accounts = replicata::accounts::RevealResult {
            worker: worker.pubkey(),
            experiment: *experiment,
            claim: claim_pda(experiment, &worker.pubkey()),
        };

        self.send(ix(accounts, replicata::instruction::Reveal { output_hash, salt }), &[worker])
    }

    pub fn reveal(&mut self, experiment: &Pubkey, worker: &Keypair, hash: [u8; 32]) -> TxResult {
        self.reveal_raw(experiment, worker, hash, salt_of(worker))
    }

    /// `[claim, token account, profile]` for each worker, as `finalize` expects.
    pub fn finalize_remaining(&self, experiment: &Pubkey, workers: &[Pubkey]) -> Vec<AccountMeta> {
        workers
            .iter()
            .flat_map(|w| {
                [
                    AccountMeta::new(claim_pda(experiment, w), false),
                    AccountMeta::new(self.ata(w), false),
                    AccountMeta::new(profile_pda(w), false),
                ]
            })
            .collect()
    }

    pub fn finalize_ix(&self, experiment: &Pubkey, remaining: Vec<AccountMeta>, authority_token_account: Pubkey) -> Instruction {
        let accounts = replicata::accounts::Finalize {
            experiment: *experiment,
            mint: self.mint,
            vault: self.vault(experiment),
            authority_token_account,
            token_program: spl_token_interface::ID,
        };
        let mut instruction = ix(accounts, replicata::instruction::Finalize {});

        instruction.accounts.extend(remaining);
        instruction
    }

    pub fn finalize_with(&mut self, experiment: &Pubkey, remaining: Vec<AccountMeta>, payer: &Keypair) -> TxResult {
        let author_ata = self.ata(&self.experiment(experiment).authority);
        let instruction = self.finalize_ix(experiment, remaining, author_ata);

        self.send(instruction, &[payer])
    }

    /// Finalizes passing every revealed claim (the honest crank behaviour).
    pub fn finalize(&mut self, experiment: &Pubkey, revealed: &[Pubkey]) -> TxResult {
        let remaining = self.finalize_remaining(experiment, revealed);
        let cranker = self.user(0);

        self.finalize_with(experiment, remaining, &cranker)
    }

    pub fn claim_royalties_with(&mut self, experiment: &Pubkey, remaining: Vec<AccountMeta>) -> TxResult {
        let exp = self.experiment(experiment);
        let accounts = replicata::accounts::ClaimRoyalties {
            experiment: *experiment,
            mint: self.mint,
            vault: self.vault(experiment),
            authority_token_account: self.ata(&exp.authority),
            token_program: spl_token_interface::ID,
        };
        let mut instruction = ix(accounts, replicata::instruction::ClaimRoyalties {});
        instruction.accounts.extend(remaining);
        let cranker = self.user(0);

        self.send(instruction, &[&cranker])
    }

    pub fn claim_royalties(&mut self, experiment: &Pubkey) -> TxResult {
        let exp = self.experiment(experiment);
        let remaining = exp.replicators[..exp.agree_count as usize]
            .iter()
            .map(|w| AccountMeta::new(self.ata(w), false))
            .collect();

        self.claim_royalties_with(experiment, remaining)
    }

    pub fn withdraw_bond(&mut self, experiment: &Pubkey, worker: &Keypair) -> TxResult {
        let accounts = replicata::accounts::WithdrawBond {
            worker: worker.pubkey(),
            experiment: *experiment,
            claim: claim_pda(experiment, &worker.pubkey()),
            mint: self.mint,
            vault: self.vault(experiment),
            worker_token_account: self.ata(&worker.pubkey()),
            token_program: spl_token_interface::ID,
        };

        self.send(ix(accounts, replicata::instruction::WithdrawBond {}), &[worker])
    }

    /// Creates an experiment, funds `n` workers and has them all claim it.
    pub fn with_claims(&mut self, params: &Params, n: usize) -> (Pubkey, Vec<Keypair>) {
        let experiment = self.create(params);
        let workers: Vec<Keypair> = (0..n).map(|_| self.user(10 * USDC)).collect();

        workers.iter().for_each(|w| self.claim(&experiment, w).unwrap());
        (experiment, workers)
    }
}

fn ix(accounts: impl ToAccountMetas, data: impl InstructionData) -> Instruction {
    Instruction { program_id: replicata::ID, accounts: accounts.to_account_metas(None), data: data.data() }
}

pub fn salt_of(worker: &Keypair) -> [u8; 32] {
    let mut salt = worker.pubkey().to_bytes();

    salt.reverse();
    salt
}

pub fn experiment_pda(authority: &Pubkey, nonce: u64) -> Pubkey {
    Pubkey::find_program_address(&[replicata::EXPERIMENT_SEED, authority.as_ref(), &nonce.to_le_bytes()], &replicata::ID).0
}

pub fn claim_pda(experiment: &Pubkey, worker: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[replicata::CLAIM_SEED, experiment.as_ref(), worker.as_ref()], &replicata::ID).0
}

pub fn profile_pda(worker: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[replicata::WORKER_SEED, worker.as_ref()], &replicata::ID).0
}

pub fn pubkeys(workers: &[Keypair]) -> Vec<Pubkey> {
    workers.iter().map(|w| w.pubkey()).collect()
}

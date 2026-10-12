//! LiteSVM harness for the Replicata program.
//!
//! Anchor 1.2 and LiteSVM 0.18 share the same `solana-address`/`solana-instruction` types
//! (semver trick), so instructions built by `replicata-client` are sent to LiteSVM directly
//! — every test here also exercises the client used by the coordinator and the worker.

use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use litesvm::types::FailedTransactionMetadata;
use litesvm::LiteSVM;
use litesvm_token::{CreateAssociatedTokenAccount, CreateMint, MintTo};
use replicata_client as client;
use replicata_client::{Claim, CreateExperimentArgs, Experiment, WorkerProfile};
use solana_clock::Clock;
use solana_instruction_error::InstructionError;
use solana_keypair::Keypair;
use solana_program_pack::Pack;
use solana_signer::Signer;
use solana_transaction::Transaction;
use solana_transaction_error::TransactionError;

pub use replicata::error::ReplicataError;
pub use replicata_client::{claim_pda, experiment_pda, profile_pda};

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
        svm.add_program_from_file(client::PROGRAM_ID, PROGRAM_SO)
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
        client::ata(owner, &self.mint)
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
        self.svm.expire_blockhash();
        let tx = Transaction::new_signed_with_payer(&[ix], Some(&signers[0].pubkey()), signers, self.svm.latest_blockhash());

        self.svm.send_transaction(tx).map(|_| ())
    }

    fn read<T: anchor_lang::AccountDeserialize>(&self, key: &Pubkey) -> T {
        let account = self.svm.get_account(key).expect("account exists");

        client::decode(&account.data).expect("decodes")
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

    pub fn try_create(&mut self, params: &Params) -> Result<Pubkey, FailedTransactionMetadata> {
        let args = self.create_args(params);
        let (experiment, ix) = client::create_experiment(&self.author.pubkey(), &self.mint, args);
        let author = self.author.insecure_clone();

        self.send(ix, &[&author]).map(|_| experiment)
    }

    pub fn create(&mut self, params: &Params) -> Pubkey {
        self.try_create(params).unwrap()
    }

    pub fn cancel(&mut self, experiment: &Pubkey) -> TxResult {
        let mut ix = client::cancel_experiment(experiment, &self.experiment(experiment));
        // Let tests impersonate the author: sign and pay with `self.author`.
        ix.accounts[0].pubkey = self.author.pubkey();
        ix.accounts[4].pubkey = self.ata(&self.author.pubkey());
        let author = self.author.insecure_clone();

        self.send(ix, &[&author])
    }

    pub fn claim(&mut self, experiment: &Pubkey, worker: &Keypair) -> TxResult {
        let ix = client::claim(&worker.pubkey(), experiment, &self.experiment(experiment));

        self.send(ix, &[worker])
    }

    pub fn commit_raw(&mut self, experiment: &Pubkey, worker: &Keypair, commitment: [u8; 32]) -> TxResult {
        self.send(client::commit(&worker.pubkey(), experiment, commitment), &[worker])
    }

    /// Commits `hash` with a salt derived from the worker key (tests reuse it in `reveal`).
    pub fn commit(&mut self, experiment: &Pubkey, worker: &Keypair, hash: [u8; 32]) -> TxResult {
        let commitment = replicata_core::commitment(&hash, &salt_of(worker), &worker.pubkey().to_bytes());

        self.commit_raw(experiment, worker, commitment)
    }

    pub fn reveal_raw(&mut self, experiment: &Pubkey, worker: &Keypair, output_hash: [u8; 32], salt: [u8; 32]) -> TxResult {
        self.send(client::reveal(&worker.pubkey(), experiment, output_hash, salt), &[worker])
    }

    pub fn reveal(&mut self, experiment: &Pubkey, worker: &Keypair, hash: [u8; 32]) -> TxResult {
        self.reveal_raw(experiment, worker, hash, salt_of(worker))
    }

    /// `[claim, token account, profile]` for each worker, as `finalize` expects.
    pub fn finalize_remaining(&self, experiment: &Pubkey, workers: &[Pubkey]) -> Vec<AccountMeta> {
        client::finalize(experiment, &self.experiment(experiment), workers).accounts.split_off(5)
    }

    pub fn finalize_ix(&self, experiment: &Pubkey, remaining: Vec<AccountMeta>, authority_token_account: Pubkey) -> Instruction {
        let mut ix = client::finalize(experiment, &self.experiment(experiment), &[]);

        ix.accounts[3].pubkey = authority_token_account;
        ix.accounts.extend(remaining);
        ix
    }

    pub fn finalize_with(&mut self, experiment: &Pubkey, remaining: Vec<AccountMeta>, payer: &Keypair) -> TxResult {
        let author_ata = self.ata(&self.experiment(experiment).authority);
        let ix = self.finalize_ix(experiment, remaining, author_ata);

        self.send(ix, &[payer])
    }

    /// Finalizes passing every revealed claim (the honest crank behaviour).
    pub fn finalize(&mut self, experiment: &Pubkey, revealed: &[Pubkey]) -> TxResult {
        let ix = client::finalize(experiment, &self.experiment(experiment), revealed);
        let cranker = self.user(0);

        self.send(ix, &[&cranker])
    }

    pub fn claim_royalties_with(&mut self, experiment: &Pubkey, remaining: Vec<AccountMeta>) -> TxResult {
        let mut ix = client::claim_royalties(experiment, &self.experiment(experiment));
        ix.accounts.truncate(5);
        ix.accounts.extend(remaining);
        let cranker = self.user(0);

        self.send(ix, &[&cranker])
    }

    pub fn claim_royalties(&mut self, experiment: &Pubkey) -> TxResult {
        let ix = client::claim_royalties(experiment, &self.experiment(experiment));
        let cranker = self.user(0);

        self.send(ix, &[&cranker])
    }

    pub fn withdraw_bond(&mut self, experiment: &Pubkey, worker: &Keypair) -> TxResult {
        let ix = client::withdraw_bond(&worker.pubkey(), experiment, &self.experiment(experiment));

        self.send(ix, &[worker])
    }

    /// Creates an experiment, funds `n` workers and has them all claim it.
    pub fn with_claims(&mut self, params: &Params, n: usize) -> (Pubkey, Vec<Keypair>) {
        let experiment = self.create(params);
        let workers: Vec<Keypair> = (0..n).map(|_| self.user(10 * USDC)).collect();

        workers.iter().for_each(|w| self.claim(&experiment, w).unwrap());
        (experiment, workers)
    }
}

pub fn salt_of(worker: &Keypair) -> [u8; 32] {
    let mut salt = worker.pubkey().to_bytes();

    salt.reverse();
    salt
}

pub fn pubkeys(workers: &[Keypair]) -> Vec<Pubkey> {
    workers.iter().map(|w| w.pubkey()).collect()
}

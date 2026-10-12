//! claim_royalties: x402 access payments land in the vault and are split.

use anchor_lang::solana_program::instruction::AccountMeta;
use replicata_program_tests::*;
use solana_signer::Signer;

const A: [u8; 32] = [0xaa; 32];

fn replicated(
    env: &mut Env,
    params: &Params,
    hashes: &[[u8; 32]],
) -> (anchor_lang::prelude::Pubkey, Vec<solana_keypair::Keypair>) {
    let (experiment, workers) = env.with_claims(params, hashes.len());

    workers.iter().zip(hashes).for_each(|(w, h)| env.commit(&experiment, w, *h).unwrap());
    workers.iter().zip(hashes).for_each(|(w, h)| env.reveal(&experiment, w, *h).unwrap());
    env.finalize(&experiment, &pubkeys(&workers)).unwrap();
    (experiment, workers)
}

#[test]
fn splits_70_10_10_10() {
    let mut env = Env::new();
    let (experiment, workers) = replicated(&mut env, &Params::default(), &[A, A, A]);
    let author_before = env.balance(&env.author.pubkey());
    let consumer = env.user(5 * USDC);
    let vault = env.vault(&experiment);

    env.transfer(&consumer, &vault, USDC);
    env.claim_royalties(&experiment).unwrap();

    assert_eq!(env.balance(&env.author.pubkey()), author_before + 700_000);
    for w in &workers {
        assert_eq!(env.balance(&w.pubkey()), 11 * USDC + 100_000);
    }
    assert_eq!(env.token_amount(&vault), 0);
    assert_eq!(env.experiment(&experiment).royalties_paid, USDC);
    assert_program_err(env.claim_royalties(&experiment), ReplicataError::NoRoyalties);
}

#[test]
fn only_agreeing_workers_share_royalties() {
    let mut env = Env::new();
    let (experiment, workers) = replicated(&mut env, &Params::default(), &[A, [1; 32], A]);
    let dissenter_before = env.balance(&workers[1].pubkey());
    let consumer = env.user(5 * USDC);
    let vault = env.vault(&experiment);

    env.transfer(&consumer, &vault, 1_000_001);
    env.claim_royalties(&experiment).unwrap();

    // 30% of 1_000_001 = 300_000 (floored) split in 2; remainder to the author.
    assert_eq!(env.balance(&workers[1].pubkey()), dissenter_before);
    assert_eq!(env.balance(&workers[0].pubkey()), 12 * USDC + 150_000);
    assert_eq!(env.token_amount(&vault), 0);
    assert_eq!(env.experiment(&experiment).royalties_paid, 1_000_001);
}

#[test]
fn requires_replicated_status() {
    let mut env = Env::new();
    let experiment = env.create(&Params::default());

    assert_program_err(env.claim_royalties_with(&experiment, vec![]), ReplicataError::NotReplicated);
}

#[test]
fn rejects_wrong_or_missing_replicator_accounts() {
    let mut env = Env::new();
    let (experiment, workers) = replicated(&mut env, &Params::default(), &[A, A, A]);
    let consumer = env.user(5 * USDC);
    let vault = env.vault(&experiment);
    env.transfer(&consumer, &vault, USDC);

    let short: Vec<AccountMeta> = pubkeys(&workers[..2]).iter().map(|w| AccountMeta::new(env.ata(w), false)).collect();
    assert_program_err(env.claim_royalties_with(&experiment, short), ReplicataError::InvalidRemainingAccounts);

    let thief = env.user(0);
    let mut stolen: Vec<AccountMeta> = pubkeys(&workers).iter().map(|w| AccountMeta::new(env.ata(w), false)).collect();
    stolen[0] = AccountMeta::new(env.ata(&thief.pubkey()), false);
    assert_program_err(env.claim_royalties_with(&experiment, stolen), ReplicataError::InvalidRemainingAccounts);

    env.claim_royalties(&experiment).unwrap();
}

#[test]
fn author_share_extremes() {
    for (bps, author_gets, each_worker_gets) in [(0u16, 1u64, 333_333u64), (10_000, 1_000_000, 0)] {
        let mut env = Env::new();
        let params = Params { author_share_bps: bps, ..Default::default() };
        let (experiment, workers) = replicated(&mut env, &params, &[A, A, A]);
        let author_before = env.balance(&env.author.pubkey());
        let consumer = env.user(5 * USDC);
        let vault = env.vault(&experiment);

        env.transfer(&consumer, &vault, USDC);
        env.claim_royalties(&experiment).unwrap();
        assert_eq!(env.balance(&env.author.pubkey()), author_before + author_gets);
        assert_eq!(env.balance(&workers[2].pubkey()), 11 * USDC + each_worker_gets);
    }
}

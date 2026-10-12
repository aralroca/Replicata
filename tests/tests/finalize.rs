//! finalize / withdraw_bond.

use anchor_lang::solana_program::instruction::AccountMeta;
use replicata::state::{ClaimState, ExperimentStatus};
use replicata_program_tests::*;
use solana_keypair::Keypair;
use solana_signer::Signer;

const A: [u8; 32] = [0xaa; 32];
const B: [u8; 32] = [0xbb; 32];
const C: [u8; 32] = [0xcc; 32];

/// Claims, commits and reveals `hashes[i]` for worker `i` (all slots, early reveal).
fn revealed(env: &mut Env, params: &Params, hashes: &[[u8; 32]]) -> (anchor_lang::prelude::Pubkey, Vec<Keypair>) {
    let (experiment, workers) = env.with_claims(params, hashes.len());

    workers.iter().zip(hashes).for_each(|(w, h)| env.commit(&experiment, w, *h).unwrap());
    if hashes.len() < params.max_claims as usize {
        env.advance(params.commit_in + 1);
    }
    workers.iter().zip(hashes).for_each(|(w, h)| env.reveal(&experiment, w, *h).unwrap());
    (experiment, workers)
}

#[test]
fn three_of_three_pays_exactly() {
    let mut env = Env::new();
    let author_before = env.balance(&env.author.pubkey());
    let (experiment, workers) = revealed(&mut env, &Params::default(), &[A, A, A]);

    env.finalize(&experiment, &pubkeys(&workers)).unwrap();

    let exp = env.experiment(&experiment);
    assert_eq!(exp.status, ExperimentStatus::Replicated);
    assert_eq!((exp.result_hash, exp.agree_count, exp.escrowed), (A, 3, 0));
    assert_eq!(exp.replicators[..3].to_vec(), pubkeys(&workers));
    for w in &workers {
        assert_eq!(env.balance(&w.pubkey()), 11 * USDC, "bond back + 1 USDC reward");
        assert_eq!(env.claim_account(&experiment, &w.pubkey()).state, ClaimState::Paid);
        let profile = env.profile(&w.pubkey());
        assert_eq!((profile.ok, profile.bad, profile.earned), (1, 0, USDC));
    }
    assert_eq!(env.balance(&env.author.pubkey()), author_before - 3 * USDC);
    assert_eq!(env.token_amount(&env.vault(&experiment)), 0);
}

#[test]
fn two_of_three_confiscates_the_dissenting_bond() {
    let mut env = Env::new();
    let (experiment, workers) = revealed(&mut env, &Params::default(), &[A, B, A]);

    env.finalize(&experiment, &pubkeys(&workers)).unwrap();

    let exp = env.experiment(&experiment);
    assert_eq!((exp.status, exp.agree_count, exp.result_hash), (ExperimentStatus::Replicated, 2, A));
    // (3 reward + 1 confiscated bond) / 2 = 2 each, plus their own bond back.
    assert_eq!(env.balance(&workers[0].pubkey()), 9 * USDC + USDC + 2 * USDC);
    assert_eq!(env.balance(&workers[2].pubkey()), 12 * USDC);
    assert_eq!(env.balance(&workers[1].pubkey()), 9 * USDC);
    assert_eq!(env.claim_account(&experiment, &workers[1].pubkey()).state, ClaimState::Slashed);
    let cheat = env.profile(&workers[1].pubkey());
    assert_eq!((cheat.ok, cheat.bad, cheat.earned), (0, 1, 0));
    assert_eq!(env.token_amount(&env.vault(&experiment)), 0);
}

#[test]
fn indivisible_remainder_goes_to_the_author() {
    let mut env = Env::new();
    let params = Params { reward: 10, bond: 0, ..Default::default() };
    let author_before = env.balance(&env.author.pubkey());
    let (experiment, workers) = revealed(&mut env, &params, &[A, A, A]);

    env.finalize(&experiment, &pubkeys(&workers)).unwrap();
    assert!(workers.iter().all(|w| env.balance(&w.pubkey()) == 10 * USDC + 3));
    assert_eq!(env.balance(&env.author.pubkey()), author_before - 10 + 1);
    assert_eq!(env.token_amount(&env.vault(&experiment)), 0);
}

#[test]
fn no_majority_is_unresolved_and_refunds() {
    let mut env = Env::new();
    let author_before = env.balance(&env.author.pubkey());
    let (experiment, workers) = revealed(&mut env, &Params::default(), &[A, B, C]);

    env.finalize(&experiment, &pubkeys(&workers)).unwrap();

    let exp = env.experiment(&experiment);
    assert_eq!((exp.status, exp.escrowed), (ExperimentStatus::Unresolved, 0));
    assert_eq!(env.balance(&env.author.pubkey()), author_before);
    for w in &workers {
        assert_eq!(env.balance(&w.pubkey()), 10 * USDC);
        assert_eq!(env.claim_account(&experiment, &w.pubkey()).state, ClaimState::Refunded);
        assert_eq!(env.profile(&w.pubkey()).bad, 0, "a non-reproducible experiment is not the worker's fault");
    }
}

#[test]
fn a_tie_is_not_a_majority() {
    let mut env = Env::new();
    let params = Params { k: 4, max_claims: 4, ..Default::default() };
    let (experiment, workers) = revealed(&mut env, &params, &[A, A, B, B]);

    env.finalize(&experiment, &pubkeys(&workers)).unwrap();
    assert_eq!(env.experiment(&experiment).status, ExperimentStatus::Unresolved);
}

#[test]
fn withholding_a_reveal_is_not_a_free_veto() {
    let mut env = Env::new();
    let author_before = env.balance(&env.author.pubkey());
    let (experiment, workers) = env.with_claims(&Params::default(), 3);
    env.commit(&experiment, &workers[0], A).unwrap();
    env.commit(&experiment, &workers[1], A).unwrap();
    env.commit(&experiment, &workers[2], B).unwrap();
    env.reveal(&experiment, &workers[0], A).unwrap();
    env.reveal(&experiment, &workers[1], A).unwrap();

    assert_program_err(env.finalize(&experiment, &pubkeys(&workers[..2])), ReplicataError::NotReadyToFinalize);
    env.advance(201);
    env.finalize(&experiment, &pubkeys(&workers[..2])).unwrap();

    // 2 reveals < k = 3 → no quorum, but the withholder forfeits its bond to the revealers.
    let exp = env.experiment(&experiment);
    assert_eq!((exp.status, exp.escrowed), (ExperimentStatus::Unresolved, 0));
    assert_eq!(env.balance(&workers[0].pubkey()), 10 * USDC + USDC / 2);
    assert_eq!(env.balance(&workers[1].pubkey()), 10 * USDC + USDC / 2);
    assert_eq!(env.balance(&env.author.pubkey()), author_before, "reward refunded");
    assert_program_err(env.withdraw_bond(&experiment, &workers[2]), ReplicataError::BondForfeited);
    assert_eq!(env.balance(&workers[2].pubkey()), 9 * USDC);
    assert_eq!(env.token_amount(&env.vault(&experiment)), 0);
}

#[test]
fn silent_workers_lose_their_bond_when_the_rest_replicate() {
    let mut env = Env::new();
    let params = Params { k: 2, ..Default::default() };
    let (experiment, workers) = env.with_claims(&params, 3);
    env.commit(&experiment, &workers[0], A).unwrap();
    env.commit(&experiment, &workers[1], A).unwrap();
    env.advance(101);
    env.reveal(&experiment, &workers[0], A).unwrap();
    env.reveal(&experiment, &workers[1], A).unwrap();
    env.advance(100);

    env.finalize(&experiment, &pubkeys(&workers[..2])).unwrap();
    // (3 reward + 1 silent bond) / 2.
    assert_eq!(env.balance(&workers[0].pubkey()), 12 * USDC);
    assert_eq!(env.balance(&workers[1].pubkey()), 12 * USDC);
    assert_program_err(env.withdraw_bond(&experiment, &workers[2]), ReplicataError::BondNotWithdrawable);
}

#[test]
fn nobody_revealed_refunds_the_author_after_the_deadline() {
    let mut env = Env::new();
    let author_before = env.balance(&env.author.pubkey());
    let (experiment, workers) = env.with_claims(&Params::default(), 1);

    env.advance(201);
    env.finalize(&experiment, &[]).unwrap();
    assert_eq!(env.experiment(&experiment).status, ExperimentStatus::Unresolved);
    assert_eq!(env.balance(&env.author.pubkey()), author_before);
    env.withdraw_bond(&experiment, &workers[0]).unwrap();
    assert_eq!(env.balance(&workers[0].pubkey()), 10 * USDC);
}

#[test]
fn finalize_cannot_run_early_or_twice() {
    let mut env = Env::new();
    let (experiment, workers) = env.with_claims(&Params::default(), 3);
    workers.iter().for_each(|w| env.commit(&experiment, w, A).unwrap());
    env.reveal(&experiment, &workers[0], A).unwrap();

    assert_program_err(env.finalize(&experiment, &pubkeys(&workers[..1])), ReplicataError::NotReadyToFinalize);
    env.reveal(&experiment, &workers[1], A).unwrap();
    env.reveal(&experiment, &workers[2], A).unwrap();
    env.finalize(&experiment, &pubkeys(&workers)).unwrap();
    assert_program_err(env.finalize(&experiment, &pubkeys(&workers)), ReplicataError::ExperimentNotOpen);
}

#[test]
fn finalize_rejects_missing_duplicate_or_foreign_accounts() {
    let mut env = Env::new();
    let (experiment, workers) = revealed(&mut env, &Params::default(), &[A, A, B]);
    let (other, other_workers) = revealed(&mut env, &Params::default(), &[A, A, A]);
    let cranker = env.user(0);
    let keys = pubkeys(&workers);

    // Omitting the dissenter would hide a confiscation.
    let missing = env.finalize_remaining(&experiment, &keys[..2]);
    assert_program_err(env.finalize_with(&experiment, missing, &cranker), ReplicataError::InvalidRemainingAccounts);

    let duplicated = env.finalize_remaining(&experiment, &[keys[0], keys[0], keys[1]]);
    assert_program_err(env.finalize_with(&experiment, duplicated, &cranker), ReplicataError::InvalidRemainingAccounts);

    let mut foreign = env.finalize_remaining(&experiment, &keys[..2]);
    foreign.extend(env.finalize_remaining(&other, &pubkeys(&other_workers[..1])));
    assert_program_err(env.finalize_with(&experiment, foreign, &cranker), ReplicataError::InvalidRemainingAccounts);

    // Redirecting a payout to an attacker-owned token account.
    let thief = env.user(0);
    let mut stolen = env.finalize_remaining(&experiment, &keys);
    stolen[1] = AccountMeta::new(env.ata(&thief.pubkey()), false);
    assert_program_err(env.finalize_with(&experiment, stolen, &cranker), ReplicataError::InvalidRemainingAccounts);

    // Passing someone else's profile.
    let mut wrong_profile = env.finalize_remaining(&experiment, &keys);
    wrong_profile.swap(2, 5);
    assert_program_err(
        env.finalize_with(&experiment, wrong_profile, &cranker),
        ReplicataError::InvalidRemainingAccounts,
    );

    let honest = env.finalize_remaining(&experiment, &keys);
    env.finalize_with(&experiment, honest, &cranker).unwrap();
}

#[test]
fn finalize_requires_the_authors_token_account() {
    let mut env = Env::new();
    let (experiment, workers) = revealed(&mut env, &Params::default(), &[A, A, A]);
    let thief = env.user(0);
    let cranker = env.user(0);
    let remaining = env.finalize_remaining(&experiment, &pubkeys(&workers));
    let thief_ata = env.ata(&thief.pubkey());

    let ix = env.finalize_ix(&experiment, remaining.clone(), thief_ata);
    assert!(env.send(ix, &[&cranker]).is_err());
    env.finalize_with(&experiment, remaining, &cranker).unwrap();
}

#[test]
fn withdraw_bond_only_when_unresolved() {
    let mut env = Env::new();
    let (experiment, workers) = env.with_claims(&Params::default(), 1);

    assert_program_err(env.withdraw_bond(&experiment, &workers[0]), ReplicataError::BondNotWithdrawable);
}

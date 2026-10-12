//! create / cancel / claim / commit / reveal.

use replicata::state::{ClaimState, ExperimentStatus};
use replicata_program_tests::*;
use solana_signer::Signer;

#[test]
fn create_escrows_the_reward() {
    let mut env = Env::new();
    let before = env.balance(&env.author.pubkey());
    let experiment = env.create(&Params::default());

    let exp = env.experiment(&experiment);
    assert_eq!(exp.status, ExperimentStatus::Open);
    assert_eq!(exp.authority, env.author.pubkey());
    assert_eq!(exp.escrowed, 3 * USDC);
    assert_eq!((exp.k, exp.max_claims, exp.author_share_bps), (3, 3, 7_000));
    assert_eq!(exp.spec_hash, [7; 32]);
    assert_eq!(env.token_amount(&env.vault(&experiment)), 3 * USDC);
    assert_eq!(env.balance(&env.author.pubkey()), before - 3 * USDC);
}

#[test]
fn create_rejects_invalid_params() {
    let mut env = Env::new();
    let cases = [
        (Params { reward: 0, ..Default::default() }, ReplicataError::ZeroReward),
        (Params { k: 0, ..Default::default() }, ReplicataError::InvalidReplicationParams),
        (Params { k: 4, max_claims: 3, ..Default::default() }, ReplicataError::InvalidReplicationParams),
        (Params { k: 3, max_claims: 6, ..Default::default() }, ReplicataError::InvalidReplicationParams),
        (Params { author_share_bps: 10_001, ..Default::default() }, ReplicataError::InvalidAuthorShare),
        (Params { commit_in: 0, ..Default::default() }, ReplicataError::InvalidDeadlines),
        (Params { commit_in: 100, reveal_in: 100, ..Default::default() }, ReplicataError::InvalidDeadlines),
    ];

    for (params, err) in cases {
        let result = env.try_create(&params).map(|_| ());
        assert_program_err(result, err);
    }
}

#[test]
fn create_fails_without_funds() {
    let mut env = Env::new();
    let result = env.try_create(&Params { reward: 10_000 * USDC, ..Default::default() }).map(|_| ());

    assert!(result.is_err());
}

#[test]
fn cancel_refunds_when_nobody_claimed() {
    let mut env = Env::new();
    let before = env.balance(&env.author.pubkey());
    let experiment = env.create(&Params::default());

    env.cancel(&experiment).unwrap();
    assert_eq!(env.experiment(&experiment).status, ExperimentStatus::Cancelled);
    assert_eq!(env.balance(&env.author.pubkey()), before);
    assert_eq!(env.token_amount(&env.vault(&experiment)), 0);
    assert_program_err(env.cancel(&experiment), ReplicataError::ExperimentNotOpen);
}

#[test]
fn cancel_fails_with_active_claims() {
    let mut env = Env::new();
    let (experiment, _) = env.with_claims(&Params::default(), 1);

    assert_program_err(env.cancel(&experiment), ReplicataError::HasClaims);
}

#[test]
fn cancel_requires_the_author() {
    let mut env = Env::new();
    let experiment = env.create(&Params::default());
    env.author = env.user(0);

    assert!(env.cancel(&experiment).is_err());
}

#[test]
fn claim_locks_the_bond_and_creates_the_profile() {
    let mut env = Env::new();
    let (experiment, workers) = env.with_claims(&Params::default(), 1);
    let worker = workers[0].pubkey();

    assert_eq!(env.balance(&worker), 9 * USDC);
    assert_eq!(env.token_amount(&env.vault(&experiment)), 4 * USDC);
    let exp = env.experiment(&experiment);
    assert_eq!((exp.claims_count, exp.escrowed), (1, 4 * USDC));
    let claim = env.claim_account(&experiment, &worker);
    assert_eq!((claim.state, claim.worker, claim.experiment), (ClaimState::Claimed, worker, experiment));
    assert_eq!(env.profile(&worker).claims, 1);
}

#[test]
fn double_claim_fails() {
    let mut env = Env::new();
    let (experiment, workers) = env.with_claims(&Params::default(), 1);

    assert!(env.claim(&experiment, &workers[0]).is_err());
}

#[test]
fn claims_beyond_max_fail() {
    let mut env = Env::new();
    let (experiment, _) = env.with_claims(&Params::default(), 3);
    let late = env.user(10 * USDC);

    assert_program_err(env.claim(&experiment, &late), ReplicataError::ClaimsFull);
}

#[test]
fn claim_after_commit_deadline_fails() {
    let mut env = Env::new();
    let experiment = env.create(&Params::default());
    let worker = env.user(10 * USDC);

    env.advance(101);
    assert_program_err(env.claim(&experiment, &worker), ReplicataError::CommitWindowClosed);
}

#[test]
fn claim_without_bond_funds_fails() {
    let mut env = Env::new();
    let experiment = env.create(&Params::default());
    let broke = env.user(0);

    assert!(env.claim(&experiment, &broke).is_err());
}

#[test]
fn profile_is_reused_across_experiments() {
    let mut env = Env::new();
    let a = env.create(&Params::default());
    let b = env.create(&Params::default());
    let worker = env.user(10 * USDC);

    env.claim(&a, &worker).unwrap();
    env.claim(&b, &worker).unwrap();
    assert_eq!(env.profile(&worker.pubkey()).claims, 2);
}

#[test]
fn commit_once_and_only_before_deadline() {
    let mut env = Env::new();
    let (experiment, workers) = env.with_claims(&Params::default(), 2);

    env.commit(&experiment, &workers[0], [1; 32]).unwrap();
    assert_eq!(env.claim_account(&experiment, &workers[0].pubkey()).state, ClaimState::Committed);
    assert_eq!(env.experiment(&experiment).commits_count, 1);
    assert_program_err(env.commit(&experiment, &workers[0], [1; 32]), ReplicataError::InvalidClaimState);

    env.advance(101);
    assert_program_err(env.commit(&experiment, &workers[1], [1; 32]), ReplicataError::CommitWindowClosed);
}

#[test]
fn commit_requires_own_claim() {
    let mut env = Env::new();
    let (experiment, _) = env.with_claims(&Params::default(), 1);
    let stranger = env.user(0);

    assert!(env.commit(&experiment, &stranger, [1; 32]).is_err());
}

#[test]
fn reveal_checks_the_commitment() {
    let mut env = Env::new();
    let (experiment, workers) = env.with_claims(&Params::default(), 3);
    workers.iter().for_each(|w| env.commit(&experiment, w, [1; 32]).unwrap());

    assert_program_err(
        env.reveal_raw(&experiment, &workers[0], [1; 32], [0; 32]),
        ReplicataError::CommitmentMismatch,
    );
    assert_program_err(env.reveal(&experiment, &workers[0], [2; 32]), ReplicataError::CommitmentMismatch);
    env.reveal(&experiment, &workers[0], [1; 32]).unwrap();

    let claim = env.claim_account(&experiment, &workers[0].pubkey());
    assert_eq!((claim.state, claim.revealed_hash, claim.salt), (ClaimState::Revealed, [1; 32], salt_of(&workers[0])));
    assert_eq!(env.experiment(&experiment).reveals_count, 1);
    assert_program_err(env.reveal(&experiment, &workers[0], [1; 32]), ReplicataError::InvalidClaimState);
}

#[test]
fn copying_another_workers_commitment_does_not_let_you_reveal_their_result() {
    let mut env = Env::new();
    let (experiment, workers) = env.with_claims(&Params::default(), 3);
    let honest = &workers[0];
    let copycat = &workers[1];
    let honest_commitment = env.claim_account(&experiment, &honest.pubkey()).commitment;

    env.commit(&experiment, honest, [5; 32]).unwrap();
    let copied = env.claim_account(&experiment, &honest.pubkey()).commitment;
    assert_ne!(copied, honest_commitment);
    env.commit_raw(&experiment, copycat, copied).unwrap();
    env.commit(&experiment, &workers[2], [5; 32]).unwrap();

    assert_program_err(
        env.reveal_raw(&experiment, copycat, [5; 32], salt_of(honest)),
        ReplicataError::CommitmentMismatch,
    );
}

#[test]
fn reveal_window() {
    let mut env = Env::new();
    let (experiment, workers) = env.with_claims(&Params { max_claims: 4, ..Default::default() }, 3);
    workers.iter().for_each(|w| env.commit(&experiment, w, [1; 32]).unwrap());

    // A free slot remains, so reveals wait for the commit deadline.
    assert_program_err(env.reveal(&experiment, &workers[0], [1; 32]), ReplicataError::RevealWindowClosed);
    env.advance(101);
    env.reveal(&experiment, &workers[0], [1; 32]).unwrap();
    env.advance(100);
    assert_program_err(env.reveal(&experiment, &workers[1], [1; 32]), ReplicataError::RevealWindowClosed);
}

#[test]
fn reveal_opens_early_when_every_slot_is_committed() {
    let mut env = Env::new();
    let (experiment, workers) = env.with_claims(&Params::default(), 3);

    env.commit(&experiment, &workers[0], [1; 32]).unwrap();
    env.commit(&experiment, &workers[1], [1; 32]).unwrap();
    assert_program_err(env.reveal(&experiment, &workers[0], [1; 32]), ReplicataError::RevealWindowClosed);
    env.commit(&experiment, &workers[2], [1; 32]).unwrap();
    env.reveal(&experiment, &workers[0], [1; 32]).unwrap();
}

#[test]
fn reveal_requires_commit() {
    let mut env = Env::new();
    let (experiment, workers) = env.with_claims(&Params::default(), 3);

    env.advance(101);
    assert_program_err(env.reveal(&experiment, &workers[0], [1; 32]), ReplicataError::InvalidClaimState);
}

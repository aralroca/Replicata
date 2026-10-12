//! Property test over random full lifecycles: funds are conserved, the vault ends empty,
//! and the outcome matches an independent model of the rules.

use proptest::prelude::*;
use replicata::state::ExperimentStatus;
use replicata_program_tests::*;
use solana_keypair::Keypair;
use solana_signer::Signer;

#[derive(Debug, Clone, Copy)]
enum Action {
    Silent,
    CommitOnly,
    Reveal(u8),
}

#[derive(Debug, Clone)]
struct Scenario {
    k: u8,
    max_claims: u8,
    reward: u64,
    bond: u64,
    author_share_bps: u16,
    actions: Vec<Action>,
    royalties: u64,
}

fn action() -> impl Strategy<Value = Action> {
    prop_oneof![
        1 => Just(Action::Silent),
        1 => Just(Action::CommitOnly),
        6 => (0u8..3).prop_map(Action::Reveal),
    ]
}

fn scenario() -> impl Strategy<Value = Scenario> {
    (1u8..=5)
        .prop_flat_map(|k| (Just(k), k..=5))
        .prop_flat_map(|(k, max_claims)| {
            (
                Just(k),
                Just(max_claims),
                1u64..=10 * USDC,
                0u64..=5 * USDC,
                0u16..=10_000,
                proptest::collection::vec(action(), 0..=max_claims as usize),
                0u64..=3 * USDC,
            )
        })
        .prop_map(|(k, max_claims, reward, bond, author_share_bps, actions, royalties)| Scenario {
            k,
            max_claims,
            reward,
            bond,
            author_share_bps,
            actions,
            royalties,
        })
}

/// Expected `(replicated, majority hash index)` according to the written rules.
fn model(s: &Scenario) -> (bool, Option<u8>) {
    let reveals: Vec<u8> =
        s.actions.iter().filter_map(|a| if let Action::Reveal(h) = a { Some(*h) } else { None }).collect();
    let best = (0u8..3).map(|h| (h, reveals.iter().filter(|r| **r == h).count())).max_by_key(|(_, c)| *c);
    let quorum = best.is_some_and(|(_, c)| reveals.len() >= s.k as usize && c * 2 > reveals.len());

    (quorum, best.filter(|_| quorum).map(|(h, _)| h))
}

fn hash(h: u8) -> [u8; 32] {
    [h + 1; 32]
}

fn total(env: &Env, owners: &[anchor_lang::prelude::Pubkey], vault: &anchor_lang::prelude::Pubkey) -> u64 {
    owners.iter().map(|o| env.balance(o)).sum::<u64>() + env.token_amount(vault)
}

fn run(s: Scenario) -> Result<(), TestCaseError> {
    let mut env = Env::new();
    let params = Params {
        reward: s.reward,
        bond: s.bond,
        k: s.k,
        max_claims: s.max_claims,
        author_share_bps: s.author_share_bps,
        ..Default::default()
    };
    let (experiment, workers) = env.with_claims(&params, s.actions.len());
    let consumer = env.user(5 * USDC);
    let vault = env.vault(&experiment);
    let mut owners = pubkeys(&workers);
    owners.extend([env.author.pubkey(), consumer.pubkey()]);
    let initial_total = total(&env, &owners, &vault);
    // Balances after the bond is locked.
    let before: Vec<u64> = workers.iter().map(|w| env.balance(&w.pubkey())).collect();

    for (w, a) in workers.iter().zip(&s.actions) {
        if let Action::CommitOnly | Action::Reveal(_) = a {
            let h = if let Action::Reveal(h) = a { hash(*h) } else { hash(9) };
            env.commit(&experiment, w, h).unwrap();
        }
    }
    env.advance(101);
    let revealers: Vec<&Keypair> = workers
        .iter()
        .zip(&s.actions)
        .filter_map(|(w, a)| {
            if let Action::Reveal(h) = a {
                env.reveal(&experiment, w, hash(*h)).unwrap();
                Some(w)
            } else {
                None
            }
        })
        .collect();
    env.advance(100);
    let revealed_keys: Vec<_> = revealers.iter().map(|w| w.pubkey()).collect();
    env.finalize(&experiment, &revealed_keys).unwrap();

    let exp = env.experiment(&experiment);
    let (replicated, majority) = model(&s);
    prop_assert_eq!(exp.status == ExperimentStatus::Replicated, replicated);
    prop_assert!(env.token_amount(&vault) >= exp.escrowed);

    if replicated {
        let agree = s.actions.iter().filter(|a| matches!(a, Action::Reveal(h) if Some(*h) == majority)).count() as u64;
        prop_assert_eq!(exp.agree_count as u64, agree);
        prop_assert_eq!(exp.result_hash, hash(majority.unwrap()));
        for ((w, a), b) in workers.iter().zip(&s.actions).zip(&before) {
            let now = env.balance(&w.pubkey());
            match a {
                Action::Reveal(h) if Some(*h) == majority => prop_assert!(now >= *b + s.bond),
                _ => prop_assert_eq!(now, *b, "dissenting/silent bond confiscated"),
            }
        }
        if s.royalties > 0 {
            env.transfer(&consumer, &vault, s.royalties);
            env.claim_royalties(&experiment).unwrap();
        }
    } else {
        for (w, a) in workers.iter().zip(&s.actions) {
            if !matches!(a, Action::Reveal(_)) {
                env.withdraw_bond(&experiment, w).unwrap();
            }
        }
        for (w, b) in workers.iter().zip(&before) {
            prop_assert_eq!(env.balance(&w.pubkey()), *b + s.bond, "every bond refunded");
        }
    }

    prop_assert_eq!(env.token_amount(&vault), 0, "vault fully settled");
    prop_assert_eq!(env.experiment(&experiment).escrowed, 0);
    prop_assert_eq!(total(&env, &owners, &vault), initial_total, "no token created or destroyed");
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 96, ..ProptestConfig::default() })]

    #[test]
    fn lifecycle_conserves_funds(s in scenario()) {
        run(s)?;
    }
}

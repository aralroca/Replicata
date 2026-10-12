//! Permissionless housekeeping, done by the coordinator so nobody has to: `finalize`
//! experiments whose windows are over and distribute pending royalties.

use std::time::Duration;

use replicata_client::{self as client, ClaimState, ExperimentStatus, Phase, Pubkey};

use crate::snapshot::{now, Snapshot};
use crate::state::AppState;

#[derive(Debug, PartialEq, Eq)]
pub enum Job {
    Finalize { experiment: Pubkey, revealed: Vec<Pubkey> },
    Royalties { experiment: Pubkey },
}

/// Work to do for the current snapshot.
pub fn jobs(snapshot: &Snapshot, now: i64) -> Vec<Job> {
    snapshot
        .experiments
        .iter()
        .filter_map(|(id, exp)| {
            if client::phase(exp, now) == Phase::Finalizable {
                let revealed = snapshot
                    .claims_of(id)
                    .iter()
                    .filter(|c| c.state == ClaimState::Revealed)
                    .map(|c| c.worker)
                    .collect();
                return Some(Job::Finalize { experiment: *id, revealed });
            }
            (exp.status == ExperimentStatus::Replicated && snapshot.royalties_pending(exp) > 0)
                .then_some(Job::Royalties { experiment: *id })
        })
        .collect()
}

async fn run_job(state: &AppState, snapshot: &Snapshot, job: &Job) -> anyhow::Result<String> {
    match job {
        Job::Finalize { experiment, revealed } => {
            let exp = &snapshot.experiments[experiment];
            state.send(&[client::finalize(experiment, exp, revealed)]).await
        }
        Job::Royalties { experiment } => {
            let exp = &snapshot.experiments[experiment];
            state.send(&[client::claim_royalties(experiment, exp)]).await
        }
    }
}

pub async fn run(state: AppState, every: Duration) {
    loop {
        tokio::time::sleep(every).await;
        let snapshot = state.snapshot.read().await.clone();
        for job in jobs(&snapshot, now()) {
            match run_job(&state, &snapshot, &job).await {
                Ok(signature) => tracing::info!(?job, %signature, "crank"),
                Err(err) => tracing::warn!(?job, "crank failed: {err:#}"),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use replicata_client::{Claim, Experiment, MAX_CLAIMS};

    use super::*;

    fn experiment(status: ExperimentStatus, reveal_deadline: i64, vault: Pubkey) -> Experiment {
        Experiment {
            authority: Pubkey::new_unique(),
            mint: Pubkey::new_unique(),
            vault,
            nonce: 0,
            spec_hash: [0; 32],
            image_digest: [0; 32],
            reward: 1,
            bond: 1,
            k: 1,
            max_claims: 2,
            claims_count: 2,
            commits_count: 2,
            reveals_count: 1,
            commit_deadline: reveal_deadline - 10,
            reveal_deadline,
            status,
            result_hash: [0; 32],
            agree_count: 1,
            replicators: [Pubkey::default(); MAX_CLAIMS as usize],
            access_price: 1,
            author_share_bps: 0,
            escrowed: 0,
            royalties_paid: 0,
            created_at: 0,
            bump: 0,
        }
    }

    fn claim(experiment: Pubkey, state: ClaimState) -> Claim {
        Claim {
            experiment,
            worker: Pubkey::new_unique(),
            commitment: [0; 32],
            revealed_hash: [0; 32],
            salt: [0; 32],
            state,
            claimed_at: 0,
            bump: 0,
        }
    }

    #[test]
    fn finalizes_expired_experiments_with_every_revealed_claim() {
        let (id, waiting) = (Pubkey::new_unique(), Pubkey::new_unique());
        let mut snapshot = Snapshot::default();
        snapshot.experiments.insert(id, experiment(ExperimentStatus::Open, 100, Pubkey::new_unique()));
        snapshot.experiments.insert(waiting, experiment(ExperimentStatus::Open, 1_000, Pubkey::new_unique()));
        let revealed = claim(id, ClaimState::Revealed);
        snapshot.claims.insert(id, vec![revealed.clone(), claim(id, ClaimState::Committed)]);

        assert_eq!(jobs(&snapshot, 101), vec![Job::Finalize { experiment: id, revealed: vec![revealed.worker] }]);
    }

    #[test]
    fn distributes_royalties_only_when_the_vault_holds_extra_tokens() {
        let (paid, idle, vault) = (Pubkey::new_unique(), Pubkey::new_unique(), Pubkey::new_unique());
        let mut snapshot = Snapshot::default();
        snapshot.experiments.insert(paid, experiment(ExperimentStatus::Replicated, 0, vault));
        snapshot.experiments.insert(idle, experiment(ExperimentStatus::Replicated, 0, Pubkey::new_unique()));
        snapshot.vault_balances.insert(vault, 10_000);

        assert_eq!(jobs(&snapshot, 1_000), vec![Job::Royalties { experiment: paid }]);
    }
}

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
                let revealed = snapshot.claims_of(id).iter().filter(|c| c.state == ClaimState::Revealed).map(|c| c.worker).collect();
                return Some(Job::Finalize { experiment: *id, revealed });
            }
            (exp.status == ExperimentStatus::Replicated && snapshot.royalties_pending(exp) > 0).then_some(Job::Royalties { experiment: *id })
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

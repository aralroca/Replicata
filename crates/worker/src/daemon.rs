//! `replicata run` without `--once`: every few seconds, resume unfinished jobs, claim new open
//! experiments and recover refundable bonds. Jobs run concurrently (a job waiting for its
//! reveal window never blocks claiming others).

use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::job::{log, Worker};
use crate::state::{Stage, State};

type InFlight = Arc<Mutex<HashSet<String>>>;

fn spawn(worker: &Arc<Worker>, in_flight: &InFlight, id: String) {
    if !in_flight.lock().unwrap().insert(id.clone()) {
        return;
    }
    let (worker, in_flight) = (worker.clone(), in_flight.clone());
    tokio::spawn(async move {
        if let Err(err) = worker.process(&id).await {
            log(&id, format!("{err:#}"));
        }
        in_flight.lock().unwrap().remove(&id);
    });
}

/// Saved jobs that still need work, plus open experiments with a free slot.
async fn todo(worker: &Worker, state: &State) -> Vec<String> {
    let me = worker.chain.pubkey().to_string();
    let pending = state.jobs.iter().filter(|(_, j)| j.stage.in_progress()).map(|(id, _)| id.clone());
    let open = worker.api.experiments("status=open&phase=claiming").await.unwrap_or_default();
    let fresh = open
        .into_iter()
        .filter(|e| e.claims_count < e.max_claims && e.claim_of(&me).is_none() && !state.jobs.contains_key(&e.id))
        .map(|e| e.id);

    pending.chain(fresh).collect()
}

/// Unrevealed jobs of experiments that ended `unresolved` with no reveal at all.
/// One coordinator call per tick, intersected with the local jobs.
async fn refundable(worker: &Worker, state: &State) -> Vec<String> {
    let unresolved = worker.api.experiments("status=unresolved&limit=200").await.unwrap_or_default();

    unresolved
        .into_iter()
        .filter(|e| e.reveals_count == 0)
        .filter(|e| state.jobs.get(&e.id).is_some_and(|j| j.stage.in_progress() || j.stage == Stage::Failed))
        .map(|e| e.id)
        .collect()
}

pub async fn run_forever(worker: Arc<Worker>, every: Duration) {
    let in_flight: InFlight = Arc::default();
    loop {
        let state = State::load(&worker.state_path()).unwrap_or_default();
        for id in refundable(&worker, &state).await {
            if let Err(err) = worker.withdraw(&id).await {
                log(&id, format!("withdraw failed: {err:#}"));
            }
        }
        todo(&worker, &state).await.into_iter().for_each(|id| spawn(&worker, &in_flight, id));
        tokio::time::sleep(every).await;
    }
}

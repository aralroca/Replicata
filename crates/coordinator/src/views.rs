//! JSON shapes returned by the API (documented in `docs/API.md` and `/openapi.json`).

use std::collections::{HashMap, HashSet};

use replicata_client::{self as client, Claim, ClaimState, Experiment, ExperimentStatus, Phase, Pubkey, WorkerProfile};
use replicata_core::{to_hex, JobSpec};
use serde::Serialize;
use utoipa::ToSchema;

use crate::snapshot::Snapshot;

#[derive(Serialize, ToSchema, Debug, Clone)]
pub struct ClaimView {
    pub worker: String,
    pub state: &'static str,
    pub commitment: String,
    pub revealed_hash: Option<String>,
    pub claimed_at: i64,
    pub has_bundle: bool,
}

#[derive(Serialize, ToSchema, Debug, Clone)]
pub struct ExperimentView {
    pub id: String,
    pub authority: String,
    pub mint: String,
    pub vault: String,
    pub nonce: u64,
    pub spec_hash: String,
    pub image_digest: String,
    pub reward: u64,
    pub bond: u64,
    pub k: u8,
    pub max_claims: u8,
    pub claims_count: u8,
    pub commits_count: u8,
    pub reveals_count: u8,
    pub commit_deadline: i64,
    pub reveal_deadline: i64,
    pub created_at: i64,
    pub status: &'static str,
    pub phase: &'static str,
    pub result_hash: Option<String>,
    pub agree_count: u8,
    pub replicators: Vec<String>,
    pub access_price: u64,
    pub author_share_bps: u16,
    pub escrowed: u64,
    pub royalties_paid: u64,
    pub royalties_pending: u64,
    #[schema(value_type = Object)]
    pub spec: Option<JobSpec>,
    pub claims: Vec<ClaimView>,
}

#[derive(Serialize, ToSchema, Debug)]
pub struct WorkerView {
    pub worker: String,
    pub claims: u32,
    pub ok: u32,
    pub bad: u32,
    pub earned: u64,
    pub history: Vec<HistoryEntry>,
}

#[derive(Serialize, ToSchema, Debug)]
pub struct HistoryEntry {
    pub experiment: String,
    pub state: &'static str,
}

#[derive(Serialize, ToSchema, Debug)]
pub struct AuthorView {
    pub author: String,
    pub experiments: Vec<ExperimentView>,
    pub royalties_paid: u64,
    pub royalties_pending: u64,
}

pub fn status_name(status: ExperimentStatus) -> &'static str {
    match status {
        ExperimentStatus::Open => "open",
        ExperimentStatus::Replicated => "replicated",
        ExperimentStatus::Unresolved => "unresolved",
        ExperimentStatus::Cancelled => "cancelled",
    }
}

pub fn phase_name(phase: Phase) -> &'static str {
    match phase {
        Phase::Claiming => "claiming",
        Phase::Revealing => "revealing",
        Phase::Finalizable => "finalizable",
        Phase::Settled => "settled",
    }
}

pub fn claim_state_name(state: ClaimState) -> &'static str {
    match state {
        ClaimState::Claimed => "claimed",
        ClaimState::Committed => "committed",
        ClaimState::Revealed => "revealed",
        ClaimState::Paid => "paid",
        ClaimState::Slashed => "slashed",
        ClaimState::Refunded => "refunded",
    }
}

fn revealed(claim: &Claim) -> bool {
    matches!(claim.state, ClaimState::Revealed | ClaimState::Paid | ClaimState::Slashed)
        || (claim.state == ClaimState::Refunded && claim.revealed_hash != [0; 32])
}

/// Extra off-chain data joined into the views.
#[derive(Default)]
pub struct Joined {
    pub specs: HashMap<String, JobSpec>,
    pub bundles: HashSet<(String, String)>,
}

pub fn claim_view(experiment: &Pubkey, claim: &Claim, joined: &Joined) -> ClaimView {
    ClaimView {
        worker: claim.worker.to_string(),
        state: claim_state_name(claim.state),
        commitment: to_hex(&claim.commitment),
        revealed_hash: revealed(claim).then(|| to_hex(&claim.revealed_hash)),
        claimed_at: claim.claimed_at,
        has_bundle: joined.bundles.contains(&(experiment.to_string(), claim.worker.to_string())),
    }
}

pub fn experiment_view(
    id: &Pubkey,
    exp: &Experiment,
    snapshot: &Snapshot,
    joined: &Joined,
    now: i64,
) -> ExperimentView {
    let spec_hash = to_hex(&exp.spec_hash);

    ExperimentView {
        id: id.to_string(),
        authority: exp.authority.to_string(),
        mint: exp.mint.to_string(),
        vault: exp.vault.to_string(),
        nonce: exp.nonce,
        spec: joined.specs.get(&spec_hash).cloned(),
        spec_hash,
        image_digest: to_hex(&exp.image_digest),
        reward: exp.reward,
        bond: exp.bond,
        k: exp.k,
        max_claims: exp.max_claims,
        claims_count: exp.claims_count,
        commits_count: exp.commits_count,
        reveals_count: exp.reveals_count,
        commit_deadline: exp.commit_deadline,
        reveal_deadline: exp.reveal_deadline,
        created_at: exp.created_at,
        status: status_name(exp.status),
        phase: phase_name(client::phase(exp, now)),
        result_hash: (exp.status == ExperimentStatus::Replicated).then(|| to_hex(&exp.result_hash)),
        agree_count: exp.agree_count,
        replicators: client::replicators(exp).iter().map(Pubkey::to_string).collect(),
        access_price: exp.access_price,
        author_share_bps: exp.author_share_bps,
        escrowed: exp.escrowed,
        royalties_paid: exp.royalties_paid,
        royalties_pending: snapshot.royalties_pending(exp),
        claims: snapshot.claims_of(id).iter().map(|c| claim_view(id, c, joined)).collect(),
    }
}

pub fn worker_view(worker: &Pubkey, profile: Option<&WorkerProfile>, snapshot: &Snapshot) -> WorkerView {
    let history = snapshot
        .claims
        .iter()
        .flat_map(|(exp, claims)| claims.iter().filter(|c| c.worker == *worker).map(move |c| (exp, c)))
        .map(|(exp, c)| HistoryEntry { experiment: exp.to_string(), state: claim_state_name(c.state) })
        .collect();

    WorkerView {
        worker: worker.to_string(),
        claims: profile.map_or(0, |p| p.claims),
        ok: profile.map_or(0, |p| p.ok),
        bad: profile.map_or(0, |p| p.bad),
        earned: profile.map_or(0, |p| p.earned),
        history,
    }
}

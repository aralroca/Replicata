//! Free, read-mostly endpoints.

use std::collections::{HashMap, HashSet};
use std::str::FromStr;

use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use replicata_client::{Experiment, Pubkey};
use replicata_core::{net, to_hex, JobSpec};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::db;
use crate::error::{ApiError, ApiResult};
use crate::snapshot::{now, Snapshot};
use crate::state::AppState;
use crate::views::{self, experiment_view, AuthorView, ExperimentView, Joined, WorkerView};

pub fn parse_pubkey(value: &str) -> ApiResult<Pubkey> {
    Pubkey::from_str(value).map_err(|_| ApiError::bad_request(format!("invalid public key: {value}")))
}

pub async fn config(State(state): State<AppState>) -> Json<Value> {
    let cfg = &state.cfg;

    Json(json!({
        "cluster": cfg.cluster,
        "network": cfg.network,
        "rpc_url": cfg.rpc_url,
        "program_id": cfg.program_id().to_string(),
        "mint": cfg.mint.to_string(),
        "decimals": cfg.decimals,
        "treasury": cfg.treasury.to_string(),
        "publish_fee": cfg.publish_fee,
        "facilitator_url": cfg.facilitator_url,
        "faucets": { "sol": net::SOL_FAUCET_URL, "usdc": net::USDC_FAUCET_URL },
    }))
}

pub async fn health() -> &'static str {
    "ok"
}

pub async fn post_spec(State(state): State<AppState>, body: Bytes) -> ApiResult<(StatusCode, Json<Value>)> {
    let spec = JobSpec::parse(&body).map_err(|e| ApiError::bad_request(e.to_string()))?;
    let spec_hash = to_hex(&spec.spec_hash());
    let inserted = db::insert_spec(&state.db, &spec_hash, &spec).await?;
    let status = if inserted { StatusCode::CREATED } else { StatusCode::OK };

    Ok((status, Json(json!({ "spec_hash": spec_hash }))))
}

pub async fn get_spec(State(state): State<AppState>, Path(hash): Path<String>) -> ApiResult<Json<JobSpec>> {
    let specs = db::specs(&state.db, &[hash]).await?;

    specs.into_iter().next().map(|(_, spec)| Json(spec)).ok_or_else(|| ApiError::not_found("spec"))
}

/// Loads the specs and bundle markers needed to render `experiments`.
pub async fn joined(state: &AppState, experiments: &[(Pubkey, String)]) -> ApiResult<Joined> {
    let ids: Vec<String> = experiments.iter().map(|(id, _)| id.to_string()).collect();
    let hashes: Vec<String> = experiments.iter().map(|(_, h)| h.clone()).collect();
    let specs: HashMap<String, JobSpec> = db::specs(&state.db, &hashes).await?.into_iter().collect();
    let bundles: HashSet<(String, String)> = db::bundle_keys(&state.db, &ids).await?.into_iter().collect();

    Ok(Joined { specs, bundles })
}

/// Renders the given experiments, joining their specs and bundle markers in two queries.
pub async fn render(
    state: &AppState,
    snapshot: &Snapshot,
    items: &[(Pubkey, Experiment)],
) -> ApiResult<Vec<ExperimentView>> {
    let keyed: Vec<(Pubkey, String)> = items.iter().map(|(id, e)| (*id, to_hex(&e.spec_hash))).collect();
    let joined = joined(state, &keyed).await?;
    let now = now();

    Ok(items.iter().map(|(id, exp)| experiment_view(id, exp, snapshot, &joined, now)).collect())
}

#[derive(Deserialize, Default)]
pub struct ListQuery {
    status: Option<String>,
    phase: Option<String>,
    author: Option<String>,
    limit: Option<usize>,
}

fn matches(exp: &Experiment, q: &ListQuery, now: i64) -> bool {
    q.status.as_deref().is_none_or(|s| s == views::status_name(exp.status))
        && q.phase.as_deref().is_none_or(|p| p == views::phase_name(replicata_client::phase(exp, now)))
        && q.author.as_deref().is_none_or(|a| a == exp.authority.to_string())
}

/// Filters, sorts (newest first) and limits on the raw accounts; only the page is rendered.
pub async fn list_experiments(
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> ApiResult<Json<Vec<ExperimentView>>> {
    let snapshot = state.snapshot.read().await.clone();
    let now = now();
    let mut items: Vec<(Pubkey, Experiment)> =
        snapshot.experiments.iter().filter(|(_, e)| matches(e, &q, now)).map(|(id, e)| (*id, e.clone())).collect();
    items.sort_by_key(|(id, e)| std::cmp::Reverse((e.created_at, *id)));
    items.truncate(q.limit.unwrap_or(50).min(200));

    Ok(Json(render(&state, &snapshot, &items).await?))
}

pub async fn get_experiment(State(state): State<AppState>, Path(id): Path<String>) -> ApiResult<Json<ExperimentView>> {
    let id = parse_pubkey(&id)?;
    let snapshot = state.snapshot.read().await.clone();
    // Status, claims and vault balance must come from the same moment: a fresh account rendered
    // with older claims showed a just-settled experiment with unsettled claims and its payout as
    // "royalties pending". Only an experiment the indexer has not seen yet is read from the chain.
    let experiment = match snapshot.experiments.get(&id) {
        Some(indexed) => indexed.clone(),
        None => state.experiment(&id).await.ok_or_else(|| ApiError::not_found("experiment"))?,
    };
    let views = render(&state, &snapshot, &[(id, experiment)]).await?;

    views.into_iter().next().map(Json).ok_or_else(|| ApiError::not_found("experiment"))
}

pub async fn get_worker(State(state): State<AppState>, Path(worker): Path<String>) -> ApiResult<Json<WorkerView>> {
    let worker = parse_pubkey(&worker)?;
    let snapshot = state.snapshot.read().await;

    Ok(Json(views::worker_view(&worker, snapshot.profiles.get(&worker), &snapshot)))
}

pub async fn get_author(State(state): State<AppState>, Path(author): Path<String>) -> ApiResult<Json<AuthorView>> {
    let author = parse_pubkey(&author)?;
    let snapshot = state.snapshot.read().await.clone();
    let items: Vec<(Pubkey, Experiment)> =
        snapshot.experiments.iter().filter(|(_, e)| e.authority == author).map(|(id, e)| (*id, e.clone())).collect();
    let experiments = render(&state, &snapshot, &items).await?;

    Ok(Json(AuthorView {
        author: author.to_string(),
        royalties_paid: experiments.iter().map(|e| e.royalties_paid).sum(),
        royalties_pending: experiments.iter().map(|e| e.royalties_pending).sum(),
        experiments,
    }))
}

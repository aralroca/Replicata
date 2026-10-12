//! Free, read-mostly endpoints.

use std::collections::{HashMap, HashSet};
use std::str::FromStr;

use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use replicata_client::Pubkey;
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

pub async fn render(state: &AppState, snapshot: &Snapshot, ids: Vec<Pubkey>) -> ApiResult<Vec<ExperimentView>> {
    let keyed: Vec<(Pubkey, String)> =
        ids.iter().filter_map(|id| snapshot.experiments.get(id).map(|e| (*id, to_hex(&e.spec_hash)))).collect();
    let joined = joined(state, &keyed).await?;
    let now = now();

    Ok(keyed.iter().map(|(id, _)| experiment_view(id, &snapshot.experiments[id], snapshot, &joined, now)).collect())
}

#[derive(Deserialize, Default)]
pub struct ListQuery {
    status: Option<String>,
    phase: Option<String>,
    author: Option<String>,
    limit: Option<usize>,
}

fn matches(view: &ExperimentView, q: &ListQuery) -> bool {
    q.status.as_deref().is_none_or(|s| s == view.status)
        && q.phase.as_deref().is_none_or(|p| p == view.phase)
        && q.author.as_deref().is_none_or(|a| a == view.authority)
}

pub async fn list_experiments(State(state): State<AppState>, Query(q): Query<ListQuery>) -> ApiResult<Json<Vec<ExperimentView>>> {
    let snapshot = state.snapshot.read().await.clone();
    let mut ids: Vec<Pubkey> = snapshot.experiments.keys().copied().collect();
    ids.sort_by_key(|id| std::cmp::Reverse((snapshot.experiments[id].created_at, *id)));
    let views = render(&state, &snapshot, ids).await?;
    let limit = q.limit.unwrap_or(50).min(200);

    Ok(Json(views.into_iter().filter(|v| matches(v, &q)).take(limit).collect()))
}

pub async fn get_experiment(State(state): State<AppState>, Path(id): Path<String>) -> ApiResult<Json<ExperimentView>> {
    let id = parse_pubkey(&id)?;
    let snapshot = state.snapshot.read().await.clone();
    let views = render(&state, &snapshot, vec![id]).await?;

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
    let ids: Vec<Pubkey> = snapshot.experiments.iter().filter(|(_, e)| e.authority == author).map(|(id, _)| *id).collect();
    let experiments = render(&state, &snapshot, ids).await?;

    Ok(Json(AuthorView {
        author: author.to_string(),
        royalties_paid: experiments.iter().map(|e| e.royalties_paid).sum(),
        royalties_pending: experiments.iter().map(|e| e.royalties_pending).sum(),
        experiments,
    }))
}

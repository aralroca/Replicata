//! x402-paid endpoints. Guards run **before** the paywall so that nothing is charged for a
//! resource that cannot be delivered.

use axum::body::{to_bytes, Body};
use axum::extract::{Path, Request, State};
use axum::http::{header, HeaderMap, HeaderValue};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::{Extension, Json};
use base64::Engine;
use replicata_client::{self as client, CreateExperimentArgs, ExperimentStatus, Pubkey};
use replicata_core::{bundle, to_hex, JobSpec};
use serde::Deserialize;
use serde_json::{json, Value};
use solana_transaction::Transaction;
use x402_types::proto::SettleResponse;

use crate::db;
use crate::error::{ApiError, ApiResult};
use crate::payments;
use crate::routes::public::parse_pubkey;
use crate::snapshot::now;
use crate::state::AppState;

/// The verified bundle of a replicated experiment, loaded by [`experiment_guard`].
#[derive(Clone)]
pub struct Verified {
    pub experiment: Pubkey,
    pub result_hash: String,
    pub price: u64,
    pub bundle: Vec<u8>,
}

/// Experiment id from `/v1/experiments/{id}/...`.
pub fn experiment_id(path: &str) -> Option<&str> {
    path.strip_prefix("/v1/experiments/")?.split('/').next()
}

async fn load_verified(state: &AppState, path: &str) -> ApiResult<Verified> {
    let id = parse_pubkey(experiment_id(path).unwrap_or_default())?;
    let exp = state.snapshot.read().await.experiments.get(&id).cloned().ok_or_else(|| ApiError::not_found("experiment"))?;
    if exp.status != ExperimentStatus::Replicated {
        return Err(ApiError::conflict("not_replicated", "the experiment has not been replicated yet"));
    }
    let result_hash = to_hex(&exp.result_hash);
    let bundle = db::bundle_with_hash(&state.db, &id.to_string(), &result_hash)
        .await?
        .ok_or_else(|| ApiError::conflict("bundle_missing", "no replicator uploaded the verified bundle yet"))?;

    Ok(Verified { experiment: id, result_hash, price: exp.access_price, bundle })
}

fn dataset_file(verified: &Verified, file: &str) -> ApiResult<Vec<u8>> {
    let files = bundle::unpack(&verified.bundle).map_err(anyhow::Error::from)?;

    files.into_iter().find(|(name, _)| name == file).map(|(_, bytes)| bytes).ok_or_else(|| ApiError::not_found("dataset file"))
}

/// Pre-paywall guard for `result` and `dataset`: 404/409 without charging anything.
pub async fn experiment_guard(State(state): State<AppState>, mut req: Request, next: Next) -> Response {
    let path = req.uri().path().to_string();
    let verified = match load_verified(&state, &path).await {
        Ok(verified) => verified,
        Err(err) => return err.into_response(),
    };
    if let Some(file) = path.split("/dataset/").nth(1) {
        if let Err(err) = dataset_file(&verified, file) {
            return err.into_response();
        }
    }
    req.extensions_mut().insert(verified);
    next.run(req).await
}

async fn record_or_conflict(state: &AppState, headers: &HeaderMap, settle: &Option<SettleResponse>, resource: &str, amount: u64, pay_to: &Pubkey) -> ApiResult<()> {
    if payments::record(state, headers, settle, resource, amount, pay_to).await? {
        return Ok(());
    }
    Err(ApiError::conflict("payment_replayed", "this payment was already used"))
}

pub async fn result(
    State(state): State<AppState>,
    Extension(verified): Extension<Verified>,
    Extension(settle): Extension<Option<SettleResponse>>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    let resource = format!("/v1/experiments/{}/result", verified.experiment);
    record_or_conflict(&state, &headers, &settle, &resource, verified.price, &verified.experiment).await?;
    let disposition = format!("attachment; filename=\"{}.tar.gz\"", verified.experiment);

    Ok((
        [
            (header::CONTENT_TYPE, HeaderValue::from_static("application/gzip")),
            (header::CONTENT_DISPOSITION, HeaderValue::from_str(&disposition).map_err(anyhow::Error::from)?),
            (header::HeaderName::from_static("x-result-hash"), HeaderValue::from_str(&verified.result_hash).map_err(anyhow::Error::from)?),
        ],
        verified.bundle,
    )
        .into_response())
}

pub async fn dataset(
    State(state): State<AppState>,
    Path((_, file)): Path<(String, String)>,
    Extension(verified): Extension<Verified>,
    Extension(settle): Extension<Option<SettleResponse>>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    let resource = format!("/v1/experiments/{}/dataset/{file}", verified.experiment);
    record_or_conflict(&state, &headers, &settle, &resource, verified.price, &verified.experiment).await?;

    Ok(([(header::CONTENT_TYPE, "application/octet-stream")], dataset_file(&verified, &file)?).into_response())
}

/// `POST /v1/experiments` body (agents publishing without a UI).
#[derive(Deserialize, Clone, Debug)]
pub struct PublishRequest {
    pub authority: String,
    pub spec: Value,
    pub reward: u64,
    pub bond: u64,
    pub k: u8,
    pub max_claims: u8,
    pub commit_secs: i64,
    pub reveal_secs: i64,
    pub access_price: u64,
    pub author_share_bps: u16,
}

#[derive(Clone)]
pub struct ValidPublish {
    pub authority: Pubkey,
    pub spec: JobSpec,
    pub args: CreateExperimentArgs,
}

fn validate_publish(req: PublishRequest) -> ApiResult<ValidPublish> {
    let authority = parse_pubkey(&req.authority)?;
    let spec = JobSpec::parse(req.spec.to_string().as_bytes()).map_err(|e| ApiError::bad_request(e.to_string()))?;
    let image_digest = spec.image_digest_bytes().ok_or_else(|| ApiError::bad_request("invalid image digest"))?;
    let params_ok = req.reward > 0
        && (1..=req.max_claims).contains(&req.k)
        && req.max_claims <= client::MAX_CLAIMS
        && req.author_share_bps <= 10_000
        && req.access_price > 0
        && req.commit_secs > 0
        && req.reveal_secs > 0;
    if !params_ok {
        return Err(ApiError::bad_request("need reward > 0, 1 <= k <= max_claims <= 5, author_share_bps <= 10000, access_price > 0 and positive windows"));
    }
    let now = now();
    let args = CreateExperimentArgs {
        nonce: now as u64 * 1_000 + u64::from(rand_suffix()),
        spec_hash: spec.spec_hash(),
        image_digest,
        reward: req.reward,
        bond: req.bond,
        k: req.k,
        max_claims: req.max_claims,
        commit_deadline: now + req.commit_secs,
        reveal_deadline: now + req.commit_secs + req.reveal_secs,
        access_price: req.access_price,
        author_share_bps: req.author_share_bps,
    };

    Ok(ValidPublish { authority, spec, args })
}

/// Sub-second entropy for the nonce so two publishes in the same second do not collide.
fn rand_suffix() -> u16 {
    (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().subsec_millis() % 1000) as u16
}

/// Pre-paywall guard for `POST /v1/experiments`: invalid bodies are rejected for free.
pub async fn publish_guard(req: Request, next: Next) -> Response {
    let (parts, body) = req.into_parts();
    let Ok(bytes) = to_bytes(body, 1024 * 1024).await else {
        return ApiError::bad_request("body too large").into_response();
    };
    let parsed = serde_json::from_slice::<PublishRequest>(&bytes)
        .map_err(|e| ApiError::bad_request(format!("invalid JSON: {e}")))
        .and_then(validate_publish);
    match parsed {
        Ok(valid) => {
            let mut req = Request::from_parts(parts, Body::from(bytes));
            req.extensions_mut().insert(valid);
            next.run(req).await
        }
        Err(err) => err.into_response(),
    }
}

pub async fn publish(
    State(state): State<AppState>,
    Extension(valid): Extension<ValidPublish>,
    Extension(settle): Extension<Option<SettleResponse>>,
    headers: HeaderMap,
) -> ApiResult<Json<Value>> {
    let cfg = &state.cfg;
    record_or_conflict(&state, &headers, &settle, "/v1/experiments", cfg.publish_fee, &cfg.treasury).await?;
    let spec_hash = to_hex(&valid.spec.spec_hash());
    db::insert_spec(&state.db, &spec_hash, &valid.spec).await?;
    let (experiment, ix) = client::create_experiment(&valid.authority, &cfg.mint, valid.args);
    let mut tx = Transaction::new_with_payer(&[ix], Some(&valid.authority));
    tx.message.recent_blockhash = state.rpc.get_latest_blockhash().await.map_err(anyhow::Error::from)?;
    let bytes = bincode::serialize(&tx).map_err(anyhow::Error::from)?;

    Ok(Json(json!({
        "experiment": experiment.to_string(),
        "spec_hash": spec_hash,
        "transaction": base64::engine::general_purpose::STANDARD.encode(bytes),
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_experiment_id() {
        assert_eq!(experiment_id("/v1/experiments/abc/result"), Some("abc"));
        assert_eq!(experiment_id("/v1/experiments/abc/dataset/x/y.csv"), Some("abc"));
        assert_eq!(experiment_id("/v1/other"), None);
    }

    fn request() -> PublishRequest {
        PublishRequest {
            authority: Pubkey::new_unique().to_string(),
            spec: json!({
                "version": 1, "title": "t", "description": "", "image": "alpine",
                "image_digest": format!("sha256:{}", "ab".repeat(32)), "cmd": ["true"], "env": {},
                "inputs": [], "cpus": 1, "mem_mb": 128, "timeout_s": 10, "seed": 1
            }),
            reward: 1,
            bond: 0,
            k: 1,
            max_claims: 1,
            commit_secs: 60,
            reveal_secs: 60,
            access_price: 1,
            author_share_bps: 7000,
        }
    }

    #[test]
    fn validates_publish_requests() {
        assert!(validate_publish(request()).is_ok());
        assert!(validate_publish(PublishRequest { k: 0, ..request() }).is_err());
        assert!(validate_publish(PublishRequest { max_claims: 6, k: 6, ..request() }).is_err());
        assert!(validate_publish(PublishRequest { reward: 0, ..request() }).is_err());
        assert!(validate_publish(PublishRequest { access_price: 0, ..request() }).is_err());
        assert!(validate_publish(PublishRequest { spec: json!({"version": 1}), ..request() }).is_err());
        assert!(validate_publish(PublishRequest { authority: "nope".into(), ..request() }).is_err());
    }

    #[test]
    fn deadlines_follow_the_windows() {
        let valid = validate_publish(request()).unwrap();

        assert_eq!(valid.args.reveal_deadline - valid.args.commit_deadline, 60);
        assert_eq!(valid.args.spec_hash, valid.spec.spec_hash());
    }
}

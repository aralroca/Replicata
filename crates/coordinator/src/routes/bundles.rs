//! `POST /v1/experiments/:id/bundles`: a committed worker uploads its result bundle.

use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use replicata_client::{self as client, Claim, ClaimState, Pubkey};
use replicata_core::{bundle, bundle_upload_message, from_hex, to_hex};
use serde_json::{json, Value};
use solana_signature::Signature;

use crate::db;
use crate::error::{ApiError, ApiResult};
use crate::routes::public::parse_pubkey;
use crate::state::AppState;

fn header<'a>(headers: &'a HeaderMap, name: &str) -> ApiResult<&'a str> {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| ApiError::bad_request(format!("missing header {name}")))
}

/// Checks the ed25519 signature of `"replicata:bundle:v1:" ‖ experiment ‖ H`.
fn verify_signature(experiment: &[u8; 32], worker: &[u8; 32], hash: &[u8; 32], signature: &str) -> ApiResult<()> {
    let signature: Signature = signature
        .parse()
        .map_err(|_| ApiError::new(StatusCode::UNAUTHORIZED, "bad_signature", "malformed signature"))?;

    if !signature.verify(worker, &bundle_upload_message(experiment, hash)) {
        return Err(ApiError::new(StatusCode::UNAUTHORIZED, "bad_signature", "signature does not match worker"));
    }
    Ok(())
}

/// Read straight from the chain: the worker uploads right after its commit lands, before the
/// indexer's next poll would see it.
async fn claim_state(state: &AppState, experiment: &Pubkey, worker: &Pubkey) -> Option<ClaimState> {
    let account = state.rpc.get_account(&client::claim_pda(experiment, worker)).await.ok()?;

    client::decode::<Claim>(&account.data).map(|claim| claim.state)
}

pub async fn upload(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> ApiResult<(StatusCode, Json<Value>)> {
    let experiment = parse_pubkey(&id)?;
    let worker = parse_pubkey(header(&headers, "x-worker")?)?;
    let declared = from_hex(header(&headers, "x-output-hash")?)
        .ok_or_else(|| ApiError::bad_request("X-Output-Hash must be 64 hex chars"))?;
    verify_signature(&experiment.to_bytes(), &worker.to_bytes(), &declared, header(&headers, "x-signature")?)?;

    match claim_state(&state, &experiment, &worker).await {
        None => return Err(ApiError::not_found("claim")),
        Some(ClaimState::Committed | ClaimState::Revealed) => {}
        Some(_) => {
            return Err(ApiError::conflict("claim_state", "bundles are accepted only for committed or revealed claims"))
        }
    }

    let actual = bundle::hash_bundle(&body).map_err(|e| ApiError::bad_request(format!("invalid bundle: {e}")))?;
    if actual != declared {
        return Err(ApiError::bad_request(format!("bundle hash {} does not match X-Output-Hash", to_hex(&actual))));
    }
    db::upsert_bundle(&state.db, &experiment.to_string(), &worker.to_string(), &to_hex(&actual), &body).await?;
    Ok((StatusCode::CREATED, Json(json!({ "output_hash": to_hex(&actual) }))))
}

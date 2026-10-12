//! x402 plumbing around `x402-axum`: price tags, replay protection, payment records and
//! the mapping of payment failures to clear, actionable responses.

use std::str::FromStr;

use axum::body::{to_bytes, Body};
use axum::extract::{Request, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::Json;
use base64::Engine;
use replicata_client::Pubkey;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use x402_chain_solana::chain::{SolanaChainReference, SolanaTokenDeployment};
use x402_chain_solana::V2SolanaExact;
use x402_types::proto::v2;
use x402_types::proto::SettleResponse;

use crate::config::Config;
use crate::db;
use crate::state::AppState;

pub const PAYMENT_HEADER: &str = "payment-signature";
const MAX_ERROR_BODY: usize = 64 * 1024;

pub fn token(cfg: &Config) -> SolanaTokenDeployment {
    let reference = SolanaChainReference::from_str(cfg.network.trim_start_matches("solana:")).expect("valid CAIP-2 network");

    SolanaTokenDeployment::new(reference, cfg.mint.into(), cfg.decimals)
}

pub fn price_tag(cfg: &Config, pay_to: Pubkey, amount: u64) -> v2::PriceTag {
    V2SolanaExact::price_tag(pay_to, token(cfg).amount(amount))
}

/// Stable id of a payment: sha256 of the `PAYMENT-SIGNATURE` header value.
pub fn payment_hash(headers: &HeaderMap) -> Option<String> {
    headers.get(PAYMENT_HEADER).map(|v| hex::encode(Sha256::digest(v.as_bytes())))
}

/// Rejects a payment that was already settled and recorded (409) before touching the facilitator.
pub async fn replay_guard(State(state): State<AppState>, req: Request, next: Next) -> Response {
    let Some(hash) = payment_hash(req.headers()) else {
        return next.run(req).await;
    };

    match db::payment_exists(&state.db, &hash).await {
        Ok(true) => (StatusCode::CONFLICT, Json(json!({ "error": "payment_replayed", "message": "this payment was already used" }))).into_response(),
        Ok(false) => next.run(req).await,
        Err(err) => crate::error::ApiError::from(err).into_response(),
    }
}

/// Persists a settled payment. Returns `false` on a concurrent replay.
pub async fn record(state: &AppState, headers: &HeaderMap, settle: &Option<SettleResponse>, resource: &str, amount: u64, pay_to: &Pubkey) -> anyhow::Result<bool> {
    let hash = payment_hash(headers).unwrap_or_default();
    let settle = settle.as_ref().map(|s| &s.0);
    let field = |name: &str| settle.and_then(|s| s.get(name)).and_then(Value::as_str);
    let payment = db::Payment {
        payment_hash: &hash,
        resource,
        payer: field("payer"),
        amount: amount as i64,
        pay_to: &pay_to.to_string(),
        tx_signature: field("transaction"),
    };

    Ok(db::record_payment(&state.db, &payment).await?)
}

/// What went wrong with a payment, judged from the facilitator's error text.
#[derive(Debug, PartialEq, Eq)]
pub enum Failure {
    InsufficientFunds,
    FacilitatorUnavailable,
    SettlementFailed,
    PaymentRequired,
}

pub fn classify(error: &str, settlement: bool) -> Failure {
    let lower = error.to_lowercase();
    if lower.contains("insufficient") {
        return Failure::InsufficientFunds;
    }
    let unreachable = ["error sending request", "connection refused", "timed out", "dns error", "connect error", "503", "502", "504"];
    if unreachable.iter().any(|needle| lower.contains(needle)) {
        return Failure::FacilitatorUnavailable;
    }
    if settlement {
        return Failure::SettlementFailed;
    }
    Failure::PaymentRequired
}

fn payment_required_error(headers: &HeaderMap) -> Option<String> {
    let raw = headers.get("payment-required")?.to_str().ok()?;
    let decoded = base64::engine::general_purpose::STANDARD.decode(raw).ok()?;
    let value: Value = serde_json::from_slice(&decoded).ok()?;

    value.get("error").and_then(Value::as_str).map(str::to_string)
}

fn failure_body(failure: &Failure, message: &str, faucet: &str) -> Value {
    match failure {
        Failure::InsufficientFunds => json!({ "error": "insufficient_funds", "message": message, "faucet": faucet }),
        Failure::FacilitatorUnavailable => json!({ "error": "facilitator_unavailable", "message": message }),
        Failure::SettlementFailed => json!({ "error": "settlement_failed", "message": message }),
        Failure::PaymentRequired => json!({ "error": "payment_required", "message": message }),
    }
}

/// Rewrites `x402-axum`'s 402 responses into the documented error contract.
pub async fn map_failures(State(state): State<AppState>, req: Request, next: Next) -> Response {
    let response = next.run(req).await;
    if response.status() != StatusCode::PAYMENT_REQUIRED {
        return response;
    }
    let (mut parts, body) = response.into_parts();
    let bytes = to_bytes(body, MAX_ERROR_BODY).await.unwrap_or_default();
    let settlement_details = serde_json::from_slice::<Value>(&bytes)
        .ok()
        .and_then(|v| v.get("details").and_then(Value::as_str).map(str::to_string));
    let (message, settlement) = match settlement_details {
        Some(details) => (details, true),
        None => (payment_required_error(&parts.headers).unwrap_or_else(|| "payment required".into()), false),
    };
    let failure = classify(&message, settlement);
    let faucet = if state.cfg.cluster == "localnet" { "scripts/localnet.sh fund <pubkey>" } else { replicata_core::net::USDC_FAUCET_URL };

    if failure == Failure::FacilitatorUnavailable {
        parts.status = StatusCode::SERVICE_UNAVAILABLE;
        parts.headers.insert("retry-after", HeaderValue::from_static("5"));
    }
    parts.headers.insert("content-type", HeaderValue::from_static("application/json"));
    parts.headers.remove("content-length");
    Response::from_parts(parts, Body::from(failure_body(&failure, &message, faucet).to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_failures() {
        assert_eq!(classify("Insufficient funds for transfer", true), Failure::InsufficientFunds);
        assert_eq!(classify("error sending request for url (http://x/settle)", true), Failure::FacilitatorUnavailable);
        assert_eq!(classify("Payment-Signature header is required", false), Failure::PaymentRequired);
        assert_eq!(classify("transaction simulation failed", true), Failure::SettlementFailed);
    }

    #[test]
    fn payment_hash_is_stable() {
        let mut headers = HeaderMap::new();

        assert_eq!(payment_hash(&headers), None);
        headers.insert(PAYMENT_HEADER, HeaderValue::from_static("abc"));
        assert_eq!(payment_hash(&headers), Some(hex::encode(Sha256::digest(b"abc"))));
    }
}

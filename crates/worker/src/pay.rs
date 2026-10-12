//! x402 payments from the CLI (`x402-reqwest` + the worker keypair): paid inputs,
//! `replicata buy` (agents consuming verified results) and `replicata publish`.

use std::sync::Arc;

use anyhow::Context;
use base64::Engine;
use reqwest_middleware::ClientWithMiddleware;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_keypair::Keypair;
use solana_transaction::Transaction;
use x402_chain_solana::V2SolanaExactClient;
use x402_reqwest::{ReqwestWithPayments, ReqwestWithPaymentsBuild, X402Client};

use crate::chain::Chain;

pub fn paying_client(keypair: Arc<Keypair>, rpc: Arc<RpcClient>) -> ClientWithMiddleware {
    let x402 = X402Client::new().register(V2SolanaExactClient::new(keypair, rpc));

    reqwest::Client::new().with_payments(x402).build()
}

/// Turns a non-2xx response into a readable error (the coordinator returns JSON errors).
pub async fn check(response: reqwest::Response) -> anyhow::Result<reqwest::Response> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }
    let retry = response.headers().get("retry-after").and_then(|v| v.to_str().ok()).map(str::to_string);
    let body = response.text().await.unwrap_or_default();
    let hint = retry.map(|r| format!(" (retry after {r}s)")).unwrap_or_default();

    anyhow::bail!("{status}{hint}: {body}")
}

pub struct Bought {
    pub bundle: Vec<u8>,
    pub result_hash: String,
    pub settlement: Option<String>,
}

/// Pays for and downloads a verified result, re-checking its canonical hash locally.
pub async fn buy(http: &ClientWithMiddleware, coordinator: &str, id: &str) -> anyhow::Result<Bought> {
    let response = check(http.get(format!("{coordinator}/v1/experiments/{id}/result")).send().await?).await?;
    let header = |name: &str| response.headers().get(name).and_then(|v| v.to_str().ok()).map(str::to_string);
    let result_hash = header("x-result-hash").context("missing X-Result-Hash")?;
    let settlement = header("payment-response");
    let bundle = response.bytes().await?.to_vec();
    let actual = replicata_core::to_hex(&replicata_core::bundle::hash_bundle(&bundle)?);
    anyhow::ensure!(actual == result_hash, "bundle hash {actual} does not match the on-chain result {result_hash}");

    Ok(Bought { bundle, result_hash, settlement })
}

/// Agent flow: pay the publish fee over x402, sign the returned transaction and send it.
pub async fn publish(http: &ClientWithMiddleware, chain: &Chain, coordinator: &str, request: serde_json::Value) -> anyhow::Result<(String, String)> {
    let response = check(http.post(format!("{coordinator}/v1/experiments")).json(&request).send().await?).await?;
    let body: serde_json::Value = response.json().await?;
    let tx_b64 = body["transaction"].as_str().context("missing transaction")?;
    let mut tx: Transaction = bincode::deserialize(&base64::engine::general_purpose::STANDARD.decode(tx_b64)?)?;
    let blockhash = tx.message.recent_blockhash;
    tx.try_sign(&[chain.keypair.as_ref()], blockhash)?;
    let signature = chain.rpc.send_and_confirm_transaction(&tx).await?;

    Ok((body["experiment"].as_str().unwrap_or_default().to_string(), signature.to_string()))
}

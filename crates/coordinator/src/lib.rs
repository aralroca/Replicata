//! Replicata coordinator: indexes the program, stores specs and result bundles, runs the
//! permissionless crank and sells verified results over x402.

pub mod config;
pub mod crank;
pub mod db;
pub mod error;
pub mod openapi;
pub mod payments;
pub mod routes;
pub mod snapshot;
pub mod state;
pub mod views;

use std::sync::Arc;
use std::time::Duration;

use solana_client::nonblocking::rpc_client::RpcClient;
use solana_commitment_config::CommitmentConfig;
use tokio::sync::{Notify, RwLock};

pub use config::Config;
pub use state::AppState;

pub async fn build_state(cfg: Config) -> anyhow::Result<AppState> {
    let db = db::connect(&cfg.database_url).await?;
    let rpc = Arc::new(RpcClient::new_with_commitment(cfg.rpc_url.clone(), CommitmentConfig::confirmed()));
    let snapshot = Arc::new(RwLock::new(snapshot::fetch(&rpc).await?));

    Ok(AppState { cfg: Arc::new(cfg), db, snapshot, rpc, refresh: Arc::new(Notify::new()) })
}

/// Makes sure the treasury can receive x402 publish fees (facilitators do not create ATAs).
async fn ensure_treasury_ata(state: &AppState) -> anyhow::Result<()> {
    let cfg = &state.cfg;
    let ata = replicata_client::ata(&cfg.treasury, &cfg.mint);
    if state.rpc.get_account(&ata).await.is_ok() {
        return Ok(());
    }
    let crank = solana_signer::Signer::pubkey(&cfg.crank);
    state.send(&[replicata_client::create_ata_idempotent(&crank, &cfg.treasury, &cfg.mint)]).await?;
    Ok(())
}

/// Spawns the indexer and the crank, then serves the API until the process stops.
pub async fn serve(state: AppState) -> anyhow::Result<()> {
    ensure_treasury_ata(&state).await?;
    let poll = Duration::from_millis(state.cfg.poll_ms);
    tokio::spawn(snapshot::run_indexer(state.rpc.clone(), state.snapshot.clone(), state.refresh.clone(), poll));
    tokio::spawn(crank::run(state.clone(), poll));
    let listener = tokio::net::TcpListener::bind(&state.cfg.bind).await?;

    tracing::info!("coordinator listening on {} ({})", state.cfg.bind, state.cfg.network);
    axum::serve(listener, routes::router(state)).await?;
    Ok(())
}

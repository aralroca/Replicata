use std::sync::Arc;

use replicata_client::{Experiment, ProgramInstruction, Pubkey};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_signer::Signer;
use solana_transaction::Transaction;
use sqlx::PgPool;
use tokio::sync::Notify;

use crate::config::Config;
use crate::snapshot::SharedSnapshot;

#[derive(Clone)]
pub struct AppState {
    pub cfg: Arc<Config>,
    pub db: PgPool,
    pub snapshot: SharedSnapshot,
    pub rpc: Arc<RpcClient>,
    /// Wakes the indexer up after we change on-chain state.
    pub refresh: Arc<Notify>,
}

impl AppState {
    /// One experiment, read straight from the chain (the index polls, so it may be missing or
    /// stale right after a transaction); falls back to the index if the RPC read fails.
    pub async fn experiment(&self, id: &Pubkey) -> Option<Experiment> {
        let fresh = self.rpc.get_account(id).await.ok().and_then(|a| replicata_client::decode(&a.data));
        if fresh.is_some() {
            return fresh;
        }
        self.snapshot.read().await.experiments.get(id).cloned()
    }

    /// Signs with the crank keypair (fee payer), sends and confirms.
    pub async fn send(&self, instructions: &[ProgramInstruction]) -> anyhow::Result<String> {
        let blockhash = self.rpc.get_latest_blockhash().await?;
        let crank = &self.cfg.crank;
        let tx = Transaction::new_signed_with_payer(instructions, Some(&crank.pubkey()), &[crank], blockhash);
        let signature = self.rpc.send_and_confirm_transaction(&tx).await?;

        self.refresh.notify_one();
        Ok(signature.to_string())
    }
}

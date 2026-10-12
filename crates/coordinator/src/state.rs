use std::sync::Arc;

use replicata_client::ProgramInstruction;
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

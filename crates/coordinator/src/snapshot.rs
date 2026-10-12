//! In-memory index of every Replicata account, rebuilt from the chain by polling.
//!
//! The chain is the source of truth, so a restart simply re-reads it (idempotent rebuild).

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use replicata_client::{self as client, Claim, Experiment, Pubkey, WorkerProfile};
use solana_client::nonblocking::rpc_client::RpcClient;
use tokio::sync::{Notify, RwLock};

#[derive(Default, Clone)]
pub struct Snapshot {
    pub experiments: BTreeMap<Pubkey, Experiment>,
    /// Claims keyed by experiment.
    pub claims: HashMap<Pubkey, Vec<Claim>>,
    pub profiles: HashMap<Pubkey, WorkerProfile>,
    /// Token balance of each experiment vault.
    pub vault_balances: HashMap<Pubkey, u64>,
}

pub type SharedSnapshot = Arc<RwLock<Snapshot>>;

pub fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or_default()
}

impl Snapshot {
    pub fn from_accounts(accounts: Vec<(Pubkey, Vec<u8>)>) -> Self {
        let mut snapshot = Self::default();

        for (key, data) in accounts {
            if let Some(exp) = client::decode::<Experiment>(&data) {
                snapshot.experiments.insert(key, exp);
            } else if let Some(claim) = client::decode::<Claim>(&data) {
                snapshot.claims.entry(claim.experiment).or_default().push(claim);
            } else if let Some(profile) = client::decode::<WorkerProfile>(&data) {
                snapshot.profiles.insert(profile.worker, profile);
            }
        }
        snapshot.claims.values_mut().for_each(|claims| claims.sort_by_key(|c| (c.claimed_at, c.worker)));
        snapshot
    }

    pub fn claims_of(&self, experiment: &Pubkey) -> &[Claim] {
        self.claims.get(experiment).map(Vec::as_slice).unwrap_or_default()
    }

    /// Tokens in the vault beyond what the program still owes (x402 access payments).
    pub fn royalties_pending(&self, experiment: &Experiment) -> u64 {
        let balance = self.vault_balances.get(&experiment.vault).copied().unwrap_or_default();

        balance.saturating_sub(experiment.escrowed)
    }
}

/// SPL token account amount (bytes 64..72), without pulling in the token crates.
fn token_amount(data: &[u8]) -> Option<u64> {
    data.get(64..72).map(|b| u64::from_le_bytes(b.try_into().unwrap()))
}

pub async fn fetch(rpc: &RpcClient) -> anyhow::Result<Snapshot> {
    let accounts = rpc.get_program_accounts(&client::PROGRAM_ID).await?;
    let mut snapshot = Snapshot::from_accounts(accounts.into_iter().map(|(k, a)| (k, a.data)).collect());
    let vaults: Vec<Pubkey> = snapshot.experiments.values().map(|e| e.vault).collect();

    for chunk in vaults.chunks(100) {
        let accounts = rpc.get_multiple_accounts(chunk).await?;
        for (vault, account) in chunk.iter().zip(accounts) {
            if let Some(amount) = account.and_then(|a| token_amount(&a.data)) {
                snapshot.vault_balances.insert(*vault, amount);
            }
        }
    }
    Ok(snapshot)
}

/// Polls the chain forever; `refresh` wakes it up early (after a tx we know changed state).
pub async fn run_indexer(rpc: Arc<RpcClient>, shared: SharedSnapshot, refresh: Arc<Notify>, poll: Duration) {
    loop {
        match fetch(&rpc).await {
            Ok(snapshot) => *shared.write().await = snapshot,
            Err(err) => tracing::warn!("indexer: {err:#}"),
        }
        tokio::select! {
            _ = tokio::time::sleep(poll) => {}
            _ = refresh.notified() => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_token_amount() {
        let mut data = vec![0u8; 165];

        data[64..72].copy_from_slice(&42u64.to_le_bytes());
        assert_eq!(token_amount(&data), Some(42));
        assert_eq!(token_amount(&[0; 10]), None);
    }

    #[test]
    fn ignores_unknown_accounts() {
        let snapshot = Snapshot::from_accounts(vec![(Pubkey::new_unique(), vec![1, 2, 3])]);

        assert!(snapshot.experiments.is_empty() && snapshot.claims.is_empty());
    }
}

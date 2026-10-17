//! In-memory index of every Replicata account, rebuilt from the chain by polling.
//!
//! The chain is the source of truth, so a restart simply re-reads it (idempotent rebuild).

use std::collections::{BTreeMap, HashMap};
use std::str::FromStr;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::Engine;
use replicata_client::{self as client, Claim, Experiment, Pubkey, WorkerProfile};
use serde::Deserialize;
use serde_json::json;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_request::RpcRequest;
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

/// Readers clone the inner `Arc` (O(1)); the indexer swaps in a whole new snapshot.
pub type SharedSnapshot = Arc<RwLock<Arc<Snapshot>>>;

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

/// `{ context: { slot }, value }`, as returned by account reads with a context.
#[derive(Deserialize)]
struct Slotted<T> {
    context: SlotContext,
    value: T,
}

#[derive(Deserialize)]
struct SlotContext {
    slot: u64,
}

#[derive(Deserialize)]
struct Keyed {
    pubkey: String,
    account: Encoded,
}

/// An account with `encoding: "base64"`: `data` is `[payload, "base64"]`.
#[derive(Deserialize)]
struct Encoded {
    data: (String, String),
}

impl Encoded {
    fn bytes(&self) -> anyhow::Result<Vec<u8>> {
        Ok(base64::engine::general_purpose::STANDARD.decode(&self.data.0)?)
    }
}

fn keyed_accounts(keyed: &[Keyed]) -> anyhow::Result<Vec<(Pubkey, Vec<u8>)>> {
    keyed.iter().map(|k| Ok((Pubkey::from_str(&k.pubkey)?, k.account.bytes()?))).collect()
}

/// Reads the program's accounts, then the vault balances at the same slot or later. Two
/// independent reads could otherwise pair a settled experiment (`escrowed = 0`) with its vault's
/// pre-payout balance, briefly showing the payout as "royalties pending".
pub async fn fetch(rpc: &RpcClient) -> anyhow::Result<Snapshot> {
    let commitment = rpc.commitment().commitment;
    let params = json!({ "encoding": "base64", "commitment": commitment, "withContext": true });
    let program: Slotted<Vec<Keyed>> =
        rpc.send(RpcRequest::GetProgramAccounts, json!([client::PROGRAM_ID.to_string(), params])).await?;
    let mut snapshot = Snapshot::from_accounts(keyed_accounts(&program.value)?);
    let vaults: Vec<Pubkey> = snapshot.experiments.values().map(|e| e.vault).collect();
    let params = json!({ "encoding": "base64", "commitment": commitment, "minContextSlot": program.context.slot });

    for chunk in vaults.chunks(100) {
        let keys: Vec<String> = chunk.iter().map(ToString::to_string).collect();
        let read: Slotted<Vec<Option<Encoded>>> =
            rpc.send(RpcRequest::GetMultipleAccounts, json!([keys, params])).await?;
        for (vault, account) in chunk.iter().zip(read.value) {
            if let Some(amount) = account.and_then(|a| a.bytes().ok()).and_then(|data| token_amount(&data)) {
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
            Ok(snapshot) => *shared.write().await = Arc::new(snapshot),
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
    fn parses_slotted_account_reads() {
        let key = Pubkey::new_unique();
        let program: Slotted<Vec<Keyed>> = serde_json::from_value(json!({
            "context": { "slot": 77 },
            "value": [{ "pubkey": key.to_string(), "account": { "data": ["AQID", "base64"] } }],
        }))
        .unwrap();
        let vaults: Slotted<Vec<Option<Encoded>>> =
            serde_json::from_value(json!({ "context": { "slot": 78 }, "value": [null] })).unwrap();

        assert_eq!(program.context.slot, 77);
        assert_eq!(keyed_accounts(&program.value).unwrap(), vec![(key, vec![1, 2, 3])]);
        assert!(vaults.value[0].is_none());
    }

    #[test]
    fn ignores_unknown_accounts() {
        let snapshot = Snapshot::from_accounts(vec![(Pubkey::new_unique(), vec![1, 2, 3])]);

        assert!(snapshot.experiments.is_empty() && snapshot.claims.is_empty());
    }
}

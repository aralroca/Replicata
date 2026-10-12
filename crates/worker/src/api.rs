//! Minimal client of the coordinator API (see docs/API.md).

use replicata_core::JobSpec;
use serde::Deserialize;
use solana_keypair::Keypair;
use solana_signer::Signer;

#[derive(Deserialize, Debug, Clone)]
pub struct RemoteConfig {
    pub cluster: String,
    pub rpc_url: String,
    pub mint: String,
    pub faucets: Faucets,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Faucets {
    pub sol: String,
    pub usdc: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ClaimView {
    pub worker: String,
    pub state: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ExperimentView {
    pub id: String,
    pub status: String,
    pub phase: String,
    pub spec_hash: String,
    pub bond: u64,
    pub reveals_count: u8,
    pub max_claims: u8,
    pub claims_count: u8,
    pub spec: Option<JobSpec>,
    pub result_hash: Option<String>,
    #[serde(default)]
    pub royalties_paid: u64,
    pub claims: Vec<ClaimView>,
}

impl ExperimentView {
    pub fn claim_of(&self, worker: &str) -> Option<&ClaimView> {
        self.claims.iter().find(|c| c.worker == worker)
    }
}

#[derive(Clone)]
pub struct Coordinator {
    pub base: String,
    http: reqwest::Client,
}

async fn ok_json<T: serde::de::DeserializeOwned>(response: reqwest::Response) -> anyhow::Result<T> {
    Ok(crate::pay::check(response).await?.json().await?)
}

#[derive(Deserialize, Debug, Clone)]
pub struct WorkerProfile {
    pub ok: u32,
    pub bad: u32,
    pub earned: u64,
}

impl Coordinator {
    pub fn new(base: &str) -> Self {
        Self { base: base.trim_end_matches('/').to_string(), http: reqwest::Client::new() }
    }

    pub async fn config(&self) -> anyhow::Result<RemoteConfig> {
        ok_json(self.http.get(format!("{}/v1/config", self.base)).send().await?).await
    }

    pub async fn experiments(&self, query: &str) -> anyhow::Result<Vec<ExperimentView>> {
        ok_json(self.http.get(format!("{}/v1/experiments?{query}", self.base)).send().await?).await
    }

    pub async fn worker(&self, pubkey: &str) -> anyhow::Result<WorkerProfile> {
        ok_json(self.http.get(format!("{}/v1/workers/{pubkey}", self.base)).send().await?).await
    }

    pub async fn experiment(&self, id: &str) -> anyhow::Result<ExperimentView> {
        ok_json(self.http.get(format!("{}/v1/experiments/{id}", self.base)).send().await?).await
    }

    /// Uploads a bundle signed with the worker key over `experiment ‖ H`.
    pub async fn upload_bundle(
        &self,
        experiment: &str,
        worker: &Keypair,
        output_hash: &[u8; 32],
        bundle: Vec<u8>,
    ) -> anyhow::Result<()> {
        let experiment_key: replicata_client::Pubkey = experiment.parse()?;
        let message = replicata_core::bundle_upload_message(&experiment_key.to_bytes(), output_hash);
        let response = self
            .http
            .post(format!("{}/v1/experiments/{experiment}/bundles", self.base))
            .header("content-type", "application/gzip")
            .header("x-worker", worker.pubkey().to_string())
            .header("x-output-hash", replicata_core::to_hex(output_hash))
            .header("x-signature", worker.sign_message(&message).to_string())
            .body(bundle)
            .send()
            .await?;

        ok_json::<serde_json::Value>(response).await.map(|_| ())
    }
}

//! Harness for the local end-to-end tests. Requires `scripts/localnet.sh start` (validator +
//! program + local USDC + x402 facilitator + Postgres) and Docker; reads `.localnet/env`.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Context;
use replicata_client::{self as client, CreateExperimentArgs, Pubkey};
use replicata_core::JobSpec;
use replicata_worker::api::{Coordinator, ExperimentView};
use replicata_worker::chain::Chain;
use replicata_worker::job::Worker;
use replicata_worker::pay;
use solana_keypair::Keypair;
use solana_signer::Signer;

pub const USDC: u64 = 1_000_000;

pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// Loads `.localnet/env` into the process environment (once per test process).
pub fn load_env() -> HashMap<String, String> {
    let text = std::fs::read_to_string(root().join(".localnet/env")).expect("run scripts/localnet.sh start first");
    let vars: HashMap<String, String> = text
        .lines()
        .filter_map(|l| l.strip_prefix("export ")?.split_once('='))
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();

    vars.iter().for_each(|(k, v)| std::env::set_var(k, v));
    vars
}

pub struct Ctx {
    pub base: String,
    pub api: Coordinator,
    pub rpc_url: String,
    pub mint: Pubkey,
    pub facilitator_url: String,
}

impl Ctx {
    /// Starts an in-process coordinator on a random port, using `facilitator_url`.
    pub async fn start_with_facilitator(facilitator_url: Option<&str>) -> anyhow::Result<Self> {
        let env = load_env();
        std::env::set_var("BIND", "127.0.0.1:0");
        std::env::set_var("POLL_MS", "500");
        let mut cfg = replicata_coordinator::Config::from_env().await?;
        if let Some(url) = facilitator_url {
            cfg.facilitator_url = url.to_string();
        }
        let facilitator_url = cfg.facilitator_url.clone();
        let addr = replicata_coordinator::start(replicata_coordinator::build_state(cfg).await?).await?;
        let base = format!("http://{addr}");

        Ok(Self { api: Coordinator::new(&base), base, rpc_url: env["RPC_URL"].clone(), mint: env["USDC_MINT"].parse()?, facilitator_url })
    }

    pub async fn start() -> anyhow::Result<Self> {
        Self::start_with_facilitator(None).await
    }

    pub fn chain(&self, keypair: &Arc<Keypair>) -> Chain {
        Chain::new(&self.rpc_url, keypair.clone(), self.mint)
    }

    /// A fresh keypair with 10 SOL and `usdc` whole USDC (via `scripts/localnet.sh fund`).
    pub fn user(&self, usdc: u64) -> Arc<Keypair> {
        let keypair = Keypair::new();
        let status = Command::new(root().join("scripts/localnet.sh"))
            .args(["fund", &keypair.pubkey().to_string(), &usdc.to_string()])
            .status()
            .expect("localnet.sh fund");
        assert!(status.success(), "funding failed");

        Arc::new(keypair)
    }

    pub async fn balance(&self, owner: &Pubkey) -> u64 {
        let rpc = solana_client::nonblocking::rpc_client::RpcClient::new_with_commitment(
            self.rpc_url.clone(),
            solana_commitment_config::CommitmentConfig::confirmed(),
        );

        match rpc.get_token_account_balance(&client::ata(owner, &self.mint)).await {
            Ok(b) => b.amount.parse().unwrap_or_default(),
            Err(_) => 0,
        }
    }

    pub fn worker(&self, keypair: &Arc<Keypair>, chaos: bool) -> Worker {
        let chain = self.chain(keypair);
        // Inside the repo (under $HOME) so Colima/Docker Desktop can bind-mount the work dirs.
        let base = root().join("target/e2e");
        std::fs::create_dir_all(&base).unwrap();
        let home = tempfile::Builder::new().prefix("worker").tempdir_in(base).unwrap().keep();

        Worker { http: pay::paying_client(chain.keypair.clone(), chain.rpc.clone()), chain, api: self.api.clone(), home, chaos, exit_after: None }
    }

    /// What the web app does: upload the spec, then sign `create_experiment` with the author's wallet.
    pub async fn publish(&self, author: &Arc<Keypair>, spec: &JobSpec, k: u8, max_claims: u8, reward: u64, bond: u64, access_price: u64) -> anyhow::Result<Pubkey> {
        let response = reqwest::Client::new().post(format!("{}/v1/specs", self.base)).json(spec).send().await?;
        anyhow::ensure!(response.status().is_success(), "spec upload failed: {}", response.text().await?);
        let now = replicata_coordinator::snapshot::now();
        let args = CreateExperimentArgs {
            nonce: rand_nonce(),
            spec_hash: spec.spec_hash(),
            image_digest: spec.image_digest_bytes().context("digest")?,
            reward,
            bond,
            k,
            max_claims,
            commit_deadline: now + 90,
            reveal_deadline: now + 180,
            access_price,
            author_share_bps: 7_000,
        };
        let (experiment, ix) = client::create_experiment(&author.pubkey(), &self.mint, args);
        self.chain(author).send(&[ix]).await?;

        Ok(experiment)
    }

    /// Polls the coordinator until `check` holds (or panics after `timeout`).
    pub async fn wait_for(&self, id: &Pubkey, timeout: Duration, check: impl Fn(&ExperimentView) -> bool) -> ExperimentView {
        let started = Instant::now();
        loop {
            if let Ok(view) = self.api.experiment(&id.to_string()).await {
                if check(&view) {
                    return view;
                }
                assert!(started.elapsed() < timeout, "timed out; last view: status={} phase={} claims={:?}", view.status, view.phase, view.claims.iter().map(|c| &c.state).collect::<Vec<_>>());
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }
}

fn rand_nonce() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() as u64
}

pub fn example_spec(name: &str) -> JobSpec {
    JobSpec::parse(&std::fs::read(root().join("examples/experiments").join(name).join("spec.json")).unwrap()).unwrap()
}

/// Runs several workers on the same experiment concurrently.
pub async fn replicate(workers: Vec<Worker>, id: &Pubkey) -> Vec<anyhow::Result<replicata_worker::state::Stage>> {
    let id = id.to_string();
    let handles: Vec<_> = workers
        .into_iter()
        .map(|w| {
            let id = id.clone();
            tokio::spawn(async move { w.process(&id).await })
        })
        .collect();
    let mut results = Vec::new();
    for handle in handles {
        results.push(handle.await.expect("worker task panicked"));
    }
    results
}

use std::str::FromStr;

use anyhow::Context;
use replicata_client::{Pubkey, PROGRAM_ID};
use replicata_core::net;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_keypair::Keypair;
use solana_signer::Signer;

/// Runtime configuration, read from the environment (see `.env.example`).
pub struct Config {
    pub cluster: String,
    pub rpc_url: String,
    /// CAIP-2 network id derived from the RPC genesis hash.
    pub network: String,
    pub mint: Pubkey,
    pub decimals: u8,
    pub facilitator_url: String,
    pub public_url: String,
    pub bind: String,
    pub database_url: String,
    pub crank: Keypair,
    pub treasury: Pubkey,
    pub publish_fee: u64,
    pub poll_ms: u64,
    pub max_bundle_bytes: usize,
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

/// Loads a keypair from a JSON array file, a JSON array literal or a base58 string.
pub fn load_keypair(value: &str) -> anyhow::Result<Keypair> {
    let text = if value.trim_start().starts_with('[') {
        value.to_string()
    } else if std::path::Path::new(value).exists() {
        std::fs::read_to_string(value)?
    } else {
        return Keypair::try_from(bs58::decode(value).into_vec()?.as_slice()).context("invalid base58 keypair");
    };
    let bytes: Vec<u8> = serde_json::from_str(&text)?;

    Keypair::try_from(bytes.as_slice()).context("invalid keypair bytes")
}

/// `solana:<first 32 chars of the genesis hash>`.
pub async fn network_of(rpc_url: &str) -> anyhow::Result<String> {
    let genesis = RpcClient::new(rpc_url.to_string()).get_genesis_hash().await?.to_string();

    Ok(format!("solana:{}", &genesis[..32]))
}

impl Config {
    pub async fn from_env() -> anyhow::Result<Self> {
        let rpc_url = env_or("RPC_URL", net::DEVNET_RPC);
        let crank = load_keypair(&std::env::var("CRANK_KEYPAIR").context("CRANK_KEYPAIR is required")?)?;
        let treasury = match std::env::var("TREASURY") {
            Ok(t) => Pubkey::from_str(&t)?,
            Err(_) => crank.pubkey(),
        };
        let bind = env_or("BIND", "0.0.0.0:8787");

        Ok(Self {
            cluster: env_or("CLUSTER", "devnet"),
            network: network_of(&rpc_url).await.context("reading genesis hash")?,
            rpc_url,
            mint: Pubkey::from_str(&env_or("USDC_MINT", net::USDC_DEVNET))?,
            decimals: net::USDC_DECIMALS,
            facilitator_url: env_or("FACILITATOR_URL", "https://x402.org/facilitator"),
            public_url: env_or("PUBLIC_URL", "http://localhost:8787"),
            bind,
            database_url: std::env::var("DATABASE_URL").context("DATABASE_URL is required")?,
            crank,
            treasury,
            publish_fee: env_or("PUBLISH_FEE", "100000").parse()?,
            poll_ms: env_or("POLL_MS", "1500").parse()?,
            max_bundle_bytes: env_or("MAX_BUNDLE_BYTES", &(50 * 1024 * 1024).to_string()).parse()?,
        })
    }

    pub fn program_id(&self) -> Pubkey {
        PROGRAM_ID
    }
}

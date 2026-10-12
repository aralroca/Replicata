//! Local x402 facilitator for `solana-test-validator`.
//!
//! Public facilitators (x402.org, PayAI) only know devnet/mainnet, so the local end-to-end
//! tests run this one: it exposes `/verify`, `/settle`, `/supported` and `/health` for the
//! `v2-solana-exact` scheme on whatever chain `RPC_URL` points to (the CAIP-2 reference is
//! derived from the genesis hash). The fee payer must hold SOL.
//!
//! Env: `RPC_URL` (default localnet), `FEE_PAYER` (keypair JSON path), `PORT` (default 8402).

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Context;
use replicata_client::Pubkey;
use solana_account::Account;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_config::RpcSimulateTransactionConfig;
use solana_commitment_config::CommitmentConfig;
use solana_signature::Signature;
use solana_transaction::versioned::VersionedTransaction;
use x402_chain_solana::chain::config::SolanaChainConfig;
use x402_chain_solana::chain::{
    Address, SolanaChainProvider, SolanaChainProviderError, SolanaChainProviderLike, SolanaChainReference,
};
use x402_chain_solana::V2SolanaExact;
use x402_facilitator_local::{handlers, FacilitatorLocal};
use x402_types::chain::{ChainId, ChainIdPattern, ChainProviderOps, ChainRegistry, FromConfig};
use x402_types::scheme::{
    SchemeBlueprints, SchemeConfig, SchemeRegistry, X402SchemeFacilitator, X402SchemeFacilitatorBuilder,
};

/// `SolanaChainProvider` reads accounts at `finalized` (~13 s behind on a local validator), so
/// a payer funded seconds earlier looks like it has no token account. Reads go through a
/// `confirmed` client instead; signing and sending are delegated unchanged.
struct ConfirmedProvider {
    inner: SolanaChainProvider,
    rpc: RpcClient,
}

impl SolanaChainProviderLike for ConfirmedProvider {
    async fn simulate_transaction_with_config(
        &self,
        tx: &VersionedTransaction,
        cfg: RpcSimulateTransactionConfig,
    ) -> Result<(), SolanaChainProviderError> {
        self.inner.simulate_transaction_with_config(tx, cfg).await
    }

    async fn get_multiple_accounts(
        &self,
        pubkeys: &[Pubkey],
    ) -> Result<Vec<Option<Account>>, SolanaChainProviderError> {
        Ok(self.rpc.get_multiple_accounts(pubkeys).await?)
    }

    fn max_compute_unit_limit(&self) -> u32 {
        self.inner.max_compute_unit_limit()
    }

    fn max_compute_unit_price(&self) -> u64 {
        self.inner.max_compute_unit_price()
    }

    fn pubkey(&self) -> Pubkey {
        self.inner.pubkey()
    }

    fn fee_payer(&self) -> Address {
        self.inner.fee_payer()
    }

    fn sign(&self, tx: VersionedTransaction) -> Result<VersionedTransaction, SolanaChainProviderError> {
        self.inner.sign(tx)
    }

    async fn send_and_confirm(
        &self,
        tx: &VersionedTransaction,
        commitment: CommitmentConfig,
    ) -> Result<Signature, SolanaChainProviderError> {
        self.inner.send_and_confirm(tx, commitment).await
    }
}

impl ChainProviderOps for ConfirmedProvider {
    fn signer_addresses(&self) -> Vec<String> {
        self.inner.signer_addresses()
    }

    fn chain_id(&self) -> ChainId {
        self.inner.chain_id()
    }
}

/// Blueprints are built from `&P`; this wrapper hands the scheme an owned `Arc` provider.
struct Provider(Arc<ConfirmedProvider>);

impl ChainProviderOps for Provider {
    fn signer_addresses(&self) -> Vec<String> {
        self.0.signer_addresses()
    }

    fn chain_id(&self) -> ChainId {
        self.0.chain_id()
    }
}

impl X402SchemeFacilitatorBuilder<&Provider> for V2SolanaExact {
    fn build(
        &self,
        provider: &Provider,
        config: Option<serde_json::Value>,
    ) -> Result<Box<dyn X402SchemeFacilitator>, Box<dyn std::error::Error>> {
        X402SchemeFacilitatorBuilder::<Arc<ConfirmedProvider>>::build(self, provider.0.clone(), config)
    }
}

/// CAIP-2 reference of a Solana cluster: the first 32 chars of its genesis hash.
async fn chain_reference(rpc_url: &str) -> anyhow::Result<SolanaChainReference> {
    let genesis = RpcClient::new(rpc_url.to_string()).get_genesis_hash().await?.to_string();

    genesis[..32].parse().map_err(|e| anyhow::anyhow!("{e:?}"))
}

fn fee_payer_base58(path: &str) -> anyhow::Result<String> {
    let bytes: Vec<u8> =
        serde_json::from_str(&std::fs::read_to_string(path).with_context(|| format!("reading {path}"))?)?;

    Ok(bs58::encode(bytes).into_string())
}

async fn scheme_registry(rpc_url: &str, fee_payer: &str) -> anyhow::Result<(SchemeRegistry, String)> {
    let reference = chain_reference(rpc_url).await?;
    let inner: x402_chain_solana::chain::config::SolanaChainConfigInner =
        serde_json::from_value(serde_json::json!({ "signer": fee_payer_base58(fee_payer)?, "rpc": rpc_url }))?;
    let config = SolanaChainConfig { chain_reference: reference, inner };
    let inner = SolanaChainProvider::from_config(&config).await.map_err(|e| anyhow::anyhow!("{e}"))?;
    let provider = ConfirmedProvider {
        inner,
        rpc: RpcClient::new_with_commitment(rpc_url.to_string(), CommitmentConfig::confirmed()),
    };
    let chain_id = config.chain_id();
    let schemes = vec![SchemeConfig {
        enabled: true,
        id: "v2-solana-exact".into(),
        chains: chain_id.to_string().parse::<ChainIdPattern>().map_err(|e| anyhow::anyhow!("{e:?}"))?,
        config: None,
    }];
    let chains = ChainRegistry::new(HashMap::from([(chain_id.clone(), Provider(Arc::new(provider)))]));
    let blueprints = SchemeBlueprints::new().and_register(V2SolanaExact);

    Ok((SchemeRegistry::build(chains, blueprints, &schemes), chain_id.to_string()))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter(tracing_subscriber::EnvFilter::from_default_env()).init();
    let rpc_url = std::env::var("RPC_URL").unwrap_or_else(|_| "http://127.0.0.1:8899".into());
    let fee_payer = std::env::var("FEE_PAYER").context("FEE_PAYER (keypair JSON path) is required")?;
    let port = std::env::var("PORT").unwrap_or_else(|_| "8402".into());
    let (registry, chain_id) = scheme_registry(&rpc_url, &fee_payer).await?;
    let app = axum::Router::new().merge(handlers::routes().with_state(Arc::new(FacilitatorLocal::new(registry))));
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;

    println!("x402 facilitator for {chain_id} listening on :{port}");
    axum::serve(listener, app).await?;
    Ok(())
}

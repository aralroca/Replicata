//! On-chain actions of a worker (claim / commit / reveal) and wallet helpers.

use std::sync::Arc;

use replicata_client::{self as client, Experiment, ProgramInstruction, Pubkey};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_commitment_config::CommitmentConfig;
use solana_keypair::Keypair;
use solana_signer::Signer;
use solana_transaction::Transaction;

#[derive(Clone)]
pub struct Chain {
    pub rpc: Arc<RpcClient>,
    pub keypair: Arc<Keypair>,
    pub mint: Pubkey,
}

impl Chain {
    pub fn new(rpc_url: &str, keypair: Arc<Keypair>, mint: Pubkey) -> Self {
        let rpc = Arc::new(RpcClient::new_with_commitment(rpc_url.to_string(), CommitmentConfig::confirmed()));

        Self { rpc, keypair, mint }
    }

    pub fn pubkey(&self) -> Pubkey {
        self.keypair.pubkey()
    }

    pub async fn send(&self, instructions: &[ProgramInstruction]) -> anyhow::Result<String> {
        let blockhash = self.rpc.get_latest_blockhash().await?;
        let tx =
            Transaction::new_signed_with_payer(instructions, Some(&self.pubkey()), &[self.keypair.as_ref()], blockhash);

        Ok(self.rpc.send_and_confirm_transaction(&tx).await?.to_string())
    }

    pub async fn experiment(&self, id: &Pubkey) -> anyhow::Result<Experiment> {
        let account = self.rpc.get_account(id).await?;

        client::decode(&account.data).ok_or_else(|| anyhow::anyhow!("{id} is not a Replicata experiment"))
    }

    pub async fn claim(&self, id: &Pubkey) -> anyhow::Result<String> {
        let exp = self.experiment(id).await?;

        self.send(&[client::claim(&self.pubkey(), id, &exp)]).await
    }

    pub async fn commit(&self, id: &Pubkey, commitment: [u8; 32]) -> anyhow::Result<String> {
        self.send(&[client::commit(&self.pubkey(), id, commitment)]).await
    }

    pub async fn reveal(&self, id: &Pubkey, output_hash: [u8; 32], salt: [u8; 32]) -> anyhow::Result<String> {
        self.send(&[client::reveal(&self.pubkey(), id, output_hash, salt)]).await
    }

    pub async fn ensure_token_account(&self) -> anyhow::Result<()> {
        let ata = client::ata(&self.pubkey(), &self.mint);
        if self.rpc.get_account(&ata).await.is_ok() {
            return Ok(());
        }
        self.send(&[client::create_ata_idempotent(&self.pubkey(), &self.pubkey(), &self.mint)]).await.map(|_| ())
    }

    /// `(SOL lamports, token base units)`.
    pub async fn balances(&self) -> anyhow::Result<(u64, u64)> {
        let lamports = self.rpc.get_balance(&self.pubkey()).await?;
        let ata = client::ata(&self.pubkey(), &self.mint);
        let tokens = match self.rpc.get_token_account_balance(&ata).await {
            Ok(balance) => balance.amount.parse().unwrap_or_default(),
            Err(_) => 0,
        };

        Ok((lamports, tokens))
    }
}

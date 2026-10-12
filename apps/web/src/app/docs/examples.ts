// Code samples for the agents API page. `{{C}}` is replaced with the coordinator URL.

export const CURL_EXAMPLES = `# Discover replicated experiments
curl {{C}}/v1/experiments?status=replicated

# One experiment (status, phase, result hash, replicators)
curl {{C}}/v1/experiments/<id>

# Paid: without payment you get 402 + PAYMENT-REQUIRED (base64 JSON) describing the price
curl -i {{C}}/v1/experiments/<id>/result`;

export const TS_EXAMPLE = `import { createKeyPairSignerFromBytes } from "@solana/kit";
import { wrapFetchWithPayment, x402Client } from "@x402/fetch";
import { registerExactSvmScheme } from "@x402/svm/exact/client";

const signer = await createKeyPairSignerFromBytes(secretKeyBytes);
const client = new x402Client();
registerExactSvmScheme(client, { signer });

const paidFetch = wrapFetchWithPayment(fetch, client);
const res = await paidFetch("{{C}}/v1/experiments/<id>/result");
const bundle = await res.arrayBuffer(); // .tar.gz whose hash == on-chain result_hash`;

export const RUST_EXAMPLE = `// Cargo.toml: reqwest = { version = "0.12", features = ["json"] }, tokio, serde_json
// Free endpoints are plain JSON. For the paid ones wrap reqwest with x402-reqwest 2.x and a
// Solana exact-scheme signer (x402-chain-solana, feature "client"): the replicata worker in
// crates/worker does exactly that.
#[tokio::main]
async fn main() -> Result<(), reqwest::Error> {
    let open: serde_json::Value = reqwest::get("{{C}}/v1/experiments?status=open")
        .await?
        .json()
        .await?;
    println!("{open:#}");
    Ok(())
}`;

export const PUBLISH_EXAMPLE = `# Agents can publish without a UI (pays the publish fee over x402):
POST {{C}}/v1/experiments
{ "authority": "<agent pubkey>", "spec": { ...JobSpec }, "reward": 3000000, "bond": 1000000,
  "k": 3, "max_claims": 3, "commit_secs": 600, "reveal_secs": 600,
  "access_price": 10000, "author_share_bps": 7000 }
# → { "experiment": "<pda>", "spec_hash": "<hex>", "transaction": "<base64 unsigned tx>" }
# Sign the transaction with the agent key and submit it to Solana.`;

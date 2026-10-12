# Replicata coordinator API (v1)

Base URL: `COORDINATOR_URL` (local default `http://localhost:8787`). JSON everywhere unless noted.
Amounts are integers in token base units (USDC has 6 decimals: `1000000` = 1 USDC).
Hashes are 64-char lowercase hex. Public keys are base58.

The coordinator is a convenience layer: **the source of truth is the Replicata program**. The
coordinator indexes program accounts, stores specs and result bundles, runs the `finalize` /
`claim_royalties` crank and sells verified results over **x402**.

## Free endpoints

### `GET /v1/config`
```json
{
  "cluster": "localnet" | "devnet",
  "network": "solana:EtWTRABZaYq6iMfeYKouRu166VU2xqa1",   // CAIP-2, derived from the RPC genesis hash
  "rpc_url": "http://127.0.0.1:8899",
  "program_id": "G9Ssnp8rjg7gpi4Sm3EZLoC73z9pdXd9etJzUDH39Wj9",
  "mint": "<USDC mint>", "decimals": 6,
  "treasury": "<pubkey>", "publish_fee": 100000,
  "facilitator_url": "https://x402.org/facilitator",
  "faucets": { "sol": "https://faucet.solana.com", "usdc": "https://faucet.circle.com" }
}
```

### `POST /v1/specs`
Body: a `JobSpec` (see `crates/core/src/spec.rs`). Validated, canonicalised and stored by
`spec_hash = sha256(canonical_json)`. Idempotent.
- `201 {"spec_hash": "<hex>"}` (also `200` when it already existed)
- `400 {"error": "..."}` invalid spec

### `GET /v1/specs/:spec_hash` → `JobSpec` | `404`

### `GET /v1/experiments?status=&phase=&author=&limit=`
`status`: `open|replicated|unresolved|cancelled`; `phase`: `claiming|revealing|finalizable|settled`.
Returns `ExperimentView[]`, newest first (`limit` default 50, max 200).

### `GET /v1/experiments/:id` → `ExperimentView` | `404`

```jsonc
// ExperimentView
{
  "id": "<experiment PDA>", "authority": "<pk>", "mint": "<pk>", "vault": "<pk>", "nonce": 1,
  "spec_hash": "<hex>", "image_digest": "<hex>",
  "reward": 3000000, "bond": 1000000, "k": 3, "max_claims": 3,
  "claims_count": 3, "commits_count": 3, "reveals_count": 3,
  "commit_deadline": 1760000000, "reveal_deadline": 1760000600, "created_at": 1759999000,  // unix seconds
  "status": "open", "phase": "revealing",
  "result_hash": "<hex>|null", "agree_count": 0, "replicators": ["<pk>"],
  "access_price": 10000, "author_share_bps": 7000,
  "escrowed": 6000000, "royalties_paid": 0, "royalties_pending": 0,
  "spec": { /* JobSpec */ } | null,
  "claims": [
    { "worker": "<pk>", "state": "claimed|committed|revealed|paid|slashed|refunded",
      "commitment": "<hex>", "revealed_hash": "<hex>|null", "claimed_at": 1759999100, "has_bundle": true }
  ]
}
```

### `POST /v1/experiments/:id/bundles`
A worker uploads its result bundle (`.tar.gz` of `/out`) **after committing**.
Headers:
- `Content-Type: application/gzip`
- `X-Worker: <worker pubkey>`
- `X-Output-Hash: <hex>`: canonical output hash `H` (see `crates/core/src/hash.rs`)
- `X-Signature: <base58 ed25519 signature>` over `"replicata:bundle:v1:" ‖ experiment ‖ H` (raw 32-byte keys)

Checks: claim exists and is `committed`/`revealed`; signature valid; `H` recomputed from the
bundle matches; ≤ 50 MiB. Bundles are **never** served before the experiment is `replicated`.
- `201 {"output_hash": "<hex>"}` · `400` bad hash/bundle · `401` bad signature · `404` no claim · `409` wrong claim state · `413` too large

### `GET /v1/workers/:pubkey`
`{"worker": "<pk>", "claims": 3, "ok": 2, "bad": 1, "earned": 2000000, "history": [ {experiment, state} ]}` (zeros if unknown)

### `GET /v1/authors/:pubkey`
`{"author": "<pk>", "experiments": [ExperimentView], "royalties_paid": 0, "royalties_pending": 0}`

### `GET /openapi.json`: OpenAPI 3.1 description of this API.

## Paid endpoints (x402 v2, scheme `exact`, USDC)

Request without payment → `402` with header `PAYMENT-REQUIRED` (base64 JSON) listing the accepted
requirements. Pay with any x402 v2 client (`@x402/fetch` + `@x402/svm`, `x402-reqwest`), retry
with header `PAYMENT-SIGNATURE`. Successful responses carry `PAYMENT-RESPONSE` (settlement tx).
Settlement happens **before** the resource is produced: no settlement, no resource.

| Endpoint | Price | `payTo` |
|---|---|---|
| `GET /v1/experiments/:id/result` | `access_price` | experiment PDA (USDC lands in the experiment vault → royalties) |
| `GET /v1/experiments/:id/dataset/*path` | `access_price` | experiment PDA |
| `POST /v1/experiments` | `publish_fee` | treasury |

Pre-payment guards (nothing is charged): `result`/`dataset` → `404` unknown experiment,
`409 {"error":"not_replicated"}` when not replicated, `404` unknown dataset file;
`POST /v1/experiments` → `400` invalid body.

### `GET /v1/experiments/:id/result`
`200 application/gzip`: a bundle whose canonical hash equals the on-chain `result_hash`.
Headers: `X-Result-Hash`, `Content-Disposition: attachment; filename="<id>.tar.gz"`.

### `GET /v1/experiments/:id/dataset/*path`
`200` raw bytes of one file of the verified bundle.

### `POST /v1/experiments` (agents publish without a UI)
```json
{
  "authority": "<agent pubkey>", "spec": { /* JobSpec */ },
  "reward": 3000000, "bond": 1000000, "k": 3, "max_claims": 3,
  "commit_secs": 600, "reveal_secs": 600, "access_price": 10000, "author_share_bps": 7000
}
```
`200 {"experiment": "<pda>", "spec_hash": "<hex>", "transaction": "<base64 bincode tx, unsigned>"}`.
The agent signs the transaction (it is the fee payer and the on-chain author) and submits it.

## Payment failures

| Situation | Response |
|---|---|
| No / malformed payment header | `402` + `PAYMENT-REQUIRED` |
| Payer has insufficient USDC | `402` with `{"error": "insufficient_funds", "faucet": "<url>"}` (+ `hint` on localnet) |
| Same payment replayed | `409 {"error": "payment_replayed"}` |
| Facilitator unreachable | `503` + `Retry-After: 5` |
| Settlement failed | `402 {"error": "settlement_failed"}`; resource **not** delivered |

Every settled payment is stored in the `payments` table with its settlement signature.

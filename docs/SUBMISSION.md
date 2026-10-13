# Encode × Solana Hackathon 2026 — submission kit

Copy-paste material for the Encode Club project form.

**Project name:** Replicata

**Tagline (≤ 100 chars):** Results you can verify. Replication that pays — escrowed USDC bounties + x402 on Solana.

**Tracks:** Best Use of x402 on Solana · Best Consumer App on Solana

**Links**

| | |
|---|---|
| Live app (devnet) | https://replicata.vercel.app |
| Repository | https://github.com/aralroca/Replicata |
| Program (devnet) | https://explorer.solana.com/address/G9Ssnp8rjg7gpi4Sm3EZLoC73z9pdXd9etJzUDH39Wj9?cluster=devnet |
| Worker CLI | `curl -sSf https://replicata.vercel.app/install.sh \| sh` (release v0.1.0) |
| Agents API | https://github.com/aralroca/Replicata/blob/main/docs/API.md |
| Demo video | _(add the YouTube link)_ |

## Short description

Replicata turns scientific reproducibility into a paid job. A researcher publishes a computational
experiment (a Docker image pinned by digest + a command) and escrows a USDC reward on Solana. Anyone
with a CPU replicates it with one command: the worker locks a bond, runs the experiment in a sealed
sandbox, commits the hash of the output and reveals it later. A permissionless `finalize` pays the
strict majority and slashes cheaters. The verified result is then sold over x402 — to humans from
their wallet and to AI agents over plain HTTP — and every sale lands in the experiment's vault as
royalties for the author and the replicators.

## How Solana powers the product

- **Escrow:** rewards and bonds live in a vault PDA controlled by the Anchor program — no custodian.
- **Commit-reveal:** `sha256(H ‖ salt ‖ worker)` on-chain, so nobody can copy someone else's result.
- **Settlement:** permissionless `finalize` with strict majority; dissenters and withheld reveals forfeit
  their bonds; on-chain worker reputation.
- **x402 royalties:** the paywall's `payTo` is the experiment PDA, so every access pays USDC straight into
  the vault and `claim_royalties` splits it between author and replicators.
- **Agents:** agents publish experiments by paying the fee over x402 and signing the returned transaction.

## x402 integration details (for the x402 track)

- Server: `x402-axum` v2 with dynamic per-experiment pricing; settlement before execution (no resource
  without payment); replay protection keyed on the payment transaction.
- Clients: browser wallet (`@x402/fetch` + `@x402/svm`), Rust CLI (`x402-reqwest`) and raw HTTP.
- Failed payments handled explicitly: 402 with faucet link on insufficient funds, 409 on replay or
  not-yet-replicated (nothing charged), 503 + `Retry-After` when the facilitator is down, settlement
  failures never deliver.
- A self-hosted facilitator for localnet makes the whole flow testable end to end in CI.

## What existed before the hackathon

Only the research and the implementation plan. All code was written from 12 October 2026.

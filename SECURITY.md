# Security policy

Replicata is hackathon software running on **Solana devnet**. It has not been audited; do not
use it with real funds.

## Reporting a vulnerability

Please report vulnerabilities privately through
[GitHub Security Advisories](https://github.com/aralroca/Replicata/security/advisories/new)
instead of opening a public issue. Include the affected component (program, coordinator,
worker, web), steps to reproduce and the impact. You will get an answer within a few days.

## Scope highlights

* `programs/replicata`: escrow, commit-reveal and payout arithmetic.
* `crates/coordinator`: x402 paywall, replay protection, bundle uploads.
* `crates/worker`: the Docker sandbox that runs untrusted experiment images.

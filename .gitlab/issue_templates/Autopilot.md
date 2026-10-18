## What should change

<!-- One or two sentences. The Autopilot plans, implements, audits and ships it. -->

## How we know it works

<!-- The behaviour or test that proves it. -->

---

Creating this issue starts **Replicata Autopilot**: it posts a plan, opens a merge request that
merges itself only when tests, security scans and a real replication on Solana devnet pass, and
comments back here when the change is live. Changes to the on-chain program wait for a code owner.

/label ~autopilot

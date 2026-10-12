# Demo video script (2–4 min)

Setup before recording: `anchor build && scripts/localnet.sh start && scripts/coordinator-local.sh`
(or the public devnet deployment), web open on `/`, three terminals with worker keypairs, one with
the consumer keypair, explorer tab ready.

| Time | Screen | Say |
|---|---|---|
| 0:00 | Landing | "AI writes papers and reviews faster than anyone can check them. Reproducing results is the scarce part, and nobody pays for it." |
| 0:20 | `/publish` | Fill the π Monte-Carlo experiment (image by digest, command, seed), k = 3, reward 3 USDC, bond 1 USDC, access 0.01 USDC, 70/30. Sign with Phantom. "The reward is now escrowed on Solana." |
| 1:00 | 3 terminals: `replicata run --once <id>` | "Three strangers replicate it in a sealed Docker sandbox: no network, pinned image, fixed seed." Show claim → commit → reveal; switch to `/e/<id>` timeline going green; `finalize`; explorer shows USDC paid to the three workers. |
| 2:00 | Terminal: `curl -i …/result` → 402, then `replicata buy <id>` | "An AI agent pays one cent over x402 and gets the verified bundle." Show the `PAYMENT-RESPONSE`, then `/r/<author>`: royalties split 70/30 by `claim_royalties`. |
| 2:40 | `/e/<non-reproducible id>` + a `--chaos` worker | "If results disagree, the majority wins and the cheater's bond is confiscated; if nothing agrees, everyone is refunded." |
| 3:10 | Badge + `/docs` | "Paste the badge in your paper. Agents can publish and buy through the same API. Replicata: verification as a paid job, settled on Solana." |

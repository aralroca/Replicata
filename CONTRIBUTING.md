# Contributing to Replicata

Thanks for helping make reproducible science pay! Issues and pull requests are welcome.

## Development setup

You need Rust (stable), the Solana/Agave CLI 4.1, Anchor 1.2.1, Docker, Node 24+ and pnpm.

```bash
anchor build                 # program + IDL
pnpm install                 # web + codegen
scripts/localnet.sh start    # local validator, USDC, x402 facilitator, Postgres
scripts/coordinator-local.sh # coordinator on http://127.0.0.1:8787
pnpm web:dev                 # web on http://localhost:3000
```

## Before opening a pull request

```bash
cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace                    # needs TEST_DATABASE_URL (localnet Postgres by default)
scripts/e2e-local.sh 1                    # full stack end-to-end
pnpm gen:check && pnpm web:lint && pnpm web:test && pnpm web:e2e
```

* If you change the program, run `pnpm idl` to refresh `idl/replicata.json` and `pnpm gen` to regenerate the web client.
* If you change a canonical encoding (`crates/core`), regenerate the vectors with `BLESS=1 cargo test -p replicata-core` and keep the TypeScript side in sync.
* Commits follow [Conventional Commits](https://www.conventionalcommits.org) (`feat:`, `fix:`, `docs:`...).
* Every behaviour change comes with a test. CI runs everything above on each push.

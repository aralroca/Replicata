-- Experiment and claim state lives on-chain (rebuilt by the indexer on every start);
-- the database only stores what the chain cannot: specs, result bundles and x402 payments.

CREATE TABLE IF NOT EXISTS specs (
    spec_hash   TEXT PRIMARY KEY,
    spec        JSONB NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS bundles (
    experiment   TEXT NOT NULL,
    worker       TEXT NOT NULL,
    output_hash  TEXT NOT NULL,
    data         BYTEA NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (experiment, worker)
);

CREATE INDEX IF NOT EXISTS bundles_by_hash ON bundles (experiment, output_hash);

CREATE TABLE IF NOT EXISTS payments (
    id            BIGSERIAL PRIMARY KEY,
    payment_hash  TEXT NOT NULL UNIQUE,
    resource      TEXT NOT NULL,
    payer         TEXT,
    amount        BIGINT NOT NULL,
    pay_to        TEXT NOT NULL,
    tx_signature  TEXT,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

//! Postgres storage for what the chain does not hold: specs, bundles and x402 payments.

use replicata_core::JobSpec;
use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::Row;

pub async fn connect(url: &str) -> anyhow::Result<PgPool> {
    let pool = PgPoolOptions::new().max_connections(10).connect(url).await?;

    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}

/// Returns `true` when the spec was newly inserted.
pub async fn insert_spec(pool: &PgPool, spec_hash: &str, spec: &JobSpec) -> sqlx::Result<bool> {
    let result = sqlx::query("INSERT INTO specs (spec_hash, spec) VALUES ($1, $2) ON CONFLICT DO NOTHING")
        .bind(spec_hash)
        .bind(sqlx::types::Json(spec))
        .execute(pool)
        .await?;

    Ok(result.rows_affected() == 1)
}

pub async fn specs(pool: &PgPool, hashes: &[String]) -> sqlx::Result<Vec<(String, JobSpec)>> {
    let rows =
        sqlx::query("SELECT spec_hash, spec FROM specs WHERE spec_hash = ANY($1)").bind(hashes).fetch_all(pool).await?;

    Ok(rows.into_iter().map(|row| (row.get("spec_hash"), row.get::<sqlx::types::Json<JobSpec>, _>("spec").0)).collect())
}

pub async fn upsert_bundle(
    pool: &PgPool,
    experiment: &str,
    worker: &str,
    output_hash: &str,
    data: &[u8],
    files: &[String],
) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO bundles (experiment, worker, output_hash, data, files) VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (experiment, worker) DO UPDATE
         SET output_hash = EXCLUDED.output_hash, data = EXCLUDED.data, files = EXCLUDED.files, created_at = now()",
    )
    .bind(experiment)
    .bind(worker)
    .bind(output_hash)
    .bind(data)
    .bind(files)
    .execute(pool)
    .await?;
    Ok(())
}

/// File names of a bundle of `experiment` with canonical hash `output_hash` (no blob loaded).
pub async fn bundle_files(pool: &PgPool, experiment: &str, output_hash: &str) -> sqlx::Result<Option<Vec<String>>> {
    let row =
        sqlx::query("SELECT files FROM bundles WHERE experiment = $1 AND output_hash = $2 ORDER BY created_at LIMIT 1")
            .bind(experiment)
            .bind(output_hash)
            .fetch_optional(pool)
            .await?;

    Ok(row.map(|row| row.get("files")))
}

/// `(experiment, worker)` pairs that have a bundle, for the given experiments.
pub async fn bundle_keys(pool: &PgPool, experiments: &[String]) -> sqlx::Result<Vec<(String, String)>> {
    let rows = sqlx::query("SELECT experiment, worker FROM bundles WHERE experiment = ANY($1)")
        .bind(experiments)
        .fetch_all(pool)
        .await?;

    Ok(rows.into_iter().map(|row| (row.get("experiment"), row.get("worker"))).collect())
}

/// Any bundle of `experiment` whose canonical hash is `output_hash`.
pub async fn bundle_with_hash(pool: &PgPool, experiment: &str, output_hash: &str) -> sqlx::Result<Option<Vec<u8>>> {
    let row =
        sqlx::query("SELECT data FROM bundles WHERE experiment = $1 AND output_hash = $2 ORDER BY created_at LIMIT 1")
            .bind(experiment)
            .bind(output_hash)
            .fetch_optional(pool)
            .await?;

    Ok(row.map(|row| row.get("data")))
}

pub struct Payment<'a> {
    pub payment_hash: &'a str,
    pub resource: &'a str,
    pub payer: Option<&'a str>,
    pub amount: i64,
    pub pay_to: &'a str,
    pub tx_signature: Option<&'a str>,
}

pub async fn payment_exists(pool: &PgPool, payment_hash: &str) -> sqlx::Result<bool> {
    let row =
        sqlx::query("SELECT 1 FROM payments WHERE payment_hash = $1").bind(payment_hash).fetch_optional(pool).await?;

    Ok(row.is_some())
}

/// Returns `false` if the payment was already recorded (replay).
pub async fn record_payment(pool: &PgPool, p: &Payment<'_>) -> sqlx::Result<bool> {
    let result = sqlx::query(
        "INSERT INTO payments (payment_hash, resource, payer, amount, pay_to, tx_signature)
         VALUES ($1, $2, $3, $4, $5, $6) ON CONFLICT DO NOTHING",
    )
    .bind(p.payment_hash)
    .bind(p.resource)
    .bind(p.payer)
    .bind(p.amount)
    .bind(p.pay_to)
    .bind(p.tx_signature)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() == 1)
}

pub async fn payments_count(pool: &PgPool) -> sqlx::Result<i64> {
    sqlx::query("SELECT count(*) AS n FROM payments").fetch_one(pool).await.map(|row| row.get("n"))
}

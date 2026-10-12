use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,sqlx=warn"));
    tracing_subscriber::fmt().with_env_filter(filter).init();
    let cfg = replicata_coordinator::Config::from_env().await?;
    let state = replicata_coordinator::build_state(cfg).await?;

    replicata_coordinator::serve(state).await
}

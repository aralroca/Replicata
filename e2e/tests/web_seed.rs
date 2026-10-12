//! Seeds the on-chain scenarios the web Playwright suite uses in real mode
//! (`scripts/web-e2e-real.sh`) and writes their ids to `target/web-scenarios.env`.

use std::time::Duration;

use replicata_e2e::*;

#[tokio::test]
#[ignore]
async fn seed_web_scenarios() {
    let ctx = Ctx::start().await.unwrap();
    let author = ctx.user(100);
    let spec = example_spec("lorenz-int");
    let replicated = ctx.publish(&author, &spec, 1, 1, USDC, USDC, 10_000).await.unwrap();
    // Priced far above what the test wallet holds → the "insufficient funds" scenario.
    let expensive = ctx.publish(&author, &spec, 1, 1, USDC, USDC, 1_000_000 * USDC).await.unwrap();
    let pending = ctx.publish(&author, &example_spec("pi-montecarlo"), 3, 3, USDC, USDC, 10_000).await.unwrap();

    replicate(vec![ctx.worker(&ctx.user(5), false)], &replicated).await;
    replicate(vec![ctx.worker(&ctx.user(5), false)], &expensive).await;
    ctx.wait_for(&replicated, Duration::from_secs(90), |v| v.status == "replicated").await;
    ctx.wait_for(&expensive, Duration::from_secs(90), |v| v.status == "replicated").await;

    let env = format!("E2E_ID_REPLICATED={replicated}\nE2E_ID_NO_FUNDS={expensive}\nE2E_ID_NOT_REPLICATED={pending}\n");
    std::fs::write(root().join("target/web-scenarios.env"), env).unwrap();
}

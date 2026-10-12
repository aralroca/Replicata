//! Full-system tests on the local stack. Run with `scripts/e2e-local.sh` (or start
//! `scripts/localnet.sh` and `cargo test -p replicata-e2e -- --ignored --test-threads=1`).

use std::sync::{Arc, Mutex};
use std::time::Duration;

use http::Extensions;
use replicata_e2e::*;
use replicata_worker::pay;
use replicata_worker::state::Stage;
use reqwest_middleware::{ClientBuilder, Middleware, Next};
use solana_signer::Signer;
use x402_chain_solana::V2SolanaExactClient;
use x402_reqwest::X402Client;

const WAIT: Duration = Duration::from_secs(90);

fn expected_hash(name: &str) -> String {
    std::fs::read_to_string(root().join("examples/experiments").join(name).join("expected_hash"))
        .unwrap()
        .trim()
        .to_string()
}

#[tokio::test]
#[ignore]
async fn happy_path_replicates_pays_workers_and_sells_the_result() {
    let ctx = Ctx::start().await.unwrap();
    let author = ctx.user(100);
    let workers: Vec<_> = (0..3).map(|_| ctx.user(10)).collect();
    let consumer = ctx.user(1);
    let author_start = ctx.balance(&author.pubkey()).await;
    let id = ctx.publish(&author, &example_spec("pi-montecarlo"), terms(3, 3 * USDC, USDC, 10_000)).await.unwrap();

    let stages = replicate(workers.iter().map(|w| ctx.worker(w, false)).collect(), &id).await;
    assert!(stages.iter().all(|s| matches!(s, Ok(Stage::Revealed))), "{stages:?}");
    let view = ctx.wait_for(&id, WAIT, |v| v.status == "replicated").await;
    assert_eq!(view.result_hash.as_deref(), Some(expected_hash("pi-montecarlo").as_str()));
    for w in &workers {
        assert_eq!(ctx.balance(&w.pubkey()).await, 11 * USDC, "bond back + 1 USDC reward");
    }
    assert_eq!(ctx.balance(&author.pubkey()).await, author_start - 3 * USDC);

    // A consumer (or agent) pays 0.01 USDC over x402 and gets the verified bundle.
    let chain = ctx.chain(&consumer);
    let http = pay::paying_client(chain.keypair.clone(), chain.rpc.clone());
    let bought = pay::buy(&http, &ctx.base, &id.to_string()).await.unwrap();
    assert_eq!(bought.result_hash, expected_hash("pi-montecarlo"));
    assert!(bought.settlement.is_some(), "PAYMENT-RESPONSE header present");
    assert_eq!(ctx.balance(&consumer.pubkey()).await, USDC - 10_000);

    // The crank distributes the access payment: 70% author, 10% per replicator.
    ctx.wait_for(&id, WAIT, |v| v.claims.len() == 3 && v.status == "replicated" && v.royalties_paid == 10_000).await;
    assert_eq!(ctx.balance(&author.pubkey()).await, author_start - 3 * USDC + 7_000);
    for w in &workers {
        assert_eq!(ctx.balance(&w.pubkey()).await, 11 * USDC + 1_000);
    }
}

#[tokio::test]
#[ignore]
async fn a_cheating_worker_loses_its_bond_to_the_honest_majority() {
    let ctx = Ctx::start().await.unwrap();
    let author = ctx.user(100);
    let workers: Vec<_> = (0..3).map(|_| ctx.user(10)).collect();
    let id = ctx.publish(&author, &example_spec("lorenz-int"), terms(3, 3 * USDC, USDC, 10_000)).await.unwrap();

    let jobs = vec![ctx.worker(&workers[0], false), ctx.worker(&workers[1], true), ctx.worker(&workers[2], false)];
    replicate(jobs, &id).await;
    let view = ctx.wait_for(&id, WAIT, |v| v.status == "replicated").await;

    assert_eq!(view.result_hash.as_deref(), Some(expected_hash("lorenz-int").as_str()));
    assert_eq!(ctx.balance(&workers[1].pubkey()).await, 9 * USDC, "cheater's bond confiscated");
    // (3 reward + 1 confiscated bond) / 2 honest workers, plus their own bond back.
    assert_eq!(ctx.balance(&workers[0].pubkey()).await, 12 * USDC);
    assert_eq!(ctx.balance(&workers[2].pubkey()).await, 12 * USDC);
    assert_eq!(view.claim_of(&workers[1].pubkey().to_string()).unwrap().state, "slashed");
}

#[tokio::test]
#[ignore]
async fn a_non_reproducible_experiment_is_unresolved_and_everyone_is_refunded() {
    let ctx = Ctx::start().await.unwrap();
    let author = ctx.user(100);
    let workers: Vec<_> = (0..3).map(|_| ctx.user(10)).collect();
    let author_start = ctx.balance(&author.pubkey()).await;
    let id = ctx.publish(&author, &example_spec("non-reproducible"), terms(3, 3 * USDC, USDC, 10_000)).await.unwrap();

    replicate(workers.iter().map(|w| ctx.worker(w, false)).collect(), &id).await;
    ctx.wait_for(&id, WAIT, |v| v.status == "unresolved").await;

    assert_eq!(ctx.balance(&author.pubkey()).await, author_start);
    for w in &workers {
        assert_eq!(ctx.balance(&w.pubkey()).await, 10 * USDC);
    }
    let response = reqwest::get(format!("{}/v1/experiments/{id}/result", ctx.base)).await.unwrap();
    assert_eq!(response.status(), 409, "an unresolved result is never sold");
}

/// Replicates a 1-of-1 experiment quickly and returns its id.
async fn replicated_experiment(ctx: &Ctx) -> replicata_client::Pubkey {
    let author = ctx.user(10);
    let worker = ctx.user(5);
    let id = ctx.publish(&author, &example_spec("lorenz-int"), terms(1, USDC, USDC, 10_000)).await.unwrap();

    replicate(vec![ctx.worker(&worker, false)], &id).await;
    ctx.wait_for(&id, WAIT, |v| v.status == "replicated").await;
    id
}

#[derive(Clone, Default)]
struct Recorder(Arc<Mutex<Vec<String>>>);

#[async_trait::async_trait]
impl Middleware for Recorder {
    async fn handle(
        &self,
        req: reqwest::Request,
        ext: &mut Extensions,
        next: Next<'_>,
    ) -> reqwest_middleware::Result<reqwest::Response> {
        if let Some(header) = req.headers().get("payment-signature") {
            self.0.lock().unwrap().push(header.to_str().unwrap().to_string());
        }
        next.run(req, ext).await
    }
}

#[tokio::test]
#[ignore]
async fn failed_and_replayed_payments_never_deliver_or_double_charge() {
    let ctx = Ctx::start().await.unwrap();
    let id = replicated_experiment(&ctx).await;
    let result_url = format!("{}/v1/experiments/{id}/result", ctx.base);

    // Not replicated yet → 409 before any payment is attempted.
    let author = ctx.user(10);
    let open = ctx.publish(&author, &example_spec("lorenz-int"), terms(1, USDC, 0, 10_000)).await.unwrap();
    let payer = ctx.user(1);
    let chain = ctx.chain(&payer);
    let http = pay::paying_client(chain.keypair.clone(), chain.rpc.clone());
    let not_ready = http.get(format!("{}/v1/experiments/{open}/result", ctx.base)).send().await.unwrap();
    assert_eq!(not_ready.status(), 409);
    assert_eq!(ctx.balance(&payer.pubkey()).await, USDC, "nothing charged");

    // No payment → 402 with the PAYMENT-REQUIRED header.
    let unpaid = reqwest::get(&result_url).await.unwrap();
    assert_eq!(unpaid.status(), 402);
    assert!(unpaid.headers().contains_key("payment-required"));

    // Insufficient funds → 402 insufficient_funds with a faucet hint; nothing delivered.
    let broke = ctx.user(0);
    let broke_chain = ctx.chain(&broke);
    let broke_http = pay::paying_client(broke_chain.keypair.clone(), broke_chain.rpc.clone());
    let response = broke_http.get(&result_url).send().await.unwrap();
    assert_eq!(response.status(), 402);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["error"], "insufficient_funds", "{body}");
    assert!(body["faucet"].is_string());

    // A successful payment can't be replayed.
    let recorder = Recorder::default();
    let x402 = X402Client::new()
        .register(V2SolanaExactClient::new(chain.keypair.clone(), pay::ConfirmedRpc(chain.rpc.clone())));
    let recording = ClientBuilder::new(reqwest::Client::new()).with(x402).with(recorder.clone()).build();
    assert_eq!(recording.get(&result_url).send().await.unwrap().status(), 200);
    let header = recorder.0.lock().unwrap().last().cloned().expect("a payment header was sent");
    let replay = reqwest::Client::new().get(&result_url).header("payment-signature", header).send().await.unwrap();
    assert_eq!(replay.status(), 409);
    assert_eq!(replay.json::<serde_json::Value>().await.unwrap()["error"], "payment_replayed");
    assert_eq!(ctx.balance(&payer.pubkey()).await, USDC - 10_000, "charged exactly once");
}

#[tokio::test]
#[ignore]
async fn an_unreachable_facilitator_returns_503_with_retry_after() {
    let ctx = Ctx::start().await.unwrap();
    let id = replicated_experiment(&ctx).await;
    let supported = reqwest::get(format!("{}/supported", ctx.facilitator_url)).await.unwrap().text().await.unwrap();
    let flaky = flaky_facilitator(&supported).await;
    let degraded = Ctx::start_with_facilitator(Some(&flaky)).await.unwrap();
    let payer = ctx.user(1);
    let chain = degraded.chain(&payer);
    let http = pay::paying_client(chain.keypair.clone(), chain.rpc.clone());

    let response = http.get(format!("{}/v1/experiments/{id}/result", degraded.base)).send().await.unwrap();
    assert_eq!(response.status(), 503);
    assert_eq!(response.headers().get("retry-after").unwrap(), "5");
    assert_eq!(ctx.balance(&payer.pubkey()).await, USDC, "nothing charged");
}

/// A facilitator that advertises the real capabilities but whose `/settle` is down.
async fn flaky_facilitator(supported: &str) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let supported = supported.to_string();
    tokio::spawn(async move {
        loop {
            let (mut socket, _) = listener.accept().await.unwrap();
            let supported = supported.clone();
            tokio::spawn(async move {
                use tokio::io::{AsyncReadExt, AsyncWriteExt};
                let mut buf = vec![0u8; 64 * 1024];
                let n = socket.read(&mut buf).await.unwrap_or(0);
                let request = String::from_utf8_lossy(&buf[..n]);
                let response = if request.starts_with("GET /supported") {
                    format!(
                        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{supported}",
                        supported.len()
                    )
                } else {
                    "HTTP/1.1 503 Service Unavailable\r\ncontent-length: 0\r\n\r\n".to_string()
                };
                let _ = socket.write_all(response.as_bytes()).await;
            });
        }
    });
    format!("http://{addr}")
}

#[tokio::test]
#[ignore]
async fn an_agent_publishes_by_paying_the_fee_over_x402() {
    let ctx = Ctx::start().await.unwrap();
    let agent = ctx.user(10);
    let config: serde_json::Value =
        reqwest::get(format!("{}/v1/config", ctx.base)).await.unwrap().json().await.unwrap();
    let treasury: replicata_client::Pubkey = config["treasury"].as_str().unwrap().parse().unwrap();
    let treasury_before = ctx.balance(&treasury).await;
    let mut request: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(root().join("examples/experiments/lorenz-int/publish.json")).unwrap(),
    )
    .unwrap();
    request["authority"] = agent.pubkey().to_string().into();
    let chain = ctx.chain(&agent);
    let http = pay::paying_client(chain.keypair.clone(), chain.rpc.clone());

    let (experiment, _sig) = pay::publish(&http, &chain, &ctx.base, request).await.unwrap();
    let view = ctx.wait_for(&experiment.parse().unwrap(), WAIT, |v| v.status == "open").await;

    assert!(view.spec.is_some(), "the coordinator stored the spec");
    assert_eq!(ctx.balance(&treasury).await, treasury_before + 100_000, "publish fee paid to the treasury");
    assert_eq!(ctx.balance(&agent.pubkey()).await, 10 * USDC - 100_000 - 3 * USDC, "fee + escrowed reward");

    let bad = http
        .post(format!("{}/v1/experiments", ctx.base))
        .json(&serde_json::json!({"authority": "x"}))
        .send()
        .await
        .unwrap();
    assert_eq!(bad.status(), 400, "invalid bodies are rejected before payment");
}

#[tokio::test]
#[ignore]
async fn a_worker_that_crashes_after_committing_resumes_and_reveals() {
    let ctx = Ctx::start().await.unwrap();
    let author = ctx.user(10);
    let keypair = ctx.user(5);
    let id = ctx.publish(&author, &example_spec("lorenz-int"), terms(1, USDC, USDC, 10_000)).await.unwrap();
    let mut first = ctx.worker(&keypair, false);
    first.exit_after = Some(Stage::Committed);

    assert_eq!(first.process(&id.to_string()).await.unwrap(), Stage::Committed);
    let resumed = replicata_worker::job::Worker { exit_after: None, ..first };
    assert_eq!(resumed.process(&id.to_string()).await.unwrap(), Stage::Revealed);
    ctx.wait_for(&id, WAIT, |v| v.status == "replicated").await;
    assert_eq!(ctx.balance(&keypair.pubkey()).await, 6 * USDC);
}

#[tokio::test]
#[ignore]
async fn closing_your_token_account_cannot_block_settlement() {
    let ctx = Ctx::start().await.unwrap();
    let author = ctx.user(10);
    let saboteur = ctx.user(1);
    let id = ctx.publish(&author, &example_spec("lorenz-int"), terms(1, USDC, USDC, 10_000)).await.unwrap();
    let mut worker = ctx.worker(&saboteur, false);
    worker.exit_after = Some(Stage::Uploaded);
    worker.process(&id.to_string()).await.unwrap();

    // Its whole balance is locked as bond, so it can close its (empty) token account.
    let ata = replicata_client::ata(&saboteur.pubkey(), &ctx.mint);
    let close = spl_token_interface::instruction::close_account(
        &spl_token_interface::ID,
        &ata,
        &saboteur.pubkey(),
        &saboteur.pubkey(),
        &[],
    )
    .unwrap();
    ctx.chain(&saboteur).send(&[close]).await.unwrap();

    let resumed = replicata_worker::job::Worker { exit_after: None, ..worker };
    assert_eq!(resumed.process(&id.to_string()).await.unwrap(), Stage::Revealed);
    ctx.wait_for(&id, WAIT, |v| v.status == "replicated").await;
    assert_eq!(ctx.balance(&saboteur.pubkey()).await, 2 * USDC, "the crank recreated the account and paid it");
}

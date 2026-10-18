//! Route-level tests: real Postgres (`TEST_DATABASE_URL`, defaults to the localnet one), an
//! injected chain snapshot (no validator needed) and a `wiremock` x402 facilitator.

use std::collections::BTreeMap;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use base64::Engine;
use replicata_client::{Claim, ClaimState, Experiment, ExperimentStatus, Pubkey, MAX_CLAIMS};
use replicata_coordinator::snapshot::{now, Snapshot};
use replicata_coordinator::{db, routes, AppState, Config};
use replicata_core::{bundle, to_hex, JobSpec};
use serde_json::{json, Value};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_keypair::Keypair;
use tokio::sync::{Notify, RwLock};
use tower::ServiceExt;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const NETWORK: &str = "solana:EtWTRABZaYq6iMfeYKouRu166VU2xqa1";

fn database_url() -> String {
    std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://replicata:replicata@127.0.0.1:54329/replicata".into())
}

fn spec(seed: u64) -> JobSpec {
    JobSpec {
        version: 1,
        title: "route test".into(),
        description: String::new(),
        image: "alpine".into(),
        image_digest: format!("sha256:{}", "ab".repeat(32)),
        cmd: vec!["true".into()],
        env: BTreeMap::new(),
        inputs: vec![],
        cpus: 1,
        mem_mb: 128,
        timeout_s: 10,
        seed,
    }
}

fn experiment(authority: Pubkey, status: ExperimentStatus, spec: &JobSpec, result_hash: [u8; 32]) -> Experiment {
    let now = now();

    Experiment {
        authority,
        mint: Pubkey::new_unique(),
        vault: Pubkey::new_unique(),
        nonce: 1,
        spec_hash: spec.spec_hash(),
        image_digest: spec.image_digest_bytes().unwrap(),
        reward: 3_000_000,
        bond: 1_000_000,
        k: 1,
        max_claims: 1,
        claims_count: 1,
        commits_count: 1,
        reveals_count: 1,
        commit_deadline: now + 100,
        reveal_deadline: now + 200,
        status,
        result_hash,
        agree_count: u8::from(status == ExperimentStatus::Replicated),
        replicators: [Pubkey::default(); MAX_CLAIMS as usize],
        access_price: 10_000,
        author_share_bps: 7_000,
        escrowed: 0,
        royalties_paid: 0,
        created_at: now,
        bump: 255,
    }
}

struct Fixture {
    app: Router,
    state: AppState,
    open: Pubkey,
    replicated: Pubkey,
    author: Pubkey,
    worker: Pubkey,
    bundle: Vec<u8>,
}

async fn fixture(facilitator: &str) -> Fixture {
    let (author, worker) = (Pubkey::new_unique(), Pubkey::new_unique());
    let (open, replicated) = (Pubkey::new_unique(), Pubkey::new_unique());
    let bundle = bundle::pack_files(&[
        ("plots/run 1.csv".into(), b"1,2\n".to_vec()),
        ("result.json".into(), b"{\"ok\":true}".to_vec()),
    ])
    .unwrap();
    let result_hash = bundle::hash_bundle(&bundle).unwrap();
    let (open_spec, replicated_spec) = (spec(rand_seed()), spec(rand_seed()));
    let mut snapshot = Snapshot::default();
    snapshot.experiments.insert(open, experiment(author, ExperimentStatus::Open, &open_spec, [0; 32]));
    snapshot
        .experiments
        .insert(replicated, experiment(author, ExperimentStatus::Replicated, &replicated_spec, result_hash));
    snapshot.claims.insert(open, vec![claim(open, worker, ClaimState::Committed)]);
    let db = db::connect(&database_url())
        .await
        .expect("Postgres for tests: run scripts/localnet.sh start or set TEST_DATABASE_URL");
    db::insert_spec(&db, &to_hex(&open_spec.spec_hash()), &open_spec).await.unwrap();
    let files = vec!["plots/run 1.csv".to_string(), "result.json".to_string()];
    db::upsert_bundle(&db, &replicated.to_string(), &worker.to_string(), &to_hex(&result_hash), &bundle, &files)
        .await
        .unwrap();
    let state = AppState {
        cfg: Arc::new(config(facilitator)),
        db,
        snapshot: Arc::new(RwLock::new(Arc::new(snapshot))),
        rpc: Arc::new(RpcClient::new("http://127.0.0.1:1".into())),
        refresh: Arc::new(Notify::new()),
    };

    Fixture { app: routes::router(state.clone()), state, open, replicated, author, worker, bundle }
}

/// Unique per call, across parallel tests and across runs sharing the same database.
fn rand_seed() -> u64 {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() as u64;

    nanos.wrapping_add(COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
}

fn claim(experiment: Pubkey, worker: Pubkey, state: ClaimState) -> Claim {
    Claim {
        experiment,
        worker,
        commitment: [1; 32],
        revealed_hash: [0; 32],
        salt: [0; 32],
        state,
        claimed_at: now(),
        bump: 255,
    }
}

fn config(facilitator: &str) -> Config {
    Config {
        cluster: "devnet".into(),
        rpc_url: "http://127.0.0.1:1".into(),
        network: NETWORK.into(),
        mint: replicata_core::net::USDC_DEVNET.parse().unwrap(),
        decimals: 6,
        facilitator_url: facilitator.into(),
        public_url: "http://localhost:8787".into(),
        bind: "127.0.0.1:0".into(),
        database_url: database_url(),
        crank: Keypair::new(),
        treasury: Pubkey::new_unique(),
        publish_fee: 100_000,
        poll_ms: 1_000,
        max_bundle_bytes: 1024 * 1024,
    }
}

async fn call(app: &Router, req: Request<Body>) -> (StatusCode, axum::http::HeaderMap, Vec<u8>) {
    let response = app.clone().oneshot(req).await.unwrap();
    let (parts, body) = response.into_parts();

    (parts.status, parts.headers, axum::body::to_bytes(body, usize::MAX).await.unwrap().to_vec())
}

async fn get(app: &Router, uri: &str) -> (StatusCode, Value) {
    let (status, _, body) = call(app, Request::get(uri).body(Body::empty()).unwrap()).await;

    (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
}

/// A facilitator that advertises devnet support (so price tags get a fee payer).
async fn facilitator() -> MockServer {
    let server = MockServer::start().await;
    let supported = json!({"kinds": [{"x402Version": 2, "scheme": "exact", "network": NETWORK, "extra": {"feePayer": Pubkey::new_unique().to_string()}}], "extensions": [], "signers": {}});
    Mock::given(method("GET"))
        .and(path("/supported"))
        .respond_with(ResponseTemplate::new(200).set_body_json(supported))
        .mount(&server)
        .await;
    server
}

#[tokio::test]
async fn config_and_openapi() {
    let f = fixture("http://127.0.0.1:9").await;
    let (status, config) = get(&f.app, "/v1/config").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(config["network"], NETWORK);
    assert_eq!(config["program_id"], replicata_client::PROGRAM_ID.to_string());
    let (status, openapi) = get(&f.app, "/openapi.json").await;
    assert_eq!(status, StatusCode::OK);
    assert!(openapi["paths"]["/v1/experiments/{id}/result"].is_object());
    let (status, _, version) = call(&f.app, Request::get("/version").body(Body::empty()).unwrap()).await;
    assert_eq!(status, StatusCode::OK);
    assert!(!version.is_empty(), "the serving build identifies itself");
}

#[tokio::test]
async fn specs_are_validated_canonicalised_and_idempotent() {
    let f = fixture("http://127.0.0.1:9").await;
    let body = serde_json::to_vec(&spec(rand_seed())).unwrap();
    let post = |body: Vec<u8>| {
        Request::post("/v1/specs").header("content-type", "application/json").body(Body::from(body)).unwrap()
    };

    let (status, _, created) = call(&f.app, post(body.clone())).await;
    assert_eq!(status, StatusCode::CREATED);
    let hash = serde_json::from_slice::<Value>(&created).unwrap()["spec_hash"].as_str().unwrap().to_string();
    assert_eq!(call(&f.app, post(body)).await.0, StatusCode::OK);
    assert_eq!(get(&f.app, &format!("/v1/specs/{hash}")).await.0, StatusCode::OK);
    assert_eq!(call(&f.app, post(b"{\"version\":1}".to_vec())).await.0, StatusCode::BAD_REQUEST);
    assert_eq!(get(&f.app, &format!("/v1/specs/{}", "00".repeat(32))).await.0, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn experiments_list_filters_and_detail() {
    let f = fixture("http://127.0.0.1:9").await;
    let (status, list) = get(&f.app, &format!("/v1/experiments?author={}", f.author)).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(list.as_array().unwrap().len(), 2);
    let (_, replicated) = get(&f.app, &format!("/v1/experiments?author={}&status=replicated", f.author)).await;
    assert_eq!(replicated[0]["id"], f.replicated.to_string());
    let (status, open) = get(&f.app, &format!("/v1/experiments/{}", f.open)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(open["status"], "open");
    assert_eq!(open["spec"]["title"], "route test", "spec joined from the database");
    assert_eq!(open["claims"][0]["state"], "committed");
    assert_eq!(get(&f.app, &format!("/v1/experiments/{}", Pubkey::new_unique())).await.0, StatusCode::NOT_FOUND);
    assert_eq!(get(&f.app, "/v1/experiments/not-a-key").await.0, StatusCode::BAD_REQUEST);
}

/// JSON-RPC `getAccountInfo` answer for a Replicata account.
fn account_info(experiment: &Experiment) -> Value {
    let data = replicata_client::encode(experiment);
    let base64 = base64::engine::general_purpose::STANDARD.encode(&data);
    let value = json!({ "data": [base64, "base64"], "executable": false, "lamports": 1,
        "owner": replicata_client::PROGRAM_ID.to_string(), "rentEpoch": 0, "space": data.len() });

    json!({ "jsonrpc": "2.0", "id": 1, "result": { "context": { "slot": 9 }, "value": value } })
}

#[tokio::test]
async fn experiment_detail_renders_one_consistent_moment() {
    let mut f = fixture("http://127.0.0.1:9").await;
    let rpc = MockServer::start().await;
    let indexed = f.state.snapshot.read().await.experiments[&f.open].clone();
    // The chain is already past the index (e.g. just finalized) while the claims are not.
    let newer = Experiment { status: ExperimentStatus::Replicated, escrowed: 0, ..indexed };
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(account_info(&newer)))
        .mount(&rpc)
        .await;
    f.state.rpc = Arc::new(RpcClient::new(rpc.uri()));
    let app = routes::router(f.state.clone());

    let (_, open) = get(&app, &format!("/v1/experiments/{}", f.open)).await;
    assert_eq!((open["status"].as_str(), open["claims"][0]["state"].as_str()), (Some("open"), Some("committed")));
    let (status, unindexed) = get(&app, &format!("/v1/experiments/{}", Pubkey::new_unique())).await;
    assert_eq!((status, unindexed["status"].as_str()), (StatusCode::OK, Some("replicated")), "not indexed yet: chain");
}

#[tokio::test]
async fn workers_and_authors() {
    let f = fixture("http://127.0.0.1:9").await;
    let (_, worker) = get(&f.app, &format!("/v1/workers/{}", f.worker)).await;
    let (_, author) = get(&f.app, &format!("/v1/authors/{}", f.author)).await;

    assert_eq!(worker["history"][0]["experiment"], f.open.to_string());
    assert_eq!(worker["ok"], 0);
    assert_eq!(author["experiments"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn paid_routes_refuse_before_charging() {
    let server = facilitator().await;
    let f = fixture(&server.uri()).await;

    let (status, body) = get(&f.app, &format!("/v1/experiments/{}/result", f.open)).await;
    assert_eq!((status, body["error"].as_str()), (StatusCode::CONFLICT, Some("not_replicated")));
    assert_eq!(get(&f.app, &format!("/v1/experiments/{}/result", Pubkey::new_unique())).await.0, StatusCode::NOT_FOUND);
    assert_eq!(
        get(&f.app, &format!("/v1/experiments/{}/dataset/missing.csv", f.replicated)).await.0,
        StatusCode::NOT_FOUND
    );
    let bad_publish = Request::post("/v1/experiments")
        .header("content-type", "application/json")
        .body(Body::from("{\"authority\":\"x\"}"))
        .unwrap();
    assert_eq!(call(&f.app, bad_publish).await.0, StatusCode::BAD_REQUEST);
    assert!(
        server.received_requests().await.unwrap().iter().all(|r| r.url.path() == "/supported"),
        "no verify/settle calls"
    );
}

#[tokio::test]
async fn replicated_result_requires_an_x402_payment() {
    let server = facilitator().await;
    let f = fixture(&server.uri()).await;
    let (status, headers, _) =
        call(&f.app, Request::get(format!("/v1/experiments/{}/result", f.replicated)).body(Body::empty()).unwrap())
            .await;

    assert_eq!(status, StatusCode::PAYMENT_REQUIRED);
    let required = base64::engine::general_purpose::STANDARD.decode(headers.get("payment-required").unwrap()).unwrap();
    let required: Value = serde_json::from_slice(&required).unwrap();
    let accept = &required["accepts"][0];
    assert_eq!(accept["payTo"], f.replicated.to_string(), "payments land in the experiment vault");
    assert_eq!(accept["amount"], "10000");
    assert_eq!(accept["network"], NETWORK);
    assert!(accept["extra"]["feePayer"].is_string());
}

/// A syntactically valid V2 payment header for `accepts[0]` (the facilitator is mocked).
async fn payment_header(f: &Fixture) -> String {
    let (_, headers, _) =
        call(&f.app, Request::get(format!("/v1/experiments/{}/result", f.replicated)).body(Body::empty()).unwrap())
            .await;
    let b64 = base64::engine::general_purpose::STANDARD;
    let required: Value =
        serde_json::from_slice(&b64.decode(headers.get("payment-required").unwrap()).unwrap()).unwrap();
    let payload = json!({"x402Version": 2, "accepted": required["accepts"][0], "resource": required["resource"], "payload": {"transaction": b64.encode(rand_seed().to_le_bytes())}});

    b64.encode(serde_json::to_vec(&payload).unwrap())
}

async fn paid_get(f: &Fixture, header: &str) -> (StatusCode, axum::http::HeaderMap, Vec<u8>) {
    let req = Request::get(format!("/v1/experiments/{}/result", f.replicated))
        .header("payment-signature", header)
        .body(Body::empty())
        .unwrap();

    call(&f.app, req).await
}

#[tokio::test]
async fn settled_payment_delivers_once_and_replays_are_rejected() {
    let server = facilitator().await;
    Mock::given(method("POST"))
        .and(path("/settle"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"success": true, "transaction": format!("sig{}", rand_seed()), "network": NETWORK, "payer": "payer1"})),
        )
        .mount(&server)
        .await;
    let f = fixture(&server.uri()).await;
    let header = payment_header(&f).await;

    let (status, headers, body) = paid_get(&f, &header).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, f.bundle);
    assert_eq!(
        headers.get("x-result-hash").unwrap().to_str().unwrap(),
        to_hex(&bundle::hash_bundle(&f.bundle).unwrap())
    );
    assert!(headers.contains_key("payment-response"));
    assert!(db::payment_exists(&f.state.db, &hash_of(&header)).await.unwrap());

    let (status, _, body) = paid_get(&f, &header).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(serde_json::from_slice::<Value>(&body).unwrap()["error"], "payment_replayed");
}

fn hash_of(header: &str) -> String {
    let mut headers = axum::http::HeaderMap::new();
    headers.insert("payment-signature", header.parse().unwrap());

    replicata_coordinator::payments::payment_hash(&headers).unwrap()
}

#[tokio::test]
async fn facilitator_outage_is_a_503_with_retry_after() {
    let server = facilitator().await;
    Mock::given(method("POST")).and(path("/settle")).respond_with(ResponseTemplate::new(503)).mount(&server).await;
    let f = fixture(&server.uri()).await;

    let (status, headers, body) = paid_get(&f, &payment_header(&f).await).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{}", String::from_utf8_lossy(&body));
    assert_eq!(headers.get("retry-after").unwrap(), "5");
    assert_eq!(serde_json::from_slice::<Value>(&body).unwrap()["error"], "facilitator_unavailable");
}

#[tokio::test]
async fn failed_settlement_delivers_nothing() {
    let server = facilitator().await;
    Mock::given(method("POST"))
        .and(path("/settle"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"success": false, "errorReason": "transaction_simulation", "network": NETWORK})),
        )
        .mount(&server)
        .await;
    let f = fixture(&server.uri()).await;
    let header = payment_header(&f).await;

    let (status, _, body) = paid_get(&f, &header).await;
    assert_eq!(status, StatusCode::PAYMENT_REQUIRED);
    assert_eq!(serde_json::from_slice::<Value>(&body).unwrap()["error"], "settlement_failed");
    assert!(!db::payment_exists(&f.state.db, &hash_of(&header)).await.unwrap(), "nothing recorded");
}

#[tokio::test]
async fn bundle_uploads_require_a_valid_signature() {
    let f = fixture("http://127.0.0.1:9").await;
    let upload = |signature: &str| {
        Request::post(format!("/v1/experiments/{}/bundles", f.open))
            .header("x-worker", f.worker.to_string())
            .header("x-output-hash", to_hex(&bundle::hash_bundle(&f.bundle).unwrap()))
            .header("x-signature", signature)
            .body(Body::from(f.bundle.clone()))
            .unwrap()
    };

    assert_eq!(call(&f.app, upload("not-a-signature")).await.0, StatusCode::UNAUTHORIZED);
    let wrong = solana_signer::Signer::sign_message(&Keypair::new(), b"whatever").to_string();
    assert_eq!(call(&f.app, upload(&wrong)).await.0, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn dataset_names_are_percent_decoded_and_unknown_files_cost_nothing() {
    let server = facilitator().await;
    Mock::given(method("POST"))
        .and(path("/settle"))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            json!({"success": true, "transaction": format!("sig{}", rand_seed()), "network": NETWORK, "payer": "p"}),
        ))
        .mount(&server)
        .await;
    let f = fixture(&server.uri()).await;
    let base = format!("/v1/experiments/{}/dataset", f.replicated);

    assert_eq!(get(&f.app, &format!("{base}/missing.csv")).await.0, StatusCode::NOT_FOUND);
    assert_eq!(get(&f.app, &format!("{base}/plots/run%201.csv")).await.0, StatusCode::PAYMENT_REQUIRED);
    let req = Request::get(format!("{base}/plots/run%201.csv"))
        .header("payment-signature", payment_header(&f).await)
        .body(Body::empty())
        .unwrap();
    let (status, _, body) = call(&f.app, req).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    assert_eq!(body, b"1,2\n");
}

#[tokio::test]
async fn re_encoding_a_settled_payment_header_is_still_a_replay() {
    let server = facilitator().await;
    Mock::given(method("POST"))
        .and(path("/settle"))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            json!({"success": true, "transaction": format!("sig{}", rand_seed()), "network": NETWORK, "payer": "p"}),
        ))
        .mount(&server)
        .await;
    let f = fixture(&server.uri()).await;
    let header = payment_header(&f).await;
    let (status, _, body) = paid_get(&f, &header).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));

    // Same signed transaction, different JSON bytes (pretty-printed).
    let b64 = base64::engine::general_purpose::STANDARD;
    let payload: Value = serde_json::from_slice(&b64.decode(&header).unwrap()).unwrap();
    let reencoded = b64.encode(serde_json::to_vec_pretty(&payload).unwrap());
    assert_ne!(reencoded, header);
    assert_eq!(paid_get(&f, &reencoded).await.0, StatusCode::CONFLICT);
}

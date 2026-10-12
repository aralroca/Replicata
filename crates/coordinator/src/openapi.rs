//! `GET /openapi.json`: machine-readable description for agents (human version: docs/API.md).

use axum::Json;
use utoipa::OpenApi;

use crate::views::{AuthorView, ClaimView, ExperimentView, HistoryEntry, WorkerView};

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Replicata coordinator",
        version = "1.0.0",
        description = "Verifiable, paid replication of computational experiments on Solana. \
            Paid endpoints use x402 v2 (scheme `exact`, USDC): call without payment to get a 402 with \
            the PAYMENT-REQUIRED header, pay and retry with PAYMENT-SIGNATURE. See docs/API.md."
    ),
    paths(
        config, post_spec, get_spec, list_experiments, get_experiment, upload_bundle, get_worker, get_author,
        get_result, get_dataset, publish
    ),
    components(schemas(ExperimentView, ClaimView, WorkerView, HistoryEntry, AuthorView))
)]
pub struct ApiDoc;

pub async fn serve() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}

// Path declarations only feed utoipa; the real handlers live in `routes`.

#[allow(dead_code)]
#[utoipa::path(get, path = "/v1/config", responses((status = 200, description = "Cluster, network, mint, treasury and faucets")))]
fn config() {}

#[allow(dead_code)]
#[utoipa::path(post, path = "/v1/specs", request_body(content = Object, description = "JobSpec"),
    responses((status = 201, description = "Stored; returns spec_hash"), (status = 400, description = "Invalid spec")))]
fn post_spec() {}

#[allow(dead_code)]
#[utoipa::path(get, path = "/v1/specs/{spec_hash}", params(("spec_hash" = String, Path)),
    responses((status = 200, description = "JobSpec"), (status = 404)))]
fn get_spec() {}

#[allow(dead_code)]
#[utoipa::path(get, path = "/v1/experiments",
    params(("status" = Option<String>, Query), ("phase" = Option<String>, Query), ("author" = Option<String>, Query), ("limit" = Option<usize>, Query)),
    responses((status = 200, body = [ExperimentView])))]
fn list_experiments() {}

#[allow(dead_code)]
#[utoipa::path(get, path = "/v1/experiments/{id}", params(("id" = String, Path)), responses((status = 200, body = ExperimentView), (status = 404)))]
fn get_experiment() {}

#[allow(dead_code)]
#[utoipa::path(post, path = "/v1/experiments/{id}/bundles", params(("id" = String, Path),
        ("X-Worker" = String, Header), ("X-Output-Hash" = String, Header), ("X-Signature" = String, Header)),
    request_body(content = Vec<u8>, content_type = "application/gzip"),
    responses((status = 201), (status = 400), (status = 401), (status = 404), (status = 409), (status = 413)))]
fn upload_bundle() {}

#[allow(dead_code)]
#[utoipa::path(get, path = "/v1/workers/{pubkey}", params(("pubkey" = String, Path)), responses((status = 200, body = WorkerView)))]
fn get_worker() {}

#[allow(dead_code)]
#[utoipa::path(get, path = "/v1/authors/{pubkey}", params(("pubkey" = String, Path)), responses((status = 200, body = AuthorView)))]
fn get_author() {}

#[allow(dead_code)]
#[utoipa::path(get, path = "/v1/experiments/{id}/result", params(("id" = String, Path)),
    responses((status = 200, description = "x402-paid: verified result bundle (application/gzip)"),
        (status = 402, description = "Payment required / insufficient funds"), (status = 409, description = "Not replicated or payment replayed"),
        (status = 503, description = "Facilitator unavailable")))]
fn get_result() {}

#[allow(dead_code)]
#[utoipa::path(get, path = "/v1/experiments/{id}/dataset/{file}", params(("id" = String, Path), ("file" = String, Path)),
    responses((status = 200, description = "x402-paid: one file of the verified bundle"), (status = 402), (status = 404), (status = 409)))]
fn get_dataset() {}

#[allow(dead_code)]
#[utoipa::path(post, path = "/v1/experiments", request_body(content = Object, description = "PublishRequest"),
    responses((status = 200, description = "x402-paid: unsigned create_experiment transaction for the agent to sign"),
        (status = 400), (status = 402)))]
fn publish() {}

pub mod bundles;
pub mod paid;
pub mod public;

use axum::extract::DefaultBodyLimit;
use axum::middleware::{from_fn, from_fn_with_state};
use axum::routing::{get, post};
use axum::Router;
use replicata_client::Pubkey;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use x402_axum::X402Middleware;

use crate::payments;
use crate::state::AppState;

/// CORS must expose the x402 headers so browsers can read them.
fn cors() -> CorsLayer {
    CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any).expose_headers([
        "payment-required".parse().unwrap(),
        "payment-response".parse().unwrap(),
        "x-result-hash".parse().unwrap(),
        "content-disposition".parse().unwrap(),
        "retry-after".parse().unwrap(),
    ])
}

fn paid_routes(state: &AppState) -> Router<AppState> {
    let base = url::Url::parse(&state.cfg.public_url).expect("PUBLIC_URL must be a URL");
    let x402 = X402Middleware::new(&state.cfg.facilitator_url).with_base_url(base).settle_before_execution();
    let shared = state.clone();
    let experiment_price = x402
        .with_dynamic_price(move |_headers, uri, _base| {
            let state = shared.clone();
            let path = uri.path().to_string();
            async move {
                let id = paid::experiment_id(&path).and_then(|id| id.parse::<Pubkey>().ok());
                let price = match id {
                    Some(id) => state.experiment(&id).await.map(|e| (id, e.access_price)),
                    None => None,
                };
                price.map(|(id, amount)| payments::price_tag(&state.cfg, id, amount)).into_iter().collect::<Vec<_>>()
            }
        })
        .with_description("Replicata: verified, replicated experiment result".into())
        .with_mime_type("application/gzip".into());
    let publish_price = x402
        .with_price_tag(payments::price_tag(&state.cfg, state.cfg.treasury, state.cfg.publish_fee))
        .with_description("Replicata: publish an experiment".into());
    // Layers run outermost-last: failure mapping → replay guard → resource guard → x402 → handler.
    let publish = post(paid::publish)
        .layer(publish_price)
        .layer(from_fn(paid::publish_guard))
        .layer(from_fn_with_state(state.clone(), payments::replay_guard))
        .layer(from_fn_with_state(state.clone(), payments::map_failures));

    Router::new()
        .route("/v1/experiments/{id}/result", get(paid::result))
        .route("/v1/experiments/{id}/dataset/{*file}", get(paid::dataset))
        .route_layer(experiment_price)
        .route_layer(from_fn_with_state(state.clone(), paid::experiment_guard))
        .route_layer(from_fn_with_state(state.clone(), payments::replay_guard))
        .route_layer(from_fn_with_state(state.clone(), payments::map_failures))
        .route("/v1/experiments", get(public::list_experiments).merge(publish))
}

pub fn router(state: AppState) -> Router {
    let max_bundle = state.cfg.max_bundle_bytes;
    let free = Router::new()
        .route("/health", get(public::health))
        .route("/version", get(public::version))
        .route("/openapi.json", get(crate::openapi::serve))
        .route("/v1/config", get(public::config))
        .route("/v1/specs", post(public::post_spec))
        .route("/v1/specs/{hash}", get(public::get_spec))
        .route("/v1/experiments/{id}", get(public::get_experiment))
        .route("/v1/experiments/{id}/bundles", post(bundles::upload).layer(DefaultBodyLimit::max(max_bundle)))
        .route("/v1/workers/{pubkey}", get(public::get_worker))
        .route("/v1/authors/{pubkey}", get(public::get_author));

    free.merge(paid_routes(&state)).layer(cors()).layer(TraceLayer::new_for_http()).with_state(state)
}

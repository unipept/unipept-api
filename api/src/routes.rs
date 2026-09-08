use std::time::Duration;

use axum::{
    BoxError, Router, error_handling::HandleErrorLayer, extract::DefaultBodyLimit, http::StatusCode, routing::get
};
use tower::{Layer, ServiceBuilder, timeout::TimeoutLayer};
use tower_http::limit::RequestBodyLimitLayer;

use crate::{
    AppState,
    controllers::{
        api::{
            pept2ec, pept2funct, pept2go, pept2interpro, pept2lca, pept2prot, pept2taxa, peptinfo, protinfo, taxa2lca,
            taxa2tree, taxonomy
        },
        datasets::sampledata,
        mpa::pept2data,
        private_api::{
            ecnumbers, goterms, interpros, metadata, proteins, proteins_filter, reference_proteomes,
            reference_proteomes_filter, taxa, taxa_filter, taxa2rank
        }
    },
    middleware::{
        cors::create_cors_layer,
        normalize_path::{NormalizePath, NormalizePathLayer},
        tracing::create_tracing_layer
    }
};

const REQUEST_TIMEOUT_DURATION: u64 = 150;

/// The routes and their middleware, without the path normalisation `create_app` adds.
///
/// Installing the tracing subscriber used to happen here. It could only ever happen once per
/// process — `.init()` panics on a second call — which made a router something a program could
/// build exactly one of. It belongs to `start`, which runs once by construction.
pub fn create_router(state: AppState) -> Router {
    create_router_with_timeout(state, Duration::from_secs(REQUEST_TIMEOUT_DURATION))
}

/// The status a request that failed inside the middleware stack answers with.
///
/// A downcast rather than a match: `HandleErrorLayer` hands over a `BoxError`, so which error this
/// is can only be asked at run time. Nothing about that is checked when the code compiles, which
/// is why it is a named function with a test rather than a closure inline below — were `Elapsed`
/// to become a different type, a timeout would quietly start answering 500.
pub fn timeout_status(err: BoxError) -> StatusCode {
    if err.is::<tower::timeout::error::Elapsed>() {
        StatusCode::REQUEST_TIMEOUT
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}

/// As [`create_router`], with the timeout given rather than taken from the constant.
///
/// Production has one timeout and does not need this; a test that waits 150 seconds to observe one
/// is not a test anybody runs.
pub fn create_router_with_timeout(state: AppState, timeout: Duration) -> Router {
    Router::new()
        .route("/", get(|| async { "Unipept API server" }))
        .nest("/api", create_api_routes())
        .nest("/datasets", create_datasets_routes())
        .nest("/mpa", create_mpa_routes())
        .nest("/private_api", create_private_api_routes())
        .layer(
            ServiceBuilder::new()
                .layer(HandleErrorLayer::new(|err: BoxError| async move { timeout_status(err) }))
                .layer(TimeoutLayer::new(timeout))
                // Set max request size to 50MiB (default is 2MiB)
                .layer(DefaultBodyLimit::max(50 * 1024 * 1024))
                .layer(RequestBodyLimitLayer::new(50 * 1024 * 1024))
                .layer(create_tracing_layer())
                .layer(create_cors_layer())
        )
        .with_state(state)
}

/// The whole service, exactly as `start` serves it.
///
/// Path normalisation has to wrap the router rather than sit inside it: it rewrites the URI, and
/// `Router::layer` only runs after routing has already used it. That is why this returns a service
/// rather than a `Router` — and why composing it here matters, since a caller that builds only the
/// router is exercising a different stack than the one production runs.
pub fn create_app(state: AppState) -> NormalizePath<Router> {
    NormalizePathLayer::normalize_uris().layer(create_router(state))
}

/// As [`create_app`], with the timeout given rather than taken from the constant.
pub fn create_app_with_timeout(state: AppState, timeout: Duration) -> NormalizePath<Router> {
    NormalizePathLayer::normalize_uris().layer(create_router_with_timeout(state, timeout))
}

/// `/api/v1` is a deprecated alias for `/api/v2`, on purpose.
///
/// v1 is deprecated but stays mounted, because many tools still call it. It no longer serves
/// what it originally did: v1 taxonomy differed slightly from v2, and that difference is gone.
/// A v1 caller now gets v2 semantics, since there is no `create_api_v1_routes` and
/// `LineageVersion` has no `V1` variant.
///
/// The two prefixes therefore change together, and `the_two_api_versions_answer_identically`
/// in `tests/endpoints/routing.rs` fails if one is given routes the other does not have.
fn create_api_routes() -> Router<AppState> {
    Router::new().nest("/v1", create_api_v2_routes()).nest("/v2", create_api_v2_routes())
}

macro_rules! define_routes {
    (
        $( $path:tt, $handlers:expr_2021),*
    ) => {{
        let mut router = Router::new();

        $(
            router = router
                .route($path, $handlers)
                .route(concat!($path, ".json"), $handlers);
        )*

        router
    }};
}

fn create_api_v2_routes() -> Router<AppState> {
    define_routes!(
        "/pept2ec",
        get(pept2ec::get_json_handler).post(pept2ec::post_json_handler),
        "/pept2funct",
        get(pept2funct::get_json_handler).post(pept2funct::post_json_handler),
        "/pept2go",
        get(pept2go::get_json_handler).post(pept2go::post_json_handler),
        "/pept2interpro",
        get(pept2interpro::get_json_handler).post(pept2interpro::post_json_handler),
        "/pept2lca",
        get(pept2lca::get_json_handler_v2).post(pept2lca::post_json_handler_v2),
        "/pept2prot",
        get(pept2prot::get_json_handler).post(pept2prot::post_json_handler),
        "/pept2taxa",
        get(pept2taxa::get_json_handler_v2).post(pept2taxa::post_json_handler_v2),
        "/peptinfo",
        get(peptinfo::get_json_handler_v2).post(peptinfo::post_json_handler_v2),
        "/protinfo",
        get(protinfo::get_json_handler_v2).post(protinfo::post_json_handler_v2),
        "/taxa2lca",
        get(taxa2lca::get_json_handler_v2).post(taxa2lca::post_json_handler_v2),
        "/taxa2tree",
        get(taxa2tree::get_json_handler_v2).post(taxa2tree::post_json_handler_v2),
        "/taxonomy",
        get(taxonomy::get_json_handler_v2).post(taxonomy::post_json_handler_v2)
    )
    .route("/taxa2tree.html", get(taxa2tree::get_html_handler_v2).post(taxa2tree::post_html_handler_v2))
}

fn create_datasets_routes() -> Router<AppState> {
    define_routes!("/sampledata", get(sampledata::get_json_handler).post(sampledata::post_json_handler))
}

fn create_mpa_routes() -> Router<AppState> {
    define_routes!("/pept2data", get(pept2data::get_json_handler).post(pept2data::post_json_handler))
}

fn create_private_api_routes() -> Router<AppState> {
    define_routes!(
        "/ecnumbers",
        get(ecnumbers::get_json_handler).post(ecnumbers::post_json_handler),
        "/goterms",
        get(goterms::get_json_handler).post(goterms::post_json_handler),
        "/interpros",
        get(interpros::get_json_handler).post(interpros::post_json_handler),
        "/metadata",
        get(metadata::get_json_handler).post(metadata::post_json_handler),
        "/proteins",
        get(proteins::get_json_handler).post(proteins::post_json_handler),
        "/proteins/count",
        get(proteins_filter::get_json_count_handler).post(proteins_filter::post_json_count_handler),
        "/proteins/filter",
        get(proteins_filter::get_json_filter_handler).post(proteins_filter::post_json_filter_handler),
        "/proteomes",
        get(reference_proteomes::get_json_handler).post(reference_proteomes::post_json_handler),
        "/proteomes/count",
        get(reference_proteomes_filter::get_json_count_handler)
            .post(reference_proteomes_filter::post_json_count_handler),
        "/proteomes/filter",
        get(reference_proteomes_filter::get_json_filter_handler)
            .post(reference_proteomes_filter::post_json_filter_handler),
        "/taxa",
        get(taxa::get_json_handler).post(taxa::post_json_handler),
        "/taxa2rank",
        get(taxa2rank::get_json_handler).post(taxa2rank::post_json_handler),
        "/taxa/count",
        get(taxa_filter::get_json_count_handler).post(taxa_filter::post_json_count_handler),
        "/taxa/filter",
        get(taxa_filter::get_json_filter_handler).post(taxa_filter::post_json_filter_handler)
    )
}

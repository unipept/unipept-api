use std::time::Duration;

use axum::{http::{header::{CONTENT_TYPE, ETAG, IF_MODIFIED_SINCE, IF_NONE_MATCH}, Method}, routing::get, Router};
use tower_http::cors::{Any, CorsLayer};

use crate::{controllers::{api::{pept2ec, pept2funct, pept2go, pept2interpro, pept2lca, pept2prot, pept2taxa, peptinfo, protinfo, taxa2lca, taxa2tree, taxonomy}, datasets::sampledata, mpa::{pept2data, pept2filtered}, private_api::{ecnumbers, goterms, interpros, metadata, proteins, taxa}}, AppState};

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/", get(|| async { "Unipept API server" }))
        .nest("/api", create_api_routes())
        .nest("/datasets", create_datasets_routes())
        .nest("/mpa", create_mpa_routes())
        .nest("/private_api", create_private_api_routes())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .expose_headers([ETAG])
                .allow_methods([Method::GET, Method::POST])
                .allow_headers([CONTENT_TYPE, IF_MODIFIED_SINCE, IF_NONE_MATCH])
                .max_age(Duration::from_secs(86400))
        )
        .with_state(state)
}

fn create_api_routes() -> Router<AppState> {
    Router::new()
        .nest("/v1", create_api_v1_routes())
        .nest("/v2", create_api_v2_routes())
}

fn create_api_v1_routes() -> Router<AppState> {
    Router::new()
        .route("/pept2ec", get(pept2ec::get_handler).post(pept2ec::post_handler))
        .route("/pept2funct", get(pept2funct::get_handler).post(pept2funct::post_handler))
        .route("/pept2go", get(pept2go::get_handler).post(pept2go::post_handler))
        .route("/pept2interpro", get(pept2interpro::get_handler).post(pept2interpro::post_handler))
        .route("/pept2lca", get(pept2lca::get_handler_v1).post(pept2lca::post_handler_v1))
        .route("/pept2prot", get(pept2prot::get_handler).post(pept2prot::post_handler))
        .route("/pept2taxa", get(pept2taxa::get_handler_v1).post(pept2taxa::post_handler_v1))
        .route("/peptinfo", get(peptinfo::get_handler_v1).post(peptinfo::post_handler_v1))
        .route("/protinfo", get(protinfo::get_handler_v1).post(protinfo::post_handler_v1))
        .route("/taxa2lca", get(taxa2lca::get_handler_v1).post(taxa2lca::post_handler_v1))
        .route("/taxa2tree", get(taxa2tree::handler).post(taxa2tree::handler)) // TODO: implement
        .route("/taxonomy", get(taxonomy::get_handler_v1).post(taxonomy::post_handler_v1))
}

fn create_api_v2_routes() -> Router<AppState> {
    Router::new()
        .route("/pept2ec", get(pept2ec::get_handler).post(pept2ec::post_handler))
        .route("/pept2funct", get(pept2funct::get_handler).post(pept2funct::post_handler))
        .route("/pept2go", get(pept2go::get_handler).post(pept2go::post_handler))
        .route("/pept2interpro", get(pept2interpro::get_handler).post(pept2interpro::post_handler))
        .route("/pept2lca", get(pept2lca::get_handler_v2).post(pept2lca::post_handler_v2))
        .route("/pept2prot", get(pept2prot::get_handler).post(pept2prot::post_handler))
        .route("/pept2taxa", get(pept2taxa::get_handler_v2).post(pept2taxa::post_handler_v2))
        .route("/peptinfo", get(peptinfo::get_handler_v2).post(peptinfo::post_handler_v2))
        .route("/protinfo", get(protinfo::get_handler_v2).post(protinfo::post_handler_v2))
        .route("/taxa2lca", get(taxa2lca::get_handler_v2).post(taxa2lca::post_handler_v2))
        .route("/taxa2tree", get(taxa2tree::handler).post(taxa2tree::handler)) // TODO: implement
        .route("/taxonomy", get(taxonomy::get_handler_v2).post(taxonomy::post_handler_v2))
}

fn create_datasets_routes() -> Router<AppState> {
    Router::new()
        .route("/sampledata", get(sampledata::get_handler).post(sampledata::post_handler))
}

fn create_mpa_routes() -> Router<AppState> {
    Router::new()
        .route("/pept2data", get(pept2data::get_handler).post(pept2data::post_handler))
        .route("/pept2filtered", get(pept2filtered::get_handler).post(pept2filtered::post_handler))
}

fn create_private_api_routes() -> Router<AppState> {
    Router::new()
        .route("/ecnumbers", get(ecnumbers::get_handler).post(ecnumbers::post_handler))
        .route("/goterms", get(goterms::get_handler).post(goterms::post_handler))
        .route("/interpros", get(interpros::get_handler).post(interpros::post_handler))
        .route("/metadata", get(metadata::get_handler).post(metadata::post_handler))
        .route("/proteins", get(proteins::get_handler).post(proteins::post_handler))
        .route("/taxa", get(taxa::get_handler).post(taxa::post_handler))
}

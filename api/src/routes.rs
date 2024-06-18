use axum::{routing::get, Router};

use crate::{controllers::{api::{pept2ec, pept2funct, pept2go, pept2interpro, pept2lca, pept2prot, pept2taxa, peptinfo}, datasets::sampledata, mpa::{pept2data, pept2filtered}, private_api::{ecnumbers, goterms, interpros, metadata, proteins, taxa}}, AppState};

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/", get(|| async { "Unipept API server" }))
        .nest("/api", create_api_routes())
        .nest("/datasets", create_datasets_routes())
        .nest("/mpa", create_mpa_routes())
        .nest("/private_api", create_private_api_routes())
        .with_state(state)
}

fn create_api_routes() -> Router<AppState> {
    Router::new()
        .nest("/v1", create_api_v1_routes())
        .nest("/v2", create_api_v2_routes())
}

fn create_api_v1_routes() -> Router<AppState> {
    Router::new()
        .route("/pept2ec", get(pept2ec::handler).post(pept2ec::handler))
        .route("/pept2funct", get(pept2funct::handler).post(pept2funct::handler))
        .route("/pept2go", get(pept2go::handler).post(pept2go::handler))
        .route("/pept2interpro", get(pept2interpro::handler).post(pept2interpro::handler))
        .route("/pept2lca", get(pept2lca::handler_v1).post(pept2lca::handler_v1))
        .route("/pept2prot", get(pept2prot::handler).post(pept2prot::handler))
        .route("/pept2taxa", get(pept2taxa::handler_v1).post(pept2taxa::handler_v1))
        .route("/peptinfo", get(peptinfo::handler_v1).post(peptinfo::handler_v1))
}

fn create_api_v2_routes() -> Router<AppState> {
    Router::new()
        .route("/pept2ec", get(pept2ec::handler).post(pept2ec::handler))
        .route("/pept2funct", get(pept2funct::handler).post(pept2funct::handler))
        .route("/pept2go", get(pept2go::handler).post(pept2go::handler))
        .route("/pept2interpro", get(pept2interpro::handler).post(pept2interpro::handler))
        .route("/pept2lca", get(pept2lca::handler_v2).post(pept2lca::handler_v2))
        .route("/pept2prot", get(pept2prot::handler).post(pept2prot::handler))
        .route("/pept2taxa", get(pept2taxa::handler_v2).post(pept2taxa::handler_v2))
        .route("/peptinfo", get(peptinfo::handler_v2).post(peptinfo::handler_v2))
}

fn create_datasets_routes() -> Router<AppState> {
    Router::new()
        .route("/sampledata", get(sampledata::handler))
}

fn create_mpa_routes() -> Router<AppState> {
    Router::new()
        .route("/pept2data", get(pept2data::handler).post(pept2data::handler))
        .route("/pept2filtered", get(pept2filtered::handler).post(pept2filtered::handler))
}

fn create_private_api_routes() -> Router<AppState> {
    Router::new()
        .route("/ecnumbers", get(ecnumbers::handler).post(ecnumbers::handler))
        .route("/goterms", get(goterms::handler).post(goterms::handler))
        .route("/interpros", get(interpros::handler).post(interpros::handler))
        .route("/metadata", get(metadata::handler).post(metadata::handler))
        .route("/proteins", get(proteins::handler).post(proteins::handler))
        .route("/taxa", get(taxa::handler).post(taxa::handler))
}

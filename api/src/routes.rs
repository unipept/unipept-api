use axum::{routing::{get, post}, Router};

use crate::{controllers::{datasets::sampledata, mpa::{pept2data, pept2filtered}, private_api::{ecnumbers, goterms, interpros, metadata, proteins, taxa}}, AppState};

pub fn create_routes(state: AppState) -> Router {
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
}

fn create_api_v2_routes() -> Router<AppState> {
    Router::new()
}

fn create_datasets_routes() -> Router<AppState> {
    Router::new()
        .route("/sampledata", get(sampledata::handler))
}

fn create_mpa_routes() -> Router<AppState> {
    Router::new()
        .route("/pept2data", get(pept2data::handler))
        .route("/pept2filtered", get(pept2filtered::handler))
}

fn create_private_api_routes() -> Router<AppState> {
    Router::new()
        .route("/ecnumbers", post(ecnumbers::handler))
        .route("/goterms", post(goterms::handler))
        .route("/interpros", get(interpros::handler))
        .route("/metadata", get(metadata::handler))
        .route("/proteins", get(proteins::handler))
        .route("/taxa", get(taxa::handler))
}

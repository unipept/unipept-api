use axum::{routing::get, Router};
use datastore::sampledata::SampleData;

use crate::{controllers::{datasets::sampledata, mpa::{pept2data, pept2filtered}, private_api::{ecnumbers, goterms, interpros, metadata, proteins, taxa}}, AppState};

pub fn create_routes(state: AppState) -> Router {
    Router::new()
        .route("/", get(|| async { "Unipept API server" }))
        .nest("/api", create_api_routes())
        .nest("/datasets", create_datasets_routes(state.sample_data.clone()))
        .nest("/mpa", create_mpa_routes())
        .nest("/private_api", create_private_api_routes())
}

fn create_api_routes() -> Router {
    Router::new()
        .nest("/v1", create_api_v1_routes())
        .nest("/v2", create_api_v2_routes())
}

fn create_api_v1_routes() -> Router {
    Router::new()
}

fn create_api_v2_routes() -> Router {
    Router::new()
}

fn create_datasets_routes(sample_data: SampleData) -> Router {
    Router::new()
        .route("/sampledata", get(sampledata::handler))
        .with_state(sample_data)
}

fn create_mpa_routes() -> Router {
    Router::new()
        .route("/pept2data", get(pept2data::handler))
        .route("/pept2filtered", get(pept2filtered::handler))
}

fn create_private_api_routes() -> Router {
    Router::new()
        .route("/ecnumbers", get(ecnumbers::handler))
        .route("/goterms", get(goterms::handler))
        .route("/interpros", get(interpros::handler))
        .route("/metadata", get(metadata::handler))
        .route("/proteins", get(proteins::handler))
        .route("/taxa", get(taxa::handler))
}

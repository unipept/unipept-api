use axum::{routing::get, Router};

pub fn create_routes() -> Router {
    Router::new()
        .route("/", get(|| async { "Unipept API server" }))
        .nest("/api", create_api_routes())
        .nest("/datasets", create_datasets_routes())
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

fn create_datasets_routes() -> Router {
    Router::new()
}

fn create_mpa_routes() -> Router {
    Router::new()
}

fn create_private_api_routes() -> Router {
    Router::new()
}

use std::time::Duration;

use axum::http::{header::{CONTENT_TYPE, ETAG, IF_MODIFIED_SINCE, IF_NONE_MATCH}, Method};
use tower_http::cors::{Any, CorsLayer};

pub fn create_cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(Any)
        .expose_headers([ETAG])
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([CONTENT_TYPE, IF_MODIFIED_SINCE, IF_NONE_MATCH])
        .max_age(Duration::from_secs(86400))
}

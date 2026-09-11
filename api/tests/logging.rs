//! What the service writes down about a request it never routed.
//!
//! Its own test binary, deliberately. `tracing` keeps one process-wide maximum level and caches a
//! callsite's interest the first time it is reached; a binary whose other tests install no
//! subscriber holds that maximum at `OFF`, and the macros then compile out the very events these
//! tests read back. Every test here installs a subscriber, so the level stays raised.

#[path = "common.rs"]
mod common;

use std::time::Duration;

use axum::{
    body::Body,
    http::{Request, header}
};
use unipept_api::routes::{create_app, create_app_with_timeout};

use crate::common::BODY_LIMIT;

/// A request refused for its declared size is logged.
///
/// `RequestBodyLimitLayer` answers without the request ever reaching the router, so this is logged
/// only while the tracing layer sits above it.
#[tokio::test]
async fn a_body_refused_for_its_declared_size_is_logged() {
    let request = Request::post("/api/v2/pept2lca")
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .header(header::CONTENT_LENGTH, (BODY_LIMIT + 1).to_string())
        .body(Body::from("junk=A"))
        .unwrap();

    let logged = common::log_of(create_app, request).await;

    assert!(logged.contains("status=413"), "the refusal was not logged: {logged}");
    assert!(logged.contains(r#"route="/api/v2/pept2lca""#), "the route is missing: {logged}");
}

/// A timed-out request is logged, with the latency that explains it and the id that places it.
///
/// The 408 comes from `HandleErrorLayer`, above the router, and is the failure most worth tracing
/// back to HAProxy's own log.
#[tokio::test]
async fn a_timed_out_request_is_logged() {
    let stalled = Body::from_stream(futures_util::stream::pending::<Result<bytes::Bytes, std::io::Error>>());
    let request = Request::post("/api/v2/pept2lca")
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .header("x-request-id", "haproxy-set-this-id")
        .body(stalled)
        .unwrap();

    let logged = common::log_of(|state| create_app_with_timeout(state, Duration::from_millis(50)), request).await;

    assert!(logged.contains("status=408"), "the timeout was not logged: {logged}");
    assert!(logged.contains(r#"request_id="haproxy-set-this-id""#), "the request id is missing: {logged}");
}

/// A served request is logged at INFO, with the route it matched.
///
/// `/health` rather than a search endpoint: the search handlers call `block_in_place`, which needs
/// the multi-threaded runtime that [`common::log_of`] cannot use.
#[tokio::test]
async fn a_served_request_is_logged() {
    let request = Request::get("/health").body(Body::empty()).unwrap();

    let logged = common::log_of(create_app, request).await;

    assert!(logged.contains("status=200"), "the request was not logged: {logged}");
    assert!(logged.contains(r#"route="/health""#), "the route is missing: {logged}");
}

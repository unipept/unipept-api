//! The router itself: that it can be built, built again, and driven end to end.
//!
//! Every test here uses `flavor = "multi_thread"`. Nine controllers call `block_in_place`, which
//! panics on the current-thread runtime `#[tokio::test]` gives you by default, and the failure is
//! opaque enough to be worth stating once rather than rediscovering.

mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode}
};
use http_body_util::BodyExt;
use tower::ServiceExt;
use unipept_api::routes::create_app;

async fn get(path: &str) -> (StatusCode, String) {
    let (_dir, state) = common::offline_state();
    let response = create_app(state)
        .oneshot(Request::get(path).body(Body::empty()).unwrap())
        .await
        .expect("the app responds");

    let status = response.status();
    let bytes = response.into_body().collect().await.expect("a body").to_bytes();
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

/// Building a router used to install a process-global tracing subscriber, so the second one
/// panicked. Nothing could hold two, which meant no test suite could hold two either.
#[tokio::test(flavor = "multi_thread")]
async fn a_router_can_be_built_more_than_once() {
    let (_first_dir, first) = common::offline_state();
    let (_second_dir, second) = common::offline_state();

    let _first = create_app(first);
    let _second = create_app(second);
}

/// One route, driven through the whole stack with no listener and no port.
#[tokio::test(flavor = "multi_thread")]
async fn the_root_route_answers_through_the_whole_stack() {
    let (status, body) = get("/").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "Unipept API server");
}

#[tokio::test(flavor = "multi_thread")]
async fn an_unknown_route_is_a_not_found() {
    assert_eq!(get("/no-such-endpoint").await.0, StatusCode::NOT_FOUND);
}

/// Path normalisation is the reason `create_app` exists.
///
/// It rewrites the URI before routing, so it cannot live inside the `Router` — and a test that
/// built only the router would exercise a stack production does not serve, silently. These paths
/// are 404s without it.
#[tokio::test(flavor = "multi_thread")]
async fn trailing_and_repeated_slashes_are_normalised() {
    for path in ["/api/v2/pept2lca", "/api/v2/pept2lca/", "//api//v2//pept2lca", "/api/v2/pept2lca///"] {
        assert_eq!(get(path).await.0, StatusCode::OK, "{path} should reach the handler");
    }
}

/// A real endpoint, reached through the router with the corpus behind it.
///
/// `pept2lca` takes the lightweight taxa path and touches the index and the datastore but never
/// the database, so it answers with nothing listening on the OpenSearch address.
#[tokio::test(flavor = "multi_thread")]
async fn an_endpoint_answers_with_the_corpus_behind_it() {
    let (status, body) = get("/api/v2/pept2lca?input[]=AWDIQNGK").await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("AWDIQNGK"), "the peptide should be echoed back: {body}");
    assert!(body.contains("Crocodylus niloticus"), "its LCA should be named: {body}");
}

//! `/health` and `/health/database`. Both GET only, no `.json` twin.

use axum::{
    body::Body,
    http::{Request, StatusCode}
};
use httpmock::MockServer;

use crate::common::{SEARCH, offline_state, request_raw, test_state};

#[tokio::test(flavor = "multi_thread")]
async fn it_answers_empty_whenever_the_process_is_serving() {
    let (dir, state) = offline_state();
    let (status, body) = request_raw(state, Request::get("/health").body(Body::empty()).unwrap()).await;
    drop(dir);

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_post_is_not_allowed() {
    let (dir, state) = offline_state();
    let (status, _) = request_raw(state, Request::post("/health").body(Body::empty()).unwrap()).await;
    drop(dir);

    assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test(flavor = "multi_thread")]
async fn database_health_is_unavailable_when_opensearch_is_unreachable() {
    let (dir, state) = offline_state();
    let (status, body) = request_raw(state, Request::get("/health/database").body(Body::empty()).unwrap()).await;
    drop(dir);

    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body, "");
}

#[tokio::test(flavor = "multi_thread")]
async fn database_health_is_ok_when_the_index_of_the_version_answers() {
    let server = MockServer::start_async().await;
    server
        .mock_async(|when, then| {
            when.path(SEARCH).query_param("size", "0").query_param("terminate_after", "1");
            then.status(200).json_body(serde_json::json!({ "hits": { "hits": [] } }));
        })
        .await;

    let (dir, state) = test_state(&server.base_url());
    let (status, body) = request_raw(state, Request::get("/health/database").body(Body::empty()).unwrap()).await;
    drop(dir);

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "");
}

/// A cluster that is up but lacks the index, as on a host whose release was never loaded: every
/// protein query would fail, so the server must not be routed database traffic.
#[tokio::test(flavor = "multi_thread")]
async fn database_health_is_unavailable_when_the_index_of_the_version_is_missing() {
    let server = MockServer::start_async().await;
    server
        .mock_async(|when, then| {
            when.path("/");
            then.status(200);
        })
        .await;
    server
        .mock_async(|when, then| {
            when.path(SEARCH);
            then.status(404).json_body(serde_json::json!({ "error": { "type": "index_not_found_exception" } }));
        })
        .await;

    let (dir, state) = test_state(&server.base_url());
    let (status, body) = request_raw(state, Request::get("/health/database").body(Body::empty()).unwrap()).await;
    drop(dir);

    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body, "");
}

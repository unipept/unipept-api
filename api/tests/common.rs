// This module is compiled into every integration-test binary that declares it, and no single one
// uses all of it — the router tests need only the state, the endpoint suites need the helpers too.
#![allow(dead_code)]

//! One `AppState` over the shared corpus, for the endpoint tests to drive.
//!
//! Index, datastore and database all come from the same corpus, so a result means something:
//! built from separate fixtures every endpoint would answer with nothing and every test would
//! still pass.
//!
//! Endpoint tests need `#[tokio::test(flavor = "multi_thread")]` — the handlers call
//! `block_in_place`, which panics on the default current-thread runtime.

use std::sync::Arc;

use axum::{
    body::Body,
    http::{Request, StatusCode}
};
use http_body_util::BodyExt;
use tempfile::TempDir;
use tower::ServiceExt;
use unipept_api::{AppState, routes::create_app};

/// Builds the corpus into a temporary directory and returns state over it.
///
/// The `TempDir` comes back because dropping it deletes the files, and a memory-mapped index reads
/// them for as long as it is alive. `database_url` is only parsed, never connected to.
pub fn test_state(database_url: &str) -> (TempDir, AppState) {
    let dir = TempDir::new().expect("could not create a temporary directory");

    let paths = fixtures::write_datastore_files(dir.path());
    let datastore = datastore::DataStore::try_from_files(
        &paths.version.to_string_lossy(),
        &paths.sampledata.to_string_lossy(),
        &paths.ec_numbers.to_string_lossy(),
        &paths.go_terms.to_string_lossy(),
        &paths.interpro_entries.to_string_lossy(),
        &paths.proteomes.to_string_lossy(),
        &paths.lineages.to_string_lossy(),
        &paths.taxons.to_string_lossy()
    )
    .expect("the corpus datastore should load");

    let index_paths = fixtures::build_index_files(dir.path());
    let index = index::Index::try_from_files(
        &index_paths.suffix_array.to_string_lossy(),
        &index_paths.proteins.to_string_lossy(),
        &index_paths.mapping.to_string_lossy(),
        &index_paths.kmer_table.to_string_lossy()
    )
    .expect("the corpus index should load");

    let database = database::Database::try_from_url(database_url).expect("the url should parse");

    (dir, AppState {
        datastore: Arc::new(datastore),
        database: Arc::new(database),
        index: Arc::new(index)
    })
}

/// A state pointed at an address nothing listens on, for endpoints that never reach OpenSearch.
pub fn offline_state() -> (TempDir, AppState) {
    test_state("http://127.0.0.1:1")
}

/// Drives one request through the whole stack — the same one `start` serves — and parses the body.
pub async fn request_json(state: unipept_api::AppState, request: Request<Body>) -> (StatusCode, serde_json::Value) {
    let (status, body) = request_raw(state, request).await;
    let json = serde_json::from_str(&body).unwrap_or_else(|err| panic!("body was not JSON ({err}): {body}"));
    (status, json)
}

/// The status, the content type and the body of one request, for asserting on a failure.
pub async fn request_parts(state: unipept_api::AppState, request: Request<Body>) -> (StatusCode, String, String) {
    let response = create_app(state).oneshot(request).await.expect("the app responds");
    let status = response.status();
    let content_type = response
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .map(|value| value.to_str().unwrap_or_default().to_string())
        .unwrap_or_default();
    let bytes = response.into_body().collect().await.expect("a body").to_bytes();

    (status, content_type, String::from_utf8_lossy(&bytes).into_owned())
}

/// As [`request_json`], but leaves the body untouched: HTML routes, and plain-text rejections.
pub async fn request_raw(state: unipept_api::AppState, request: Request<Body>) -> (StatusCode, String) {
    let response = create_app(state).oneshot(request).await.expect("the app responds");
    let status = response.status();
    let bytes = response.into_body().collect().await.expect("a body").to_bytes();
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

/// A GET against an offline state, for the endpoints that never reach OpenSearch.
///
/// `dir` is dropped explicitly rather than at end of scope; `corpus_files_outlive_the_request`
/// fails if that stops happening.
pub async fn get_json(path: &str) -> (StatusCode, serde_json::Value) {
    let (dir, state) = offline_state();
    let answered = request_json(state, Request::get(path).body(Body::empty()).unwrap()).await;
    drop(dir);
    answered
}

/// The same request as a JSON POST, so both halves of every route are exercised.
pub async fn post_json(path: &str, body: serde_json::Value) -> (StatusCode, serde_json::Value) {
    let (dir, state) = offline_state();
    let request = Request::post(path)
        .header(axum::http::header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let answered = request_json(state, request).await;
    drop(dir);
    answered
}

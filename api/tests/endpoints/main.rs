//! Endpoint tests, laid out as `src/controllers/` is — a handler's tests sit at the matching path.
//!
//! Each module covers its own endpoint across the parameter combinations that change the answer;
//! `routing` holds what is true of every route. Tests need `flavor = "multi_thread"`, since the
//! handlers call `block_in_place`.

#[path = "../common.rs"]
mod common;

mod api;
mod database;
mod datasets;
mod private_api;
mod routing;

// ── the suite's own scaffolding ──────────────────────────────────────────────────────────────────

/// The corpus files must outlive the request: a memory-mapped index reads them until it is
/// dropped. Fails if a helper stops holding its `TempDir`.
#[tokio::test(flavor = "multi_thread")]
async fn corpus_files_outlive_the_request() {
    use axum::{
        body::Body,
        http::{Request, StatusCode}
    };

    use crate::common::{offline_state, request_json};

    let (dir, state) = offline_state();
    let path = dir.path().to_path_buf();

    let (status, _) =
        request_json(state, Request::get("/api/v2/taxa2lca?input[]=8501").body(Body::empty()).unwrap()).await;

    assert_eq!(status, StatusCode::OK);
    assert!(path.join("sa.bin").exists(), "the index files were deleted before the request finished");
    assert!(path.join("taxons.tsv").exists(), "the datastore files were deleted before the request finished");

    drop(dir);
    assert!(!path.exists(), "and they are cleaned up once the guard is dropped");
}

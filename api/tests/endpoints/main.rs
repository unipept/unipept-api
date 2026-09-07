//! The endpoints that read only the datastore.
//!
//! Laid out as `src/controllers/` is, so an endpoint's tests sit at the path its handler does: a
//! change to `controllers/private_api/taxa.rs` has its tests at `endpoints/private_api/taxa.rs`.
//!
//! A controller's parameters are its contract, and most take several, so each module covers its own
//! endpoint across the combinations that change the answer rather than sampling one call. A flag
//! whose two values are never both exercised is a flag no test distinguishes from a constant.
//!
//! `routing` holds what is true of every route at once and belongs to no single controller.
//!
//! Every test runs on a multi-threaded runtime: nine controllers call `tokio::task::block_in_place`,
//! which panics on the current-thread runtime `#[tokio::test]` builds by default.

#[path = "../common.rs"]
mod common;

mod api;
mod datasets;
mod private_api;
mod routing;

// ── the suite's own scaffolding ──────────────────────────────────────────────────────────────────

/// The corpus files must still be on disk while a request is being served.
///
/// A memory-mapped index reads them for as long as it is alive, so a `TempDir` dropped too early
/// would delete an index mid-search. Asserting the directory still exists *after* the await is what
/// catches a helper that stopped holding it.
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

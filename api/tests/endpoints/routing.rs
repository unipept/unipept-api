//! What is true of every route rather than of any one controller.

use axum::http::StatusCode;
use serde_json::json;

use crate::common::{get_json, post_json};

/// Every route is registered for GET and POST through one macro, and the two reach the same
/// handler by different extractors.
#[tokio::test(flavor = "multi_thread")]
async fn get_and_post_answer_alike() {
    let (get_status, from_get) = get_json("/private_api/taxa?taxids[]=8501").await;
    let (post_status, from_post) = post_json("/private_api/taxa", json!({ "taxids": [8501] })).await;

    assert_eq!(get_status, StatusCode::OK);
    assert_eq!(post_status, StatusCode::OK);
    assert_eq!(from_get, from_post);
}

/// The same macro registers a `.json` suffix beside every path.
#[tokio::test(flavor = "multi_thread")]
async fn the_json_suffix_reaches_the_same_handler() {
    for (plain, suffixed) in [
        ("/private_api/metadata", "/private_api/metadata.json"),
        ("/api/v2/taxonomy?input[]=8501", "/api/v2/taxonomy.json?input[]=8501"),
        ("/datasets/sampledata", "/datasets/sampledata.json")
    ] {
        let (plain_status, from_plain) = get_json(plain).await;
        let (suffixed_status, from_suffixed) = get_json(suffixed).await;

        assert_eq!(plain_status, suffixed_status, "{plain}");
        assert_eq!(from_plain, from_suffixed, "{plain}");
    }
}

/// `/api/v1` and `/api/v2` mount the same router — see issue #152. Asserted so that separating
/// them becomes a deliberate change rather than a silent one.
#[tokio::test(flavor = "multi_thread")]
async fn the_two_api_versions_answer_identically_issue_152() {
    let (v1_status, v1) = get_json("/api/v1/taxonomy?input[]=8501").await;
    let (v2_status, v2) = get_json("/api/v2/taxonomy?input[]=8501").await;

    assert_eq!(v1_status, v2_status);
    assert_eq!(v1, v2, "issue #152: v1 is an alias for v2, and nothing else records that");
}

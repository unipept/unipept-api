//! `/private_api/metadata` — the index version. Takes no parameters at all.

use axum::http::StatusCode;

use crate::common::{get_json, post_json};

#[tokio::test(flavor = "multi_thread")]
async fn it_reports_the_index_version() {
    let (status, body) = get_json("/private_api/metadata").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["db_version"], "2026.09-fixtures");
}

/// An endpoint taking no parameters still has to accept a body on its POST half.
#[tokio::test(flavor = "multi_thread")]
async fn a_post_answers_like_a_get() {
    let (_, from_get) = get_json("/private_api/metadata").await;
    let (status, from_post) = post_json("/private_api/metadata", serde_json::json!({})).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(from_post, from_get);
}

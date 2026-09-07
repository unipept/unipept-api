//! `/datasets/sampledata` — the configured sample datasets. Takes no parameters.

use axum::http::StatusCode;

use crate::common::get_json;

#[tokio::test(flavor = "multi_thread")]
async fn it_returns_the_configured_datasets() {
    let (status, body) = get_json("/datasets/sampledata").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["sample_data"][0]["environment"], "Fixture environment");
    assert_eq!(body["sample_data"][0]["datasets"][0]["name"], "Crocodylus peptides");
}

/// The datasets carry the peptides the rest of the corpus is built around, so a sample document
/// that drifted from the proteins would show up here.
#[tokio::test(flavor = "multi_thread")]
async fn the_datasets_name_peptides_the_corpus_contains() {
    let (_, body) = get_json("/datasets/sampledata").await;

    assert_eq!(body["sample_data"][0]["datasets"][0]["data"][0], fixtures::peptides::UNIQUE);
}

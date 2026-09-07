//! `/private_api/proteomes` — reference proteome accessions to their taxon and protein count.

use axum::http::StatusCode;
use serde_json::json;

use crate::common::{get_json, post_json};

#[tokio::test(flavor = "multi_thread")]
async fn one_accession_resolves_to_its_taxon_and_count() {
    let (status, body) = get_json("/private_api/proteomes?proteomes[]=UP000000001").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body[0]["id"], "UP000000001");
    assert_eq!(body[0]["taxon_id"], 8501);
    assert_eq!(body[0]["taxon_name"], "Crocodylus niloticus", "named through the taxon store");
    assert_eq!(body[0]["protein_count"], 3);
}

#[tokio::test(flavor = "multi_thread")]
async fn several_accessions_resolve_in_one_call() {
    let (status, body) = get_json("/private_api/proteomes?proteomes[]=UP000000001&proteomes[]=UP000000002").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().map(Vec::len), Some(2));
}

#[tokio::test(flavor = "multi_thread")]
async fn an_unknown_accession_is_omitted_rather_than_failing() {
    let (status, body) = get_json("/private_api/proteomes?proteomes[]=UP999999999&proteomes[]=UP000000001").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().map(Vec::len), Some(1));
}

#[tokio::test(flavor = "multi_thread")]
async fn no_accessions_is_an_empty_answer() {
    assert_eq!(get_json("/private_api/proteomes").await.1, json!([]));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_post_answers_like_a_get() {
    let (_, from_get) = get_json("/private_api/proteomes?proteomes[]=UP000000001").await;
    let (status, from_post) = post_json("/private_api/proteomes", json!({ "proteomes": ["UP000000001"] })).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(from_post, from_get);
}

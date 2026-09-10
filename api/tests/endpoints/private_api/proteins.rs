//! `/private_api/proteins` — accessions to protein records, straight out of the cluster.

use axum::{
    body::Body,
    http::{Request, StatusCode}
};
use httpmock::{Method::POST, MockServer};
use serde_json::json;

use crate::{
    common::{request_raw, test_state},
    database::{get_against, source}
};

#[tokio::test(flavor = "multi_thread")]
async fn proteins_returns_what_the_cluster_holds() {
    let server = MockServer::start_async().await;
    server
        .mock_async(|when, then| {
            when.method(POST).path("/uniprot_entries/_mget");
            then.status(200)
                .json_body(json!({ "docs": [ { "_source": source("P00001", 8501, "Protein one", "") } ] }));
        })
        .await;

    let (status, body) = get_against(&server, "/private_api/proteins?accessions[]=P00001").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body[0]["uniprot_accession_id"], "P00001");
    assert_eq!(body[0]["name"], "Protein one");
    assert_eq!(body[0]["taxon_id"], 8501);
    assert_eq!(body[0]["db_type"], "swissprot");
}

/// An accession the cluster does not hold comes back as a `docs` entry with no `_source`. It is
/// dropped rather than failing the batch, so one unknown accession cannot lose the rest.
#[tokio::test(flavor = "multi_thread")]
async fn an_accession_the_cluster_lacks_does_not_lose_the_others() {
    let server = MockServer::start_async().await;
    server
        .mock_async(|when, then| {
            when.method(POST).path("/uniprot_entries/_mget");
            then.status(200).json_body(json!({ "docs": [
                { "_id": "P99999", "found": false },
                { "_source": source("P00001", 8501, "Protein one", "") }
            ] }));
        })
        .await;

    let (status, body) = get_against(&server, "/private_api/proteins?accessions[]=P00001&accessions[]=P99999").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().map(Vec::len), Some(1));
    assert_eq!(body[0]["uniprot_accession_id"], "P00001");
}

/// A cluster that is down is a 500, not a panic and not an empty answer dressed as success.
///
/// The body is plain text rather than JSON — `ApiError` renders a bare message — so this reads it
/// raw. Worth knowing: a client parsing every response as JSON will fail on the error path.
#[tokio::test(flavor = "multi_thread")]
async fn a_failing_cluster_is_an_internal_error() {
    let server = MockServer::start_async().await;
    server
        .mock_async(|when, then| {
            when.method(POST);
            then.status(500).body("index is closed");
        })
        .await;

    let (_dir, state) = test_state(&server.base_url());
    let request = Request::get("/private_api/proteins?accessions[]=P00001").body(Body::empty()).unwrap();
    let (status, body) = request_raw(state, request).await;

    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(body, r#"{"error":"Internal server error"}"#, "the cluster's own message does not reach the client");
}

/// The endpoint short-circuits before any request when asked for nothing.
#[tokio::test(flavor = "multi_thread")]
async fn asking_for_no_accessions_makes_no_request() {
    let server = MockServer::start_async().await;
    let mock = server
        .mock_async(|when, then| {
            when.method(POST);
            then.status(200).json_body(json!({ "docs": [] }));
        })
        .await;

    let (status, body) = get_against(&server, "/private_api/proteins").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, json!([]));
    mock.assert_hits_async(0).await;
}

/// Every accession the corpus declares can be asked for at once.
#[tokio::test(flavor = "multi_thread")]
async fn the_whole_corpus_can_be_requested_in_one_batch() {
    let server = MockServer::start_async().await;
    server
        .mock_async(|when, then| {
            when.method(POST).path("/uniprot_entries/_mget");
            then.status(200).json_body(json!({
                "docs": fixtures::ACCESSIONS.iter().map(|a| json!({ "_source": source(a, 8501, "P", "") }))
                    .collect::<Vec<_>>()
            }));
        })
        .await;

    let query: String = fixtures::ACCESSIONS.iter().map(|a| format!("accessions[]={a}")).collect::<Vec<_>>().join("&");
    let (status, body) = get_against(&server, &format!("/private_api/proteins?{query}")).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().map(Vec::len), Some(fixtures::ACCESSIONS.len()));
}

//! `/private_api/proteins/count` and `/proteins/filter` — counting and paging the cluster.

use axum::http::StatusCode;
use httpmock::{Method::POST, MockServer};
use serde_json::json;

use crate::database::get_against;

#[tokio::test(flavor = "multi_thread")]
async fn the_protein_count_is_the_cluster_total() {
    let server = MockServer::start_async().await;
    server
        .mock_async(|when, then| {
            when.method(POST).path("/uniprot_entries/_search").query_param("size", "0");
            then.status(200).json_body(json!({ "hits": { "total": { "value": 4321 } } }));
        })
        .await;

    let (status, body) = get_against(&server, "/private_api/proteins/count").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["count"], 4321);
}

/// The count and the listing are built by two separate query builders that have drifted — one
/// sends a `match` clause for a numeric filter, the other a `term`. This drives both through the
/// endpoints that expose them, so a change to either is visible from outside.
#[tokio::test(flavor = "multi_thread")]
async fn the_filtered_count_and_listing_both_reach_the_cluster() {
    let server = MockServer::start_async().await;
    let count = server
        .mock_async(|when, then| {
            when.method(POST)
                .path("/uniprot_entries/_search")
                .query_param("size", "0")
                .json_body_partial(r#"{ "track_total_hits": true }"#);
            then.status(200).json_body(json!({ "hits": { "total": { "value": 2 } } }));
        })
        .await;
    let list = server
        .mock_async(|when, then| {
            when.method(POST).path("/uniprot_entries/_search").query_param("from", "0").query_param("size", "2");
            then.status(200).json_body(json!({ "hits": { "hits": [
                { "_source": { "uniprot_accession_number": "P00001" } },
                { "_source": { "uniprot_accession_number": "P00003" } }
            ] } }));
        })
        .await;

    let (status, counted) = get_against(&server, "/private_api/proteins/count?filter=8501").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(counted["count"], 2);
    count.assert_async().await;

    let (status, listed) = get_against(&server, "/private_api/proteins/filter?filter=8501&start=0&end=2").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(listed, json!(["P00001", "P00003"]));
    list.assert_async().await;
}

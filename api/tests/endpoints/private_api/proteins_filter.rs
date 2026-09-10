//! `/private_api/proteins/count` and `/private_api/proteins/filter` — counting and paging the
//! cluster.

use axum::{
    body::Body,
    http::{Request, StatusCode}
};
use httpmock::{Method::POST, MockServer};
use serde_json::json;

use crate::{
    common::{request_raw, test_state},
    database::get_against
};

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

/// The count and the listing are built by two separate query builders, and they have drifted: a
/// numeric filter becomes a `match` clause on one side and a `term` on the other. Both mocks name
/// the clause they expect, so neither can change without this failing — and so a controller that
/// dropped the filter altogether would match neither.
#[tokio::test(flavor = "multi_thread")]
async fn the_filtered_count_and_listing_both_reach_the_cluster() {
    let server = MockServer::start_async().await;
    let count = server
        .mock_async(|when, then| {
            when.method(POST).path("/uniprot_entries/_search").query_param("size", "0").json_body_partial(
                r#"{ "track_total_hits": true, "query": { "bool": { "minimum_should_match": 1, "should": [
                           { "wildcard": { "name": { "value": "*8501*", "case_insensitive": true } } },
                           { "prefix": { "uniprot_accession_number": { "value": "8501", "case_insensitive": true } } },
                           { "match": { "taxon_id": { "query": 8501 } } }
                         ] } } }"#
            );
            then.status(200).json_body(json!({ "hits": { "total": { "value": 2 } } }));
        })
        .await;
    let list = server
        .mock_async(|when, then| {
            when.method(POST)
                .path("/uniprot_entries/_search")
                .query_param("from", "0")
                .query_param("size", "2")
                .json_body_partial(
                    r#"{ "query": { "bool": { "minimum_should_match": 1, "should": [
                           { "wildcard": { "name": { "value": "*8501*", "case_insensitive": true } } },
                           { "prefix": { "uniprot_accession_number": { "value": "8501", "case_insensitive": true } } },
                           { "term": { "taxon_id": 8501 } }
                         ] } } }"#
                );
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

/// `end` below `start` is a malformed request, not a server fault.
///
/// Both values reach the query unvalidated, and the page size is their difference. Before this was
/// checked, the ordering panicked the handler task in a debug build and sent a negative `size` to
/// the cluster in a release one — which came back as a 500, so a bad request was logged and
/// alerted on as a server error.
///
/// The mock is asserted to have gone uncalled: the request is refused before the cluster is asked
/// anything, which is what makes this a 400 rather than a failure relayed from OpenSearch.
#[tokio::test(flavor = "multi_thread")]
async fn an_end_below_start_is_rejected() {
    let server = MockServer::start_async().await;
    let mock = server
        .mock_async(|when, then| {
            when.method(POST).path("/uniprot_entries/_search");
            then.status(200).json_body(json!({ "hits": { "hits": [] } }));
        })
        .await;

    let (dir, state) = test_state(&server.base_url());
    let request = Request::get("/private_api/proteins/filter?filter=&start=10&end=0").body(Body::empty()).unwrap();
    let (status, body) = request_raw(state, request).await;
    drop(dir);

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body.contains("end"), "the message should name the parameter, got: {body}");
    mock.assert_hits_async(0).await;
}

/// Paging past `index.max_result_window` is a malformed request, not a server fault.
///
/// OpenSearch refuses a search whose `from + size` passes the window, which the cluster leaves at
/// its default of 10,000. That refusal is a 400 from the cluster, which this crate reports as a
/// `GeneralError` and the API answered as a 500 — so asking the protein browser for the last page
/// of 149 million entries logged and alerted on a server error for a request the cluster will
/// never serve.
///
/// The bound is on `end` alone, not on the page size: `from` is `start` and `size` is their
/// difference, so `from + size` is `end`. Measured against the live API, `end = 10000` answers and
/// `end = 10005` does not.
///
/// The mock is asserted to have gone uncalled, as above: the request is refused before the cluster
/// is asked.
#[tokio::test(flavor = "multi_thread")]
async fn paging_past_the_result_window_is_rejected() {
    let server = MockServer::start_async().await;
    let mock = server
        .mock_async(|when, then| {
            when.method(POST).path("/uniprot_entries/_search");
            then.status(200).json_body(json!({ "hits": { "hits": [] } }));
        })
        .await;

    let (dir, state) = test_state(&server.base_url());
    let request = Request::get("/private_api/proteins/filter?filter=&start=9995&end=10005")
        .body(Body::empty())
        .unwrap();
    let (status, body) = request_raw(state, request).await;
    drop(dir);

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body.contains("10000"), "the message should name the limit, got: {body}");
    mock.assert_hits_async(0).await;
}

/// The last page the window does allow still reaches the cluster.
///
/// Guards the bound against being written as `>=`, or as a limit on `start`, either of which would
/// refuse a page the cluster serves today.
#[tokio::test(flavor = "multi_thread")]
async fn the_last_page_inside_the_window_is_served() {
    let server = MockServer::start_async().await;
    let mock = server
        .mock_async(|when, then| {
            when.method(POST)
                .path("/uniprot_entries/_search")
                .query_param("from", "9990")
                .query_param("size", "10");
            then.status(200).json_body(json!({ "hits": { "hits": [
                { "_source": { "uniprot_accession_number": "P00001" } }
            ] } }));
        })
        .await;

    let (dir, state) = test_state(&server.base_url());
    let request = Request::get("/private_api/proteins/filter?filter=&start=9990&end=10000")
        .body(Body::empty())
        .unwrap();
    let (status, body) = request_raw(state, request).await;
    drop(dir);

    assert_eq!(status, StatusCode::OK, "got: {body}");
    mock.assert_async().await;
}

/// `name` is mapped `text` in `uniprot_entries`, with no `.keyword` subfield, so the cluster
/// refuses to sort on it. The frontend's own type for this parameter offers it, which is why this
/// is refused by name rather than falling back to the default the way the in-memory listings do:
/// answering an accession-ordered page to a caller who asked for `name` is the silent wrong answer
/// this endpoint gave for as long as it dropped the parameter altogether.
///
/// The mock is asserted uncalled — the field never reaches the cluster.
#[tokio::test(flavor = "multi_thread")]
async fn sorting_on_an_unsortable_field_is_rejected() {
    let server = MockServer::start_async().await;
    let mock = server
        .mock_async(|when, then| {
            when.method(POST).path("/uniprot_entries/_search");
            then.status(200).json_body(json!({ "hits": { "hits": [] } }));
        })
        .await;

    let (dir, state) = test_state(&server.base_url());
    let request = Request::get("/private_api/proteins/filter?filter=&start=0&end=10&sort_by=name")
        .body(Body::empty())
        .unwrap();
    let (status, body) = request_raw(state, request).await;
    drop(dir);

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body.contains("name"), "the message should name the field, got: {body}");
    mock.assert_hits_async(0).await;
}

/// The browser sends `sort_by=uniprot_accession_number` on every request and always has — the
/// parameter was simply dropped. An absent value has to mean the same thing, or a client that omits
/// it gets a different order from one that spells out the default.
#[tokio::test(flavor = "multi_thread")]
async fn an_absent_sort_field_is_the_accession() {
    let server = MockServer::start_async().await;
    let mock = server
        .mock_async(|when, then| {
            when.method(POST)
                .path("/uniprot_entries/_search")
                .json_body_partial(r#"{ "sort": [ { "uniprot_accession_number": { "order": "asc" } } ] }"#);
            then.status(200).json_body(json!({ "hits": { "hits": [] } }));
        })
        .await;

    let (dir, state) = test_state(&server.base_url());
    let request = Request::get("/private_api/proteins/filter?filter=&start=0&end=10").body(Body::empty()).unwrap();
    let (status, body) = request_raw(state, request).await;
    drop(dir);

    assert_eq!(status, StatusCode::OK, "got: {body}");
    mock.assert_async().await;
}

//! `/api/v2/protinfo` — protein records from the cluster, named and annotated by the datastore.

use axum::http::StatusCode;
use httpmock::{Method::POST, MockServer};
use serde_json::json;

use crate::database::{get_against, source};

/// `protinfo` is the first endpoint to compose two halves of the state: the cluster supplies the
/// protein, the datastore names its taxon and expands its annotations.
#[tokio::test(flavor = "multi_thread")]
async fn protinfo_joins_the_cluster_to_the_datastore() {
    let server = MockServer::start_async().await;
    server
        .mock_async(|when, then| {
            when.method(POST).path("/uniprot_entries/_mget");
            then.status(200).json_body(json!({ "docs": [
                { "_source": source("P00001", 8501, "Protein one", "EC:1.1.1.1;GO:0009279;IPR:IPR016364") }
            ] }));
        })
        .await;

    let (status, body) = get_against(&server, "/api/v2/protinfo?input[]=P00001&extra=true&names=true").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body[0]["protein"], "P00001");
    assert_eq!(body[0]["taxon_id"], 8501);
    assert_eq!(body[0]["taxon_name"], "Crocodylus niloticus", "the taxon is named out of the datastore");
    assert_eq!(body[0]["ec"][0]["ec_number"], "1.1.1.1");
    assert_eq!(body[0]["ec"][0]["name"], "Alcohol dehydrogenase", "and the EC number expanded out of it");
}

/// `domains` reshapes the GO and InterPro halves the same way it does on the peptide endpoints:
/// a flat list becomes a list of namespace- or category-keyed maps.
#[tokio::test(flavor = "multi_thread")]
async fn domains_groups_the_annotations_by_namespace() {
    let server = MockServer::start_async().await;
    server
        .mock_async(|when, then| {
            when.method(POST).path("/uniprot_entries/_mget");
            then.status(200).json_body(json!({ "docs": [
                { "_source": source("P00001", 8501, "Protein one", "EC:1.1.1.1;GO:0009279;IPR:IPR016364") }
            ] }));
        })
        .await;

    let (_, flat) = get_against(&server, "/api/v2/protinfo?input[]=P00001&extra=true").await;
    assert_eq!(flat[0]["go"][0]["go_term"], "GO:0009279", "a flat list by default");

    let (status, grouped) = get_against(&server, "/api/v2/protinfo?input[]=P00001&extra=true&domains=true").await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        grouped[0]["go"]
            .as_array()
            .expect("namespace maps")
            .iter()
            .any(|e| e.get("cellular component").is_some()),
        "expected namespace keys: {}",
        grouped[0]["go"]
    );
}

/// `names` adds the taxon's name; `extra` adds the annotations. Both off is the default.
#[tokio::test(flavor = "multi_thread")]
async fn extra_and_names_choose_how_much_comes_back() {
    let server = MockServer::start_async().await;
    server
        .mock_async(|when, then| {
            when.method(POST).path("/uniprot_entries/_mget");
            then.status(200).json_body(json!({ "docs": [
                { "_source": source("P00001", 8501, "Protein one", "EC:1.1.1.1") }
            ] }));
        })
        .await;

    let (status, plain) = get_against(&server, "/api/v2/protinfo?input[]=P00001").await;
    assert_eq!(status, StatusCode::OK);
    assert!(plain[0]["ec"].as_array().is_some_and(|list| list.first().is_none_or(|e| e.get("name").is_none())));

    let (_, named) = get_against(&server, "/api/v2/protinfo?input[]=P00001&extra=true&names=true").await;
    assert_eq!(named[0]["taxon_name"], "Crocodylus niloticus");
    assert_eq!(named[0]["ec"][0]["name"], "Alcohol dehydrogenase");
}

/// An accession the cluster does not hold yields no row rather than an error.
#[tokio::test(flavor = "multi_thread")]
async fn an_unresolvable_accession_yields_no_row() {
    let server = MockServer::start_async().await;
    server
        .mock_async(|when, then| {
            when.method(POST).path("/uniprot_entries/_mget");
            then.status(200).json_body(json!({ "docs": [] }));
        })
        .await;

    let (status, body) = get_against(&server, "/api/v2/protinfo?input[]=P99999").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, json!([]));
}

/// The results follow the input. The accessions reach the cluster in the order they were given,
/// and `mget` answers in that same order.
///
/// The mock echoes back whichever order it is asked for, so what this pins is that the endpoint
/// asks in input order rather than in the order of a set it built along the way.
#[tokio::test(flavor = "multi_thread")]
async fn results_follow_the_order_of_the_input() {
    let server = MockServer::start_async().await;
    server
        .mock_async(|when, then| {
            when.method(POST)
                .path("/uniprot_entries/_mget")
                .json_body(json!({ "docs": [ { "_id": "P00003" }, { "_id": "P00001" } ] }));
            then.status(200).json_body(json!({ "docs": [
                { "_source": source("P00003", 8502, "Protein three", "EC:1.1.1.1") },
                { "_source": source("P00001", 8501, "Protein one", "EC:1.1.1.1") }
            ] }));
        })
        .await;

    let (status, body) = get_against(&server, "/api/v2/protinfo?input[]=P00003&input[]=P00001").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body[0]["protein"], "P00003");
    assert_eq!(body[1]["protein"], "P00001");
}

//! `/private_api/taxa` — taxon ids to names, ranks and lineages.

use axum::http::StatusCode;
use serde_json::json;

use crate::common::{get_json, post_json};

#[tokio::test(flavor = "multi_thread")]
async fn one_id_resolves_to_its_name_rank_and_lineage() {
    let (status, body) = get_json("/private_api/taxa?taxids[]=8501").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body[0]["id"], 8501);
    assert_eq!(body[0]["name"], "Crocodylus niloticus");
    assert_eq!(body[0]["rank"], "species");
    assert_eq!(
        body[0]["lineage"].as_array().map(Vec::len),
        Some(datastore::RANK_COUNT),
        "one entry per rank, recorded or not"
    );
}

/// A lineage carries `null` at every rank the taxonomy does not record, and this taxon has no
/// class — the gap the LCA reduction has to step over elsewhere.
#[tokio::test(flavor = "multi_thread")]
async fn an_unrecorded_rank_is_null_rather_than_absent() {
    let (_, body) = get_json("/private_api/taxa?taxids[]=8501").await;
    let lineage = body[0]["lineage"].as_array().expect("a lineage");

    let column = |rank: datastore::TaxonRank| rank.lineage_index().expect("a lineage column");

    assert!(lineage[column(datastore::TaxonRank::named("class"))].is_null(), "no class for Crocodylus niloticus");
    assert_eq!(lineage[column(datastore::TaxonRank::GENUS)], 8500, "but genus is recorded");
}

#[tokio::test(flavor = "multi_thread")]
async fn several_ids_resolve_in_one_call() {
    let (status, body) = get_json("/private_api/taxa?taxids[]=8501&taxids[]=8502&taxids[]=7").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().map(Vec::len), Some(3));
}

/// The corpus holds one taxon the taxonomy marks invalid. It is still a taxon, and still named.
#[tokio::test(flavor = "multi_thread")]
async fn an_invalid_taxon_is_still_returned() {
    let (status, body) = get_json(&format!("/private_api/taxa?taxids[]={}", fixtures::taxa::HELODERMA)).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body[0]["name"], "Heloderma sp.");
}

#[tokio::test(flavor = "multi_thread")]
async fn an_unknown_id_is_omitted_rather_than_failing() {
    let (status, body) = get_json("/private_api/taxa?taxids[]=999999999&taxids[]=8501").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().map(Vec::len), Some(1));
}

#[tokio::test(flavor = "multi_thread")]
async fn no_ids_is_an_empty_answer() {
    assert_eq!(get_json("/private_api/taxa").await.1, json!([]));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_post_answers_like_a_get() {
    let (_, from_get) = get_json("/private_api/taxa?taxids[]=8501").await;
    let (status, from_post) = post_json("/private_api/taxa", json!({ "taxids": [8501] })).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(from_post, from_get);
}

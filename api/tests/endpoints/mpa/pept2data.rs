//! `/mpa/pept2data` — the batch endpoint the applications actually use.

use axum::http::StatusCode;
use fixtures::peptides::*;
use serde_json::json;

use crate::common::post_json;

/// The endpoint the desktop and web applications actually post to.
#[tokio::test(flavor = "multi_thread")]
async fn pept2data_answers_with_an_lca_and_a_lineage() {
    let (status, body) = post_json("/mpa/pept2data", json!({ "peptides": [UNIQUE], "equate_il": true })).await;

    assert_eq!(status, StatusCode::OK);
    let item = &body["peptides"][0];
    assert_eq!(item["sequence"], UNIQUE);
    assert_eq!(item["lca"], 8501);
    assert_eq!(item["lineage"].as_array().map(Vec::len), Some(28));
}

/// `report_taxa` adds the taxa the peptide reached, which are otherwise reduced away to the LCA.
#[tokio::test(flavor = "multi_thread")]
async fn report_taxa_adds_the_taxa_behind_the_lca() {
    let (_, without) = post_json("/mpa/pept2data", json!({ "peptides": [GENUS_SHARED] })).await;
    assert!(without["peptides"][0].get("taxa").is_none(), "absent unless asked for");

    let (status, with) = post_json("/mpa/pept2data", json!({ "peptides": [GENUS_SHARED], "report_taxa": true })).await;
    assert_eq!(status, StatusCode::OK);

    let mut taxa: Vec<u64> = with["peptides"][0]["taxa"]
        .as_array()
        .expect("the taxa behind the answer")
        .iter()
        .map(|taxon| taxon.as_u64().expect("an id"))
        .collect();
    taxa.sort_unstable();
    assert_eq!(taxa, vec![8501, 8502], "the two species whose LCA is the genus");
}

/// `tryptic` can only narrow the answer, never widen it.
#[tokio::test(flavor = "multi_thread")]
async fn tryptic_narrows_the_proteins_a_peptide_reaches() {
    let (_, all) = post_json("/mpa/pept2data", json!({ "peptides": [COMMON], "report_taxa": true })).await;
    let (status, tryptic) =
        post_json("/mpa/pept2data", json!({ "peptides": [COMMON], "report_taxa": true, "tryptic": true })).await;

    assert_eq!(status, StatusCode::OK);

    let taxa = |value: &serde_json::Value| -> std::collections::BTreeSet<u64> {
        value["peptides"][0]["taxa"]
            .as_array()
            .map(|list| list.iter().filter_map(serde_json::Value::as_u64).collect())
            .unwrap_or_default()
    };
    assert!(taxa(&tryptic).is_subset(&taxa(&all)));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_cutoff_is_reported_on_the_result() {
    let (status, capped) = post_json("/mpa/pept2data", json!({ "peptides": [COMMON], "cutoff": 2 })).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(capped["peptides"][0]["cutoff_used"], true);
}

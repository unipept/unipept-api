//! `/mpa/pept2data` — the batch endpoint the applications actually use.

use axum::http::StatusCode;
use fixtures::peptides::*;
use serde_json::json;

use crate::common::post_json;

/// The peptide each row names, in the order they arrive.
fn sequences(body: &serde_json::Value) -> Vec<&str> {
    body["peptides"]
        .as_array()
        .expect("a list of peptides")
        .iter()
        .map(|item| item["sequence"].as_str().expect("a sequence"))
        .collect()
}

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

// ── the filter parameter ─────────────────────────────────────────────────────────────────────────
//
// `filter` picks one of three `UniprotFilter` implementations, or none. It is the only parameter
// here that changes which *proteins* contribute rather than how they are reported, so its effect
// shows up twice: in the taxa a peptide reports, and in the LCA those taxa reduce to.
//
// `COMMON` is the peptide with something to filter — seven proteins across six taxa.

/// The taxa a peptide reports under a given filter, sorted.
async fn taxa_under(filter: serde_json::Value) -> Vec<u64> {
    let mut body = json!({ "peptides": [COMMON], "report_taxa": true });
    if !filter.is_null() {
        body["filter"] = filter;
    }

    let (status, answered) = post_json("/mpa/pept2data", body).await;
    assert_eq!(status, StatusCode::OK);

    let mut taxa: Vec<u64> = answered["peptides"][0]["taxa"]
        .as_array()
        .unwrap_or_else(|| panic!("no taxa in {}", answered["peptides"][0]))
        .iter()
        .map(|taxon| taxon.as_u64().expect("an id"))
        .collect();
    taxa.sort_unstable();
    taxa
}

/// No filter keeps every protein the search found.
#[tokio::test(flavor = "multi_thread")]
async fn without_a_filter_every_taxon_is_reported() {
    assert_eq!(taxa_under(serde_json::Value::Null).await, vec![7, 9, 8501, 8502, 8503, 9503]);
}

/// A taxon filter matches on the whole lineage, not on the protein's own taxon: asking for the
/// genus keeps the three *Crocodylus* species beneath it and drops everything else.
#[tokio::test(flavor = "multi_thread")]
async fn a_taxon_filter_keeps_the_descendants_of_what_it_names() {
    assert_eq!(taxa_under(json!({ "taxa": [8500] })).await, vec![8501, 8502, 8503]);
}

/// Narrowing the filter narrows the answer, and the LCA follows it down.
#[tokio::test(flavor = "multi_thread")]
async fn a_narrower_taxon_filter_moves_the_lca_deeper() {
    let genus = post_json("/mpa/pept2data", json!({ "peptides": [COMMON], "filter": { "taxa": [8500] } })).await;
    let species = post_json("/mpa/pept2data", json!({ "peptides": [COMMON], "filter": { "taxa": [8501] } })).await;

    assert_eq!(genus.0, StatusCode::OK);
    assert_eq!(species.0, StatusCode::OK);
    assert_eq!(genus.1["peptides"][0]["lca"], 8500, "three species of one genus reduce to it");
    assert_eq!(species.1["peptides"][0]["lca"], 8501, "one species is its own ancestor");
}

/// Root is the special case: a filter naming taxon 1 would keep everything anyway, and the handler
/// short-circuits to the empty filter rather than walking every lineage to prove it.
#[tokio::test(flavor = "multi_thread")]
async fn a_filter_naming_root_keeps_everything() {
    assert_eq!(taxa_under(json!({ "taxa": [1] })).await, taxa_under(serde_json::Value::Null).await);
}

/// A proteome filter is a protein filter one step removed: the accessions come from the proteome's
/// own list, so `UP000000001` keeps P00001 and P00002 — both *C. niloticus* — and nothing else.
#[tokio::test(flavor = "multi_thread")]
async fn a_proteome_filter_keeps_only_that_proteome_s_proteins() {
    assert_eq!(taxa_under(json!({ "proteomes": ["UP000000001"] })).await, vec![8501]);
}

/// Two proteomes union rather than intersect.
#[tokio::test(flavor = "multi_thread")]
async fn several_proteomes_are_taken_together() {
    let taxa = taxa_under(json!({ "proteomes": ["UP000000001", "UP000000003"] })).await;

    assert_eq!(taxa, vec![7, 8501], "C. niloticus from the first, the bacterium from the third");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_protein_filter_keeps_only_the_accessions_it_names() {
    assert_eq!(taxa_under(json!({ "proteins": ["P00005"] })).await, vec![9503]);
    assert_eq!(taxa_under(json!({ "proteins": ["P00001", "P00006"] })).await, vec![7, 8501]);
}

/// A filter that matches nothing drops the peptide from the answer entirely, rather than reporting
/// it with an empty taxa list — the same shape an absent peptide produces.
#[tokio::test(flavor = "multi_thread")]
async fn a_filter_matching_nothing_drops_the_peptide() {
    for filter in
        [json!({ "taxa": [999999999] }), json!({ "proteomes": ["UP999999999"] }), json!({ "proteins": ["P99999"] })]
    {
        let (status, body) =
            post_json("/mpa/pept2data", json!({ "peptides": [COMMON], "filter": filter.clone() })).await;

        assert_eq!(status, StatusCode::OK, "{filter}");
        assert_eq!(body["peptides"].as_array().map(Vec::len), Some(0), "{filter} should drop the peptide");
    }
}

/// An unknown proteome is skipped rather than failing the request, so one bad accession alongside
/// a good one still answers.
#[tokio::test(flavor = "multi_thread")]
async fn an_unknown_proteome_alongside_a_known_one_is_ignored() {
    assert_eq!(taxa_under(json!({ "proteomes": ["UP000000001", "UP999999999"] })).await, vec![8501]);
}

/// Filtering happens before the annotations are aggregated, so a narrower filter cannot report an
/// annotation carried only by a protein it excluded.
#[tokio::test(flavor = "multi_thread")]
async fn the_annotations_come_from_the_surviving_proteins_only() {
    let (status, filtered) =
        post_json("/mpa/pept2data", json!({ "peptides": [COMMON], "filter": { "proteins": ["P00007"] } })).await;

    assert_eq!(status, StatusCode::OK);
    let fa = &filtered["peptides"][0]["fa"];
    assert_eq!(fa["counts"]["all"], 1, "one protein survived the filter");
}

/// The peptides that reach the index are the distinct ones, in the order they were sent.
///
/// This endpoint answers one row per distinct peptide rather than one per position, unlike the
/// seven others: it is what a whole sample is posted to, and a sample names the same peptide many
/// times. What it shares with them is the order — the answer follows the request.
#[tokio::test(flavor = "multi_thread")]
async fn peptides_are_answered_once_each_in_the_order_they_were_sent() {
    let (status, body) = post_json("/mpa/pept2data", json!({ "peptides": [GENUS_SHARED, UNIQUE, GENUS_SHARED] })).await;

    assert_eq!(status, StatusCode::OK);

    assert_eq!(sequences(&body), vec![GENUS_SHARED, UNIQUE], "one row each, in first-appearance order");

    // And the other way round, so this cannot pass on a fixed order that happens to match.
    let (_, reversed) = post_json("/mpa/pept2data", json!({ "peptides": [UNIQUE, GENUS_SHARED] })).await;
    assert_eq!(sequences(&reversed), vec![UNIQUE, GENUS_SHARED]);
}

/// Two spellings that differ only in case, or in trailing space, are one peptide.
///
/// Sanitising upper-cases and trims, and the deduplication runs after it. When it ran before, both
/// spellings survived it: the same peptide was searched twice and answered twice, in two rows a
/// caller could not tell apart.
#[tokio::test(flavor = "multi_thread")]
async fn spellings_that_sanitise_alike_are_one_peptide() {
    let lowercased = GENUS_SHARED.to_lowercase();
    let (status, body) = post_json("/mpa/pept2data", json!({ "peptides": [GENUS_SHARED, lowercased] })).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(sequences(&body), vec![GENUS_SHARED], "one row, not two identical ones");

    let trailing = format!("{GENUS_SHARED} ");
    let (_, trimmed) = post_json("/mpa/pept2data", json!({ "peptides": [GENUS_SHARED, trailing] })).await;
    assert_eq!(sequences(&trimmed), vec![GENUS_SHARED]);
}

//! `/api/v2/taxa2lca` — the lowest common ancestor of a set of taxa.
//!
//! Three optional flags — `extra`, `names`, `validate_taxa` — and the taxonomy half of `pept2lca`
//! without a peptide search in front of it.

use axum::http::StatusCode;
use serde_json::json;

use crate::common::{get_json, post_json};

#[tokio::test(flavor = "multi_thread")]
async fn two_species_reduce_to_their_genus() {
    let (status, body) = get_json("/api/v2/taxa2lca?input[]=8501&input[]=8502").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["taxon_id"], 8500);
    assert_eq!(body["taxon_name"], "Crocodylus");
    assert_eq!(body["taxon_rank"], "genus");
}

#[tokio::test(flavor = "multi_thread")]
async fn three_species_of_one_genus_still_reduce_to_it() {
    let (_, body) = get_json("/api/v2/taxa2lca?input[]=8501&input[]=8502&input[]=8503").await;

    assert_eq!(body["taxon_id"], 8500);
}

/// The reptile and the monkey diverge at class, where one of them records nothing, so their
/// ancestor sits above it.
#[tokio::test(flavor = "multi_thread")]
async fn taxa_diverging_at_class_reduce_to_the_superclass() {
    let (_, body) = get_json("/api/v2/taxa2lca?input[]=8501&input[]=9503").await;

    assert_eq!(body["taxon_id"], fixtures::taxa::SARCOPTERYGII);
    assert_eq!(body["taxon_rank"], "superclass");
}

#[tokio::test(flavor = "multi_thread")]
async fn taxa_from_different_domains_reduce_to_root() {
    let (_, body) = get_json("/api/v2/taxa2lca?input[]=8501&input[]=7").await;

    assert_eq!(body["taxon_id"], 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_single_taxon_is_its_own_ancestor() {
    let (_, body) = get_json("/api/v2/taxa2lca?input[]=8501").await;

    assert_eq!(body["taxon_id"], 8501);
}

/// `extra` adds the lineage and `names` expands it. All four combinations, since the two flags are
/// read together and only one of the four is the default.
#[tokio::test(flavor = "multi_thread")]
async fn extra_and_names_choose_how_much_lineage_comes_back() {
    let (_, plain) = get_json("/api/v2/taxa2lca?input[]=8501&input[]=8502").await;
    assert!(plain.get("genus_id").is_none(), "neither flag: no lineage at all");

    let (_, named_only) = get_json("/api/v2/taxa2lca?input[]=8501&input[]=8502&names=true").await;
    assert!(named_only.get("genus_id").is_none(), "names without extra adds nothing");

    let (_, extra_only) = get_json("/api/v2/taxa2lca?input[]=8501&input[]=8502&extra=true").await;
    assert_eq!(extra_only["genus_id"], 8500, "extra adds the ids");
    assert!(extra_only.get("genus_name").is_none(), "but not the names");

    let (status, both) = get_json("/api/v2/taxa2lca?input[]=8501&input[]=8502&extra=true&names=true").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(both["genus_id"], 8500);
    assert_eq!(both["genus_name"], "Crocodylus");
    assert_eq!(both["domain_name"], "Eukaryota");
}

/// The corpus has exactly one invalid taxon, which is what makes this flag observable.
#[tokio::test(flavor = "multi_thread")]
async fn validate_taxa_drops_a_taxon_the_taxonomy_marks_invalid() {
    let query = format!("input[]=8501&input[]={}", fixtures::taxa::HELODERMA);

    let (_, unvalidated) = get_json(&format!("/api/v2/taxa2lca?{query}&validate_taxa=false")).await;
    let (status, validated) = get_json(&format!("/api/v2/taxa2lca?{query}&validate_taxa=true")).await;

    assert_eq!(status, StatusCode::OK);
    assert_ne!(unvalidated["taxon_id"], validated["taxon_id"], "the flag has to change the answer");
    assert_eq!(validated["taxon_id"], 8501, "with the invalid taxon dropped only C. niloticus is left");
}

/// The parameter is an `Either`, so a taxon id may arrive as a number or as a string.
#[tokio::test(flavor = "multi_thread")]
async fn ids_may_be_written_as_strings() {
    let (_, numeric) = get_json("/api/v2/taxa2lca?input[]=8501&input[]=8502").await;
    let (status, quoted) = post_json("/api/v2/taxa2lca", json!({ "input": ["8501", "8502"] })).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(quoted, numeric);
}

#[tokio::test(flavor = "multi_thread")]
async fn an_unknown_taxon_does_not_fail_the_request() {
    let (status, _) = get_json("/api/v2/taxa2lca?input[]=999999999").await;

    assert_eq!(status, StatusCode::OK);
}

#[tokio::test(flavor = "multi_thread")]
async fn no_input_still_answers() {
    let (status, _) = get_json("/api/v2/taxa2lca").await;

    assert_eq!(status, StatusCode::OK);
}

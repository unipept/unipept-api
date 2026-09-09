//! `/api/v2/peptinfo` — `pept2funct` and `pept2lca` in one response.

use axum::http::StatusCode;
use fixtures::peptides::*;

use crate::common::get_json;

/// `peptinfo` is `pept2funct` and `pept2lca` in one response.
#[tokio::test(flavor = "multi_thread")]
async fn peptinfo_carries_both_the_taxon_and_the_annotations() {
    let (status, body) = get_json(&format!("/api/v2/peptinfo?input[]={UNIQUE}&extra=true")).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body[0]["peptide"], UNIQUE);
    assert_eq!(body[0]["taxon_id"], 8501);
    assert_eq!(body[0]["ec"][0]["ec_number"], "1.1.1.1");
}

/// `peptinfo` takes `domains` too, and it reshapes the GO half the same way `pept2go` does.
#[tokio::test(flavor = "multi_thread")]
async fn domains_groups_the_go_terms_by_namespace() {
    let (status, grouped) = get_json(&format!("/api/v2/peptinfo?input[]={UNIQUE}&extra=true&domains=true")).await;

    assert_eq!(status, StatusCode::OK);
    let namespaces = grouped[0]["go"].as_array().expect("a list of namespace maps");
    assert!(
        namespaces.iter().any(|entry| entry.get("cellular component").is_some()),
        "expected namespace keys: {}",
        grouped[0]["go"]
    );
}

/// It carries `validate_taxa` as well, and the taxon half is a full `pept2lca` answer.
#[tokio::test(flavor = "multi_thread")]
async fn validate_taxa_drops_the_invalid_taxon() {
    let (_, unvalidated) = get_json(&format!("/api/v2/peptinfo?input[]={VALIDATION_SHARED}&validate_taxa=false")).await;
    let (status, validated) =
        get_json(&format!("/api/v2/peptinfo?input[]={VALIDATION_SHARED}&validate_taxa=true")).await;

    assert_eq!(status, StatusCode::OK);
    assert_ne!(unvalidated[0]["taxon_id"], validated[0]["taxon_id"]);
    assert_eq!(validated[0]["taxon_id"], 8501);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_cutoff_is_reported_on_the_result() {
    let (status, capped) = get_json(&format!("/api/v2/peptinfo?input[]={COMMON}&cutoff=2")).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(capped[0]["cutoff_used"], true);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_repeated_peptide_answers_at_each_position() {
    super::a_repeat_answers_like_a_single("peptinfo", "extra=true&names=true").await;
}

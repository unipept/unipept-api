//! `/api/v2/pept2go` — the GO terms of the proteins a peptide reaches.
//!
//! The one annotation endpoint with a `domains` flag, which does not decorate the answer but
//! reshapes it.

use axum::http::StatusCode;
use fixtures::peptides::*;

use crate::common::get_json;

#[tokio::test(flavor = "multi_thread")]
async fn a_peptide_resolves_to_the_go_terms_of_its_proteins() {
    let (status, body) = get_json(&format!("/api/v2/pept2go?input[]={UNIQUE}")).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body[0]["go"][0]["go_term"], "GO:0009279");
}

#[tokio::test(flavor = "multi_thread")]
async fn extra_adds_the_name_from_the_datastore() {
    let (status, body) = get_json(&format!("/api/v2/pept2go?input[]={UNIQUE}&extra=true")).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body[0]["go"][0]["name"], "cell outer membrane");
}

/// `domains` changes the shape of the answer rather than its contents: a flat list of terms becomes
/// a list of namespace-keyed maps. A client written against one shape cannot read the other, so
/// this is the flag most worth pinning on this endpoint.
#[tokio::test(flavor = "multi_thread")]
async fn domains_groups_the_terms_by_namespace() {
    let (_, flat) = get_json(&format!("/api/v2/pept2go?input[]={UNIQUE}")).await;
    assert_eq!(flat[0]["go"][0]["go_term"], "GO:0009279", "a flat list of terms by default");

    let (status, grouped) = get_json(&format!("/api/v2/pept2go?input[]={UNIQUE}&domains=true&extra=true")).await;
    assert_eq!(status, StatusCode::OK);

    let namespaces = grouped[0]["go"].as_array().expect("a list of namespace maps");
    assert!(!namespaces.is_empty(), "grouped: {}", grouped[0]["go"]);

    let cellular = namespaces
        .iter()
        .find_map(|entry| entry.get("cellular component"))
        .unwrap_or_else(|| panic!("the term's own namespace should be a key: {}", grouped[0]["go"]));
    assert_eq!(cellular[0]["go_term"], "GO:0009279");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_cutoff_is_reported_on_the_result() {
    let (status, capped) = get_json(&format!("/api/v2/pept2go?input[]={COMMON}&cutoff=2")).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(capped[0]["cutoff_used"], true);
}

#[tokio::test(flavor = "multi_thread")]
async fn equate_il_widens_which_proteins_are_reached() {
    let (_, apart) = get_json(&format!("/api/v2/pept2go?input[]={IL_ISOLEUCINE}&equate_il=false")).await;
    let (status, together) = get_json(&format!("/api/v2/pept2go?input[]={IL_ISOLEUCINE}&equate_il=true")).await;

    assert_eq!(status, StatusCode::OK);
    assert!(together[0]["total_protein_count"].as_u64() > apart[0]["total_protein_count"].as_u64());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_repeated_peptide_answers_at_each_position() {
    super::a_repeat_answers_like_a_single("pept2go", "extra=true&domains=true").await;
}

/// A peptide matching nothing takes no position, however many times it is named.
#[tokio::test(flavor = "multi_thread")]
async fn a_repeated_peptide_that_matches_nothing_takes_no_position() {
    let (status, body) = get_json(&format!("/api/v2/pept2go?input[]={ABSENT}&input[]={UNIQUE}&input[]={ABSENT}")).await;

    assert_eq!(status, StatusCode::OK);
    let rows = body.as_array().expect("a list");
    assert_eq!(rows.len(), 1, "only the peptide that matched: {body}");
    assert_eq!(rows[0]["peptide"], UNIQUE);
}

/// `sanitize_peptides` upper-cases before anything deduplicates, so two spellings differing only in
/// case are one peptide and two positions.
#[tokio::test(flavor = "multi_thread")]
async fn case_folds_before_the_peptides_are_deduplicated() {
    let lowercased = UNIQUE.to_lowercase();
    let (status, body) = get_json(&format!("/api/v2/pept2go?input[]={UNIQUE}&input[]={lowercased}")).await;

    assert_eq!(status, StatusCode::OK);
    let rows = body.as_array().expect("a list");
    assert_eq!(rows.len(), 2, "both positions answered: {body}");
    assert_eq!(rows[0], rows[1]);
    assert_eq!(rows[0]["peptide"], UNIQUE, "answered under the sanitised spelling");
}

/// `equate_il` widens which proteins a peptide reaches; it does not make two peptides one.
#[tokio::test(flavor = "multi_thread")]
async fn equate_il_does_not_merge_two_peptides_into_one() {
    let (status, body) =
        get_json(&format!("/api/v2/pept2go?input[]={IL_ISOLEUCINE}&input[]={IL_LEUCINE}&equate_il=true")).await;

    assert_eq!(status, StatusCode::OK);
    let rows = body.as_array().expect("a list");
    assert_eq!(rows.len(), 2, "two peptides, two rows: {body}");
    assert_eq!(rows[0]["peptide"], IL_ISOLEUCINE);
    assert_eq!(rows[1]["peptide"], IL_LEUCINE);
}

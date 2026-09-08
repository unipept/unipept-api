//! `/api/v2/pept2ec` — the EC numbers of the proteins a peptide reaches.

use axum::http::StatusCode;
use fixtures::peptides::*;

use crate::common::get_json;

#[tokio::test(flavor = "multi_thread")]
async fn a_peptide_resolves_to_the_ec_numbers_of_its_proteins() {
    let (status, body) = get_json(&format!("/api/v2/pept2ec?input[]={UNIQUE}")).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body[0]["peptide"], UNIQUE);
    assert_eq!(body[0]["ec"][0]["ec_number"], "1.1.1.1");
}

/// `extra` is what turns a bare code into a named one, through the EC store.
#[tokio::test(flavor = "multi_thread")]
async fn extra_adds_the_name_from_the_datastore() {
    let (_, plain) = get_json(&format!("/api/v2/pept2ec?input[]={UNIQUE}")).await;
    assert!(plain[0]["ec"][0].get("name").is_none(), "no name without the flag");

    let (status, named) = get_json(&format!("/api/v2/pept2ec?input[]={UNIQUE}&extra=true")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(named[0]["ec"][0]["name"], "Alcohol dehydrogenase");
}

#[tokio::test(flavor = "multi_thread")]
async fn equate_il_widens_which_proteins_are_reached() {
    let (_, apart) = get_json(&format!("/api/v2/pept2ec?input[]={IL_ISOLEUCINE}&equate_il=false")).await;
    let (status, together) = get_json(&format!("/api/v2/pept2ec?input[]={IL_ISOLEUCINE}&equate_il=true")).await;

    assert_eq!(status, StatusCode::OK);
    assert!(
        together[0]["total_protein_count"].as_u64() > apart[0]["total_protein_count"].as_u64(),
        "equated, the peptide reaches a second protein"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_cutoff_is_reported_on_the_result() {
    let (_, uncapped) = get_json(&format!("/api/v2/pept2ec?input[]={COMMON}")).await;
    let (status, capped) = get_json(&format!("/api/v2/pept2ec?input[]={COMMON}&cutoff=2")).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(uncapped[0]["cutoff_used"], false);
    assert_eq!(capped[0]["cutoff_used"], true);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_protein_without_ec_numbers_answers_with_an_empty_list() {
    let (status, body) = get_json(&format!("/api/v2/pept2ec?input[]={VALIDATION_SHARED}")).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body[0]["ec"].as_array().map(Vec::len), Some(0));
}

/// A peptide that matches nothing must not shift the repeat counts of the ones that do.
///
/// `analyse` drops a peptide with no matches, so the result list is shorter than the list of unique
/// peptides that was searched, and each result has to carry the count of the peptide it names.
#[tokio::test(flavor = "multi_thread")]
async fn a_peptide_that_matches_nothing_does_not_shift_the_other_counts() {
    let input = format!(
        "input[]={ABSENT}\
         &input[]={UNIQUE}&input[]={UNIQUE}\
         &input[]={GENUS_SHARED}&input[]={GENUS_SHARED}&input[]={GENUS_SHARED}\
         &input[]={VALIDATION_SHARED}"
    );

    let (status, body) = get_json(&format!("/api/v2/pept2ec?{input}")).await;
    assert_eq!(status, StatusCode::OK);

    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for entry in body.as_array().expect("an array of results") {
        *counts.entry(entry["peptide"].as_str().expect("a peptide name").to_string()).or_insert(0) += 1;
    }

    assert_eq!(counts.get(UNIQUE), Some(&2), "asked for twice");
    assert_eq!(counts.get(GENUS_SHARED), Some(&3), "asked for three times");
    assert_eq!(counts.get(VALIDATION_SHARED), Some(&1), "asked for once");
    assert_eq!(counts.get(ABSENT), None, "matches nothing, so it has no result");
    assert_eq!(body.as_array().map(Vec::len), Some(6), "no result invented and none lost");
}

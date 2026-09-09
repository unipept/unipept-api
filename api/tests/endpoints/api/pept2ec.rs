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

/// The peptides each result names, in the order they arrive.
fn peptides(body: &serde_json::Value) -> Vec<&str> {
    body.as_array()
        .expect("an array of results")
        .iter()
        .map(|entry| entry["peptide"].as_str().expect("a peptide name"))
        .collect()
}

/// The results follow the input.
///
/// Sending the same two peptides both ways round is what separates "in input order" from "in some
/// fixed order".
#[tokio::test(flavor = "multi_thread")]
async fn results_follow_the_order_of_the_input() {
    let (status, body) = get_json(&format!("/api/v2/pept2ec?input[]={GENUS_SHARED}&input[]={UNIQUE}")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(peptides(&body), vec![GENUS_SHARED, UNIQUE]);

    let (status, reversed) = get_json(&format!("/api/v2/pept2ec?input[]={UNIQUE}&input[]={GENUS_SHARED}")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(peptides(&reversed), vec![UNIQUE, GENUS_SHARED]);
}

/// A repeated peptide is answered at each position it occupies, and one that matches nothing takes
/// no position at all.
///
/// `analyse` drops a peptide with no matches, so a result carries no answer for it and the
/// positions of every other peptide are unaffected.
#[tokio::test(flavor = "multi_thread")]
async fn a_peptide_is_answered_at_each_position_it_occupies() {
    let input = format!(
        "input[]={ABSENT}\
         &input[]={UNIQUE}&input[]={GENUS_SHARED}\
         &input[]={UNIQUE}&input[]={GENUS_SHARED}&input[]={GENUS_SHARED}\
         &input[]={VALIDATION_SHARED}"
    );

    let (status, body) = get_json(&format!("/api/v2/pept2ec?{input}")).await;
    assert_eq!(status, StatusCode::OK);

    assert_eq!(
        peptides(&body),
        vec![UNIQUE, GENUS_SHARED, UNIQUE, GENUS_SHARED, GENUS_SHARED, VALIDATION_SHARED],
        "each peptide sits where it was named; ABSENT matches nothing and is dropped"
    );
}

/// The one shape every peptide endpoint answers in, which `pept2ec` used to be alone in not
/// following. Asserted against a sibling rather than against a literal, so the two cannot drift.
#[tokio::test(flavor = "multi_thread")]
async fn repeats_sit_where_the_other_endpoints_put_them() {
    let input = format!("input[]={GENUS_SHARED}&input[]={UNIQUE}&input[]={GENUS_SHARED}");

    let (status, ecs) = get_json(&format!("/api/v2/pept2ec?{input}")).await;
    let (_, gos) = get_json(&format!("/api/v2/pept2go?{input}")).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(peptides(&ecs), peptides(&gos));
}

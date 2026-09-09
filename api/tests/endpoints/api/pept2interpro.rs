//! `/api/v2/pept2interpro` — the InterPro entries of the proteins a peptide reaches.

use axum::http::StatusCode;
use fixtures::peptides::*;

use crate::common::get_json;

#[tokio::test(flavor = "multi_thread")]
async fn a_peptide_resolves_to_the_interpro_entries_of_its_proteins() {
    let (status, body) = get_json(&format!("/api/v2/pept2interpro?input[]={UNIQUE}")).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body[0]["ipr"][0]["code"], "IPR016364");
}

#[tokio::test(flavor = "multi_thread")]
async fn extra_adds_the_name_from_the_datastore() {
    let (status, body) = get_json(&format!("/api/v2/pept2interpro?input[]={UNIQUE}&extra=true")).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body[0]["ipr"][0]["name"], "Alcohol dehydrogenase, zinc-type");
}

/// InterPro takes the same `domains` flag as GO, and groups by the entry's category.
#[tokio::test(flavor = "multi_thread")]
async fn domains_groups_the_entries_by_category() {
    let (status, grouped) = get_json(&format!("/api/v2/pept2interpro?input[]={UNIQUE}&domains=true&extra=true")).await;

    assert_eq!(status, StatusCode::OK);
    let categories = grouped[0]["ipr"].as_array().expect("a list of category maps");
    let family = categories
        .iter()
        .find_map(|entry| entry.get("Family"))
        .unwrap_or_else(|| panic!("the entry's own category should be a key: {}", grouped[0]["ipr"]));
    assert_eq!(family[0]["code"], "IPR016364");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_cutoff_is_reported_on_the_result() {
    let (status, capped) = get_json(&format!("/api/v2/pept2interpro?input[]={COMMON}&cutoff=2")).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(capped[0]["cutoff_used"], true);
}

#[tokio::test(flavor = "multi_thread")]
async fn equate_il_widens_which_proteins_are_reached() {
    let (_, apart) = get_json(&format!("/api/v2/pept2interpro?input[]={IL_ISOLEUCINE}&equate_il=false")).await;
    let (status, together) = get_json(&format!("/api/v2/pept2interpro?input[]={IL_ISOLEUCINE}&equate_il=true")).await;

    assert_eq!(status, StatusCode::OK);
    assert!(together[0]["total_protein_count"].as_u64() > apart[0]["total_protein_count"].as_u64());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_repeated_peptide_answers_at_each_position() {
    super::a_repeat_answers_like_a_single("pept2interpro", "extra=true&domains=true").await;
}

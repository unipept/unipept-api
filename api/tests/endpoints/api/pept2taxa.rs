//! `/api/v2/pept2taxa` — every taxon a peptide reaches, in either of two response shapes.

use axum::http::StatusCode;
use fixtures::peptides::*;

use crate::common::get_json;

#[tokio::test(flavor = "multi_thread")]
async fn a_low_cutoff_is_reported_on_the_result() {
    let (_, uncapped) = get_json(&format!("/api/v2/pept2taxa?input[]={COMMON}")).await;
    let (status, capped) = get_json(&format!("/api/v2/pept2taxa?input[]={COMMON}&cutoff=2")).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(uncapped[0]["cutoff_used"], false);
    assert_eq!(capped[0]["cutoff_used"], true, "seven matching proteins against a cutoff of two");
}

/// The endpoint answers in two shapes and `compact` picks between them: a row per taxon by
/// default, or a row per peptide carrying a list. Both must name the same taxa.
#[tokio::test(flavor = "multi_thread")]
async fn pept2taxa_lists_every_taxon_a_peptide_reaches() {
    let (status, dense) = get_json(&format!("/api/v2/pept2taxa?input[]={GENUS_SHARED}")).await;
    assert_eq!(status, StatusCode::OK);

    let mut from_dense: Vec<u64> = dense
        .as_array()
        .expect("a row per taxon")
        .iter()
        .map(|row| row["taxon_id"].as_u64().unwrap_or_else(|| panic!("each row names its taxon: {row}")))
        .collect();
    from_dense.sort_unstable();
    assert_eq!(from_dense, vec![8501, 8502]);

    let (status, compact) = get_json(&format!("/api/v2/pept2taxa?input[]={GENUS_SHARED}&compact=true")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(compact[0]["peptide"], GENUS_SHARED);

    let mut from_compact: Vec<u64> = compact[0]["taxa"]
        .as_array()
        .unwrap_or_else(|| panic!("compact should carry a taxa list: {}", compact[0]))
        .iter()
        .map(|taxon| taxon.as_u64().expect("an id"))
        .collect();
    from_compact.sort_unstable();
    assert_eq!(from_compact, from_dense, "the two shapes must agree about which taxa were found");
}

/// `tryptic` narrows a search to matches at a tryptic boundary, so it can only ever return a
/// subset of the same search without it. A metamorphic assertion, since the rules themselves are
/// the index's business rather than this endpoint's.
#[tokio::test(flavor = "multi_thread")]
async fn tryptic_results_are_a_subset_of_untryptic_ones() {
    let (_, all) = get_json(&format!("/api/v2/pept2taxa?input[]={COMMON}&compact=true")).await;
    let (status, tryptic) = get_json(&format!("/api/v2/pept2taxa?input[]={COMMON}&compact=true&tryptic=true")).await;

    assert_eq!(status, StatusCode::OK);

    let taxa = |value: &serde_json::Value| -> std::collections::BTreeSet<u64> {
        value[0]["taxa"]
            .as_array()
            .map(|list| list.iter().filter_map(serde_json::Value::as_u64).collect())
            .unwrap_or_default()
    };
    let (all, tryptic) = (taxa(&all), taxa(&tryptic));

    assert!(!all.is_empty(), "the common peptide must reach something, or this proves nothing");
    assert!(tryptic.is_subset(&all), "tryptic taxa not present unfiltered: {:?}", &tryptic - &all);
}

/// `extra` and `names` add the lineage to each row of the dense shape.
#[tokio::test(flavor = "multi_thread")]
async fn extra_and_names_add_a_named_lineage() {
    let (_, plain) = get_json(&format!("/api/v2/pept2taxa?input[]={UNIQUE}")).await;
    assert!(plain[0].get("genus_id").is_none());

    let (status, named) = get_json(&format!("/api/v2/pept2taxa?input[]={UNIQUE}&extra=true&names=true")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(named[0]["genus_name"], "Crocodylus");
}

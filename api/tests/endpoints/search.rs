//! What every peptide-search endpoint shares.

use axum::http::StatusCode;
use fixtures::peptides::*;

use crate::common::get_json;

/// Every search endpoint takes the same peptide list, so one batch through all of them is the
/// cheapest check that none of them chokes on more than one input.
#[tokio::test(flavor = "multi_thread")]
async fn every_search_endpoint_accepts_a_batch() {
    let batch = format!("input[]={UNIQUE}&input[]={GENUS_SHARED}&input[]={ABSENT}");

    // pept2taxa is left out of the count: it answers with a row per taxon rather than per peptide,
    // so its length is a different quantity. Its own test covers both of its shapes.
    for endpoint in ["pept2lca", "pept2ec", "pept2go", "pept2interpro", "pept2funct", "peptinfo"] {
        let (status, body) = get_json(&format!("/api/v2/{endpoint}?{batch}")).await;
        assert_eq!(status, StatusCode::OK, "{endpoint}");
        assert_eq!(body.as_array().map(Vec::len), Some(2), "{endpoint} should answer for the two that matched");
    }

    let (status, taxa) = get_json(&format!("/api/v2/pept2taxa?{batch}")).await;
    assert_eq!(status, StatusCode::OK);
    assert!(!taxa.as_array().expect("rows").is_empty(), "pept2taxa should answer for the peptides that matched");
}

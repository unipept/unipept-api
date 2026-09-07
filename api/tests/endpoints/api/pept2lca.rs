//! `/api/v2/pept2lca` — the lowest common ancestor of the taxa a peptide reaches.
//!
//! Every flag this endpoint takes has a marker peptide in the corpus placed to make its effect
//! observable, so these assert differences rather than fixed values.

use axum::http::StatusCode;
use fixtures::peptides::*;

use crate::common::get_json;

#[tokio::test(flavor = "multi_thread")]
async fn pept2lca_resolves_a_unique_peptide_to_its_species() {
    let (status, body) = get_json(&format!("/api/v2/pept2lca?input[]={UNIQUE}")).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body[0]["peptide"], UNIQUE);
    assert_eq!(body[0]["taxon_id"], 8501);
    assert_eq!(body[0]["taxon_name"], "Crocodylus niloticus");
    assert_eq!(body[0]["taxon_rank"], "species");
}

#[tokio::test(flavor = "multi_thread")]
async fn pept2lca_reduces_a_shared_peptide_to_the_genus() {
    let (status, body) = get_json(&format!("/api/v2/pept2lca?input[]={GENUS_SHARED}")).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body[0]["taxon_id"], 8500);
    assert_eq!(body[0]["taxon_name"], "Crocodylus");
}

/// The two lineages diverge at class, where one of them records nothing, so reaching their common
/// ancestor means stepping over a rank rather than matching on it.
#[tokio::test(flavor = "multi_thread")]
async fn pept2lca_steps_over_a_rank_one_lineage_omits() {
    let (status, body) = get_json(&format!("/api/v2/pept2lca?input[]={SUPERCLASS_SHARED}")).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body[0]["taxon_id"], 8287);
    assert_eq!(body[0]["taxon_rank"], "superclass");
}

#[tokio::test(flavor = "multi_thread")]
async fn pept2lca_reduces_across_domains_to_root() {
    let (status, body) = get_json(&format!("/api/v2/pept2lca?input[]={ROOT_SHARED}")).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body[0]["taxon_id"], 1);
}

/// One flag, one observable difference: the same peptide reaches one species or two, and the LCA
/// moves from the species to the genus above it.
#[tokio::test(flavor = "multi_thread")]
async fn equate_il_moves_the_answer_from_a_species_to_its_genus() {
    let (_, apart) = get_json(&format!("/api/v2/pept2lca?input[]={IL_ISOLEUCINE}&equate_il=false")).await;
    let (status, together) = get_json(&format!("/api/v2/pept2lca?input[]={IL_ISOLEUCINE}&equate_il=true")).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(apart[0]["taxon_id"], 8502, "on its own the peptide finds Crocodylus porosus");
    assert_eq!(together[0]["taxon_id"], 8500, "equated it finds both species, so the LCA is the genus");
}

/// The corpus has exactly one taxon the taxonomy marks invalid, and one peptide reaching it.
#[tokio::test(flavor = "multi_thread")]
async fn validate_taxa_drops_the_invalid_taxon() {
    let (_, unvalidated) = get_json(&format!("/api/v2/pept2lca?input[]={VALIDATION_SHARED}&validate_taxa=false")).await;
    let (status, validated) =
        get_json(&format!("/api/v2/pept2lca?input[]={VALIDATION_SHARED}&validate_taxa=true")).await;

    assert_eq!(status, StatusCode::OK);
    assert_ne!(unvalidated[0]["taxon_id"], validated[0]["taxon_id"], "the flag must change the answer");
    assert_eq!(validated[0]["taxon_id"], 8501, "with the invalid taxon dropped only C. niloticus remains");
}

/// `extra` and `names` add the lineage and expand its names.
#[tokio::test(flavor = "multi_thread")]
async fn extra_and_names_add_a_named_lineage() {
    let (status, body) = get_json(&format!("/api/v2/pept2lca?input[]={UNIQUE}&extra=true&names=true")).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body[0]["genus_id"], 8500);
    assert_eq!(body[0]["genus_name"], "Crocodylus");
    assert_eq!(body[0]["domain_name"], "Eukaryota");
}

/// A peptide the index does not hold produces no row at all — not a row with an empty answer.
#[tokio::test(flavor = "multi_thread")]
async fn an_absent_peptide_produces_no_row() {
    let (status, body) = get_json(&format!("/api/v2/pept2lca?input[]={ABSENT}&input[]={UNIQUE}")).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().map(Vec::len), Some(1), "only the peptide that matched appears");
    assert_eq!(body[0]["peptide"], UNIQUE);
}

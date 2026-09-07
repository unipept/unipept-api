//! The thirteen endpoints that read only the datastore.
//!
//! No index search and no OpenSearch, so the state points at an address nothing listens on and
//! every answer comes out of the corpus tables. These are the cheapest endpoints to cover and half
//! of everything the API exposes.

mod common;

use axum::http::StatusCode;
use common::{get_json, post_json};
use serde_json::json;

// ── the private_api lookups ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn metadata_reports_the_index_version() {
    let (status, body) = get_json("/private_api/metadata").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["db_version"], "2026.09-fixtures");
}

/// EC keys are stored bare and the caller strips `EC:` before looking them up, so this is the
/// endpoint where a store keyed the other way would return a name of `""` rather than an error.
#[tokio::test(flavor = "multi_thread")]
async fn ecnumbers_resolves_a_code_to_its_name() {
    let (status, body) = get_json("/private_api/ecnumbers?ecnumbers[]=1.1.1.1").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body[0]["code"], "1.1.1.1");
    assert_eq!(body[0]["name"], "Alcohol dehydrogenase");
}

/// GO is the one store whose keys keep their prefix.
#[tokio::test(flavor = "multi_thread")]
async fn goterms_resolves_a_code_to_its_name_and_namespace() {
    let (status, body) = get_json("/private_api/goterms?goterms[]=GO:0009279").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body[0]["code"], "GO:0009279");
    assert_eq!(body[0]["name"], "cell outer membrane");
    assert_eq!(body[0]["namespace"], "cellular component");
}

#[tokio::test(flavor = "multi_thread")]
async fn interpros_resolves_a_code_to_its_name_and_category() {
    let (status, body) = get_json("/private_api/interpros?interpros[]=IPR016364").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body[0]["code"], "IPR016364");
    assert_eq!(body[0]["name"], "Alcohol dehydrogenase, zinc-type");
    assert_eq!(body[0]["category"], "Family");
}

#[tokio::test(flavor = "multi_thread")]
async fn taxa_resolves_an_id_to_its_name_rank_and_lineage() {
    let (status, body) = get_json("/private_api/taxa?taxids[]=8501").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body[0]["id"], 8501);
    assert_eq!(body[0]["name"], "Crocodylus niloticus");
    assert_eq!(body[0]["rank"], "species");
    assert_eq!(body[0]["lineage"].as_array().expect("a lineage").len(), 28);
}

#[tokio::test(flavor = "multi_thread")]
async fn proteomes_resolves_an_accession_to_its_taxon_and_count() {
    let (status, body) = get_json("/private_api/proteomes?proteomes[]=UP000000001").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body[0]["id"], "UP000000001");
    assert_eq!(body[0]["taxon_id"], 8501);
    assert_eq!(body[0]["taxon_name"], "Crocodylus niloticus");
    assert_eq!(body[0]["protein_count"], 3);
}

/// An id the corpus has never heard of is an omission, not a failure.
#[tokio::test(flavor = "multi_thread")]
async fn unknown_identifiers_are_left_out_rather_than_erroring() {
    for path in [
        "/private_api/ecnumbers?ecnumbers[]=9.9.9.9",
        "/private_api/goterms?goterms[]=GO:0000000",
        "/private_api/interpros?interpros[]=IPR999999",
        "/private_api/taxa?taxids[]=999999999",
        "/private_api/proteomes?proteomes[]=UP999999999"
    ] {
        let (status, body) = get_json(path).await;
        assert_eq!(status, StatusCode::OK, "{path}");
        assert_eq!(body.as_array().map(Vec::len), Some(0), "{path} should answer with an empty list");
    }
}

// ── taxa2rank ───────────────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn taxa2rank_maps_two_species_onto_their_shared_genus() {
    let (status, body) = post_json("/private_api/taxa2rank", json!({ "taxa": [[8501, 8502]], "rank": "genus" })).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["mapped_taxa"], json!([[8500]]), "both crocodiles reduce to Crocodylus, deduplicated");
}

#[tokio::test(flavor = "multi_thread")]
async fn taxa2rank_rejects_a_rank_it_does_not_know() {
    let (_dir, state) = common::offline_state();
    let request = axum::http::Request::post("/private_api/taxa2rank")
        .header(axum::http::header::CONTENT_TYPE, "application/json")
        .body(axum::body::Body::from(json!({ "taxa": [[8501]], "rank": "not a rank" }).to_string()))
        .unwrap();

    let (status, body) = common::request_raw(state, request).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body.contains("Invalid rank"), "the message should name the problem: {body}");
}

// ── the counting and paging endpoints ───────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn proteome_and_taxon_counts_match_the_corpus() {
    let (status, proteomes) = get_json("/private_api/proteomes/count").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(proteomes["count"], 3, "the corpus declares three reference proteomes");

    // Every taxon that is both valid and ranked: the twenty-six rows less the invalid one and less
    // root, which carries no rank.
    let (status, taxa) = get_json("/private_api/taxa/count").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(taxa["count"], 24);
}

#[tokio::test(flavor = "multi_thread")]
async fn filtering_narrows_the_count() {
    let (_, all) = get_json("/private_api/taxa/count").await;
    let (status, crocodiles) = get_json("/private_api/taxa/count?filter=Crocodylus").await;

    assert_eq!(status, StatusCode::OK);
    assert!(
        crocodiles["count"].as_u64() < all["count"].as_u64(),
        "a filter must select fewer than everything: {crocodiles} vs {all}"
    );
    assert!(crocodiles["count"].as_u64().unwrap() >= 4, "the genus and its three species at least");
}

#[tokio::test(flavor = "multi_thread")]
async fn paging_returns_at_most_the_window_it_was_given() {
    let (status, page) = get_json("/private_api/taxa/filter?start=0&end=3").await;

    assert_eq!(status, StatusCode::OK);
    assert!(page.as_array().expect("a page").len() <= 3, "a window of three must not return more");

    let (status, proteomes) = get_json("/private_api/proteomes/filter?start=0&end=10").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(proteomes.as_array().expect("a page").len(), 3);
}

// ── the api/v2 endpoints that need no index ─────────────────────────────────────────────────────

/// The taxonomy half of `pept2lca`, reachable without a peptide search.
#[tokio::test(flavor = "multi_thread")]
async fn taxa2lca_reduces_two_species_to_their_genus() {
    let (status, body) = get_json("/api/v2/taxa2lca?input[]=8501&input[]=8502").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["taxon_id"], 8500);
    assert_eq!(body["taxon_name"], "Crocodylus");
    assert_eq!(body["taxon_rank"], "genus");
}

#[tokio::test(flavor = "multi_thread")]
async fn taxa2lca_reduces_across_domains_to_root() {
    let (status, body) = get_json("/api/v2/taxa2lca?input[]=8501&input[]=7").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["taxon_id"], 1, "a crocodile and a bacterium share only the root");
}

#[tokio::test(flavor = "multi_thread")]
async fn taxonomy_names_a_taxon() {
    let (status, body) = get_json("/api/v2/taxonomy?input[]=8501").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body[0]["taxon_id"], 8501);
    assert_eq!(body[0]["taxon_name"], "Crocodylus niloticus");
    assert_eq!(body[0]["taxon_rank"], "species");
}

/// The descendants branch, which is most of what `taxonomy` does.
///
/// A genus asked for its species finds the three the corpus places under it. The lookup goes
/// through `LineageRank`'s string form, and single-word ranks survive that round trip — the
/// multi-word ones are issue #148, which this corpus cannot reach.
#[tokio::test(flavor = "multi_thread")]
async fn taxonomy_finds_the_descendants_of_a_genus() {
    let (status, body) = get_json("/api/v2/taxonomy?input[]=8500&descendants=true&descendants_ranks[]=species").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body[0]["taxon_id"], 8500);

    let mut descendants: Vec<u64> = body[0]["descendants"]
        .as_array()
        .unwrap_or_else(|| panic!("a genus asked for descendants should list them: {}", body[0]))
        .iter()
        .map(|taxon| taxon.as_u64().expect("a taxon id"))
        .collect();
    descendants.sort_unstable();

    assert_eq!(descendants, vec![8501, 8502, 8503], "the three Crocodylus species");
}

/// Asking for descendants at a rank nothing sits at is an empty list, not an error.
#[tokio::test(flavor = "multi_thread")]
async fn taxonomy_descendants_at_an_empty_rank_are_empty() {
    let (status, body) = get_json("/api/v2/taxonomy?input[]=8500&descendants=true&descendants_ranks[]=forma").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body[0]["descendants"].as_array().map(Vec::len), Some(0));
}

#[tokio::test(flavor = "multi_thread")]
async fn taxa2tree_builds_a_tree_rooted_at_the_organism() {
    let (status, body) = get_json("/api/v2/taxa2tree?input[]=8501&input[]=8502").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["id"], 1, "the tree is rooted at taxon 1");
    assert!(body["children"].as_array().is_some_and(|c| !c.is_empty()), "root should have children: {body}");
}

/// The one route that answers with a rendered template rather than JSON.
#[tokio::test(flavor = "multi_thread")]
async fn taxa2tree_html_renders_a_document() {
    let (_dir, state) = common::offline_state();
    let request = axum::http::Request::get("/api/v2/taxa2tree.html?input[]=8501&input[]=8502")
        .body(axum::body::Body::empty())
        .unwrap();

    let (status, body) = common::request_raw(state, request).await;

    assert_eq!(status, StatusCode::OK);
    assert!(
        body.contains("<html") || body.contains("<!DOCTYPE"),
        "expected a document: {}",
        &body[..body.len().min(120)]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn sampledata_returns_the_configured_datasets() {
    let (status, body) = get_json("/datasets/sampledata").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["sample_data"][0]["environment"], "Fixture environment");
}

// ── both halves of a route ──────────────────────────────────────────────────────────────────────

/// Every route is registered for GET and POST through the same macro, and the two reach the same
/// handler by different extractors. One endpoint checked both ways is enough to catch that pairing
/// coming undone.
#[tokio::test(flavor = "multi_thread")]
async fn get_and_post_answer_alike() {
    let (get_status, from_get) = get_json("/private_api/taxa?taxids[]=8501").await;
    let (post_status, from_post) = post_json("/private_api/taxa", json!({ "taxids": [8501] })).await;

    assert_eq!(get_status, StatusCode::OK);
    assert_eq!(post_status, StatusCode::OK);
    assert_eq!(from_get, from_post);
}

/// The `.json` suffix every route also registers.
#[tokio::test(flavor = "multi_thread")]
async fn the_json_suffix_reaches_the_same_handler() {
    let (plain_status, plain) = get_json("/private_api/metadata").await;
    let (suffixed_status, suffixed) = get_json("/private_api/metadata.json").await;

    assert_eq!(plain_status, suffixed_status);
    assert_eq!(plain, suffixed);
}

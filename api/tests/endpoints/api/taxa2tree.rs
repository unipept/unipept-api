//! `/api/v2/taxa2tree` — a taxonomic tree over a set of taxa, as JSON or as a rendered page.
//!
//! Its GET and POST halves take different parameters: a list of taxa, or a map of taxon to count.

use axum::{
    body::Body,
    http::{Request, StatusCode}
};
use serde_json::json;

use crate::common::{get_json, offline_state, post_json, request_raw};

async fn get_raw(path: &str) -> (StatusCode, String) {
    let (dir, state) = offline_state();
    let answered = request_raw(state, Request::get(path).body(Body::empty()).unwrap()).await;
    drop(dir);
    answered
}

#[tokio::test(flavor = "multi_thread")]
async fn the_tree_is_rooted_at_the_organism() {
    let (status, body) = get_json("/api/v2/taxa2tree?input[]=8501&input[]=8502").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["id"], 1);
    assert_eq!(body["name"], "Organism");
    assert!(!body["children"].as_array().expect("children").is_empty());
}

/// Two species of one genus share every rung above it, so the genus carries both.
#[tokio::test(flavor = "multi_thread")]
async fn taxa_sharing_a_lineage_share_its_nodes() {
    let (_, body) = get_json("/api/v2/taxa2tree?input[]=8501&input[]=8502").await;

    // Walk down to the genus and check both species hang off it.
    let mut node = &body;
    while let Some(children) = node["children"].as_array() {
        if node["id"] == 8500 {
            break;
        }
        match children.first() {
            Some(child) if !children.is_empty() => node = child,
            _ => break
        }
    }
    assert_eq!(node["id"], 8500, "the walk should reach the genus: {body}");
    assert_eq!(node["children"].as_array().map(Vec::len), Some(2), "with both species beneath it");
}

/// The counts a POST carries end up on the nodes, which is what the GET half cannot express.
#[tokio::test(flavor = "multi_thread")]
async fn a_post_carries_counts_per_taxon() {
    let (status, body) = post_json("/api/v2/taxa2tree", json!({ "counts": { "8501": 3, "8502": 5 } })).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["count"], 8, "root aggregates the counts beneath it");
}

/// `link` does not decorate the tree — it replaces it. The response becomes a different variant
/// carrying a gist reference instead of any taxonomy at all.
///
/// That gist is the literal string `"test"`, hardcoded in the handler rather than produced from
/// anything. Asserted as it is so the stub is recorded rather than mistaken for a feature.
#[tokio::test(flavor = "multi_thread")]
async fn link_returns_a_gist_reference_instead_of_a_tree() {
    let (status, linked) = get_json("/api/v2/taxa2tree?input[]=8501&link=true").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(linked["gist"], "test", "the handler returns a placeholder, not a real link");
    assert!(linked.get("children").is_none(), "and no tree comes back at all");

    let (status, unlinked) = get_json("/api/v2/taxa2tree?input[]=8501&link=false").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(unlinked["id"], 1, "without the flag it is the tree");
}

/// An ancestor the taxon table does not name still becomes a node, with an empty name, so the
/// branch below it hangs in the right place. This used to be an unwrap.
#[tokio::test(flavor = "multi_thread")]
async fn an_unnamed_ancestor_does_not_break_the_tree() {
    let (status, body) = get_json("/api/v2/taxa2tree?input[]=8501&input[]=9503").await;

    assert_eq!(status, StatusCode::OK);
    assert!(!body["children"].as_array().expect("children").is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn no_input_still_builds_a_root() {
    let (status, body) = get_json("/api/v2/taxa2tree").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["id"], 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn the_html_route_renders_a_document() {
    let (status, body) = get_raw("/api/v2/taxa2tree.html?input[]=8501&input[]=8502").await;

    assert_eq!(status, StatusCode::OK);
    assert!(
        body.contains("<html") || body.contains("<!DOCTYPE"),
        "expected a document, got: {}",
        &body[..body.len().min(120)]
    );
}

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

/// Pulls the value the page assigns to `data` back out of the rendered document.
///
/// The terminator is the semicolon at the end of a line, not the first semicolon: taxon names are
/// free text and JSON escapes newlines but not semicolons, so a bare `;` could sit inside the data
/// itself. Both line endings are accepted — the repository sets no `.gitattributes`, so a checkout
/// with `core.autocrlf` on renders the template with CRLF.
fn rendered_data(body: &str) -> &str {
    let start = body.find("const data = ").expect("the page assigns const data") + "const data = ".len();
    let rest = &body[start..];
    let end = rest.find(";\r\n").or_else(|| rest.find(";\n")).expect("the assignment is terminated");
    &rest[..end]
}

/// The tree reaches the page as JSON the browser can parse.
///
/// The document is otherwise static, so `<html>` being present says only that a file was read from
/// disk: the page would still contain it with the tree missing, empty, or HTML-escaped into
/// `&quot;` sequences that no browser can `JSON.parse`. This is the assertion that the one value
/// the template interpolates actually arrives, and arrives intact.
#[tokio::test(flavor = "multi_thread")]
async fn the_html_route_embeds_the_tree_as_parseable_json() {
    let (_, body) = get_raw("/api/v2/taxa2tree.html?input[]=8501&input[]=8502").await;

    let data = rendered_data(&body);
    assert!(!data.is_empty(), "the page embedded no tree at all");

    let embedded: serde_json::Value =
        serde_json::from_str(data).unwrap_or_else(|err| panic!("embedded data was not JSON ({err}): {data}"));

    assert_eq!(embedded["id"], 1, "the embedded tree should be rooted at the organism");
    assert_eq!(embedded["name"], "Organism");
}

/// Sorts every `children` array by `id`, so two trees can be compared by content.
///
/// Siblings come out in whatever order the handler's map iterated, which differs between calls.
/// That is not what this test is about, so it is normalised away rather than asserted.
fn sorted_children(value: &mut serde_json::Value) {
    if let Some(children) = value.get_mut("children").and_then(serde_json::Value::as_array_mut) {
        for child in children.iter_mut() {
            sorted_children(child);
        }
        children.sort_by_key(|child| child["id"].as_i64().expect("every node carries a numeric id"));
    }
}

/// The embedded tree is the same one the JSON route serves.
///
/// Two routes, one handler: the HTML half differs only in wrapping the tree in a template. Pinning
/// them equal means a change to either the serialisation or the interpolation shows up here rather
/// than as a page that renders and quietly draws something else.
#[tokio::test(flavor = "multi_thread")]
async fn the_embedded_tree_matches_the_json_route() {
    let (_, mut from_json) = get_json("/api/v2/taxa2tree?input[]=8501&input[]=8502").await;
    let (_, page) = get_raw("/api/v2/taxa2tree.html?input[]=8501&input[]=8502").await;

    let mut embedded: serde_json::Value = serde_json::from_str(rendered_data(&page)).expect("embedded data is JSON");

    sorted_children(&mut embedded);
    sorted_children(&mut from_json);

    assert_eq!(embedded, from_json);
}

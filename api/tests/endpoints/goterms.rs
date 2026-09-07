//! `/private_api/goterms` — GO codes to names and namespaces.
//!
//! GO is the exception among the three annotation stores: its keys keep the `GO:` prefix that EC
//! and InterPro drop.

use axum::http::StatusCode;
use serde_json::json;

use crate::common::{get_json, post_json};

#[tokio::test(flavor = "multi_thread")]
async fn one_code_resolves_to_its_name_and_namespace() {
    let (status, body) = get_json("/private_api/goterms?goterms[]=GO:0009279").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body[0]["code"], "GO:0009279");
    assert_eq!(body[0]["name"], "cell outer membrane");
    assert_eq!(body[0]["namespace"], "cellular component");
}

#[tokio::test(flavor = "multi_thread")]
async fn several_codes_resolve_in_one_call() {
    let (status, body) = get_json("/private_api/goterms?goterms[]=GO:0009279&goterms[]=GO:0005515").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().map(Vec::len), Some(2));
}

/// The mirror of the EC case: stripping the prefix here is what breaks the lookup.
#[tokio::test(flavor = "multi_thread")]
async fn a_bare_code_matches_nothing() {
    let (status, body) = get_json("/private_api/goterms?goterms[]=0009279").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, json!([]));
}

#[tokio::test(flavor = "multi_thread")]
async fn an_unknown_code_is_omitted_rather_than_failing() {
    let (status, body) = get_json("/private_api/goterms?goterms[]=GO:0000000&goterms[]=GO:0009279").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().map(Vec::len), Some(1));
}

#[tokio::test(flavor = "multi_thread")]
async fn no_codes_is_an_empty_answer() {
    assert_eq!(get_json("/private_api/goterms").await.1, json!([]));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_post_answers_like_a_get() {
    let (_, from_get) = get_json("/private_api/goterms?goterms[]=GO:0009279").await;
    let (status, from_post) = post_json("/private_api/goterms", json!({ "goterms": ["GO:0009279"] })).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(from_post, from_get);
}

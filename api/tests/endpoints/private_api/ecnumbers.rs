//! `/private_api/ecnumbers` — EC codes to names.
//!
//! Keys are stored bare: the annotations carry `EC:1.1.1.1` and the caller strips the prefix before
//! the lookup. A store keyed the other way answers with an empty name rather than an error, which
//! is why the name is asserted and not just the presence of a row.

use axum::http::StatusCode;
use serde_json::json;

use crate::common::{get_json, post_json};

#[tokio::test(flavor = "multi_thread")]
async fn one_code_resolves_to_its_name() {
    let (status, body) = get_json("/private_api/ecnumbers?ecnumbers[]=1.1.1.1").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body[0]["code"], "1.1.1.1");
    assert_eq!(body[0]["name"], "Alcohol dehydrogenase");
}

#[tokio::test(flavor = "multi_thread")]
async fn several_codes_resolve_in_one_call() {
    let (status, body) = get_json("/private_api/ecnumbers?ecnumbers[]=1.1.1.1&ecnumbers[]=2.7.11.1").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().map(Vec::len), Some(2));
}

/// A prefixed key is not what the store holds, so it resolves to nothing — the failure mode a
/// caller that forgot to strip `EC:` would hit.
#[tokio::test(flavor = "multi_thread")]
async fn a_prefixed_code_matches_nothing() {
    let (status, body) = get_json("/private_api/ecnumbers?ecnumbers[]=EC:1.1.1.1").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, json!([]));
}

#[tokio::test(flavor = "multi_thread")]
async fn an_unknown_code_is_omitted_rather_than_failing() {
    let (status, body) = get_json("/private_api/ecnumbers?ecnumbers[]=9.9.9.9&ecnumbers[]=1.1.1.1").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().map(Vec::len), Some(1), "the known one still comes back");
    assert_eq!(body[0]["code"], "1.1.1.1");
}

#[tokio::test(flavor = "multi_thread")]
async fn no_codes_is_an_empty_answer() {
    let (status, body) = get_json("/private_api/ecnumbers").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, json!([]));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_post_answers_like_a_get() {
    let (_, from_get) = get_json("/private_api/ecnumbers?ecnumbers[]=1.1.1.1").await;
    let (status, from_post) = post_json("/private_api/ecnumbers", json!({ "ecnumbers": ["1.1.1.1"] })).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(from_post, from_get);
}

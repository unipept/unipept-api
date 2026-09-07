//! `/private_api/interpros` — InterPro codes to names and categories.
//!
//! Keys are stored bare, like EC and unlike GO.

use axum::http::StatusCode;
use serde_json::json;

use crate::common::{get_json, post_json};

#[tokio::test(flavor = "multi_thread")]
async fn one_code_resolves_to_its_name_and_category() {
    let (status, body) = get_json("/private_api/interpros?interpros[]=IPR016364").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body[0]["code"], "IPR016364");
    assert_eq!(body[0]["name"], "Alcohol dehydrogenase, zinc-type");
    assert_eq!(body[0]["category"], "Family");
}

#[tokio::test(flavor = "multi_thread")]
async fn several_codes_resolve_in_one_call() {
    let (status, body) = get_json("/private_api/interpros?interpros[]=IPR016364&interpros[]=IPR008816").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().map(Vec::len), Some(2));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_prefixed_code_matches_nothing() {
    assert_eq!(get_json("/private_api/interpros?interpros[]=IPR:IPR016364").await.1, json!([]));
}

#[tokio::test(flavor = "multi_thread")]
async fn an_unknown_code_is_omitted_rather_than_failing() {
    let (status, body) = get_json("/private_api/interpros?interpros[]=IPR999999&interpros[]=IPR016364").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().map(Vec::len), Some(1));
}

#[tokio::test(flavor = "multi_thread")]
async fn no_codes_is_an_empty_answer() {
    assert_eq!(get_json("/private_api/interpros").await.1, json!([]));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_post_answers_like_a_get() {
    let (_, from_get) = get_json("/private_api/interpros?interpros[]=IPR016364").await;
    let (status, from_post) = post_json("/private_api/interpros", json!({ "interpros": ["IPR016364"] })).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(from_post, from_get);
}

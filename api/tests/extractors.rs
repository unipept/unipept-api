//! The request extractors, driven directly.
//!
//! `GetContent` and `PostContent` are generic over `S: Send + Sync`, so `()` serves as the state
//! and none of this needs an `AppState`, an index or a database. That is what makes these the
//! cheapest tests in the workspace to run and the closest to where malformed input actually
//! arrives.

use axum::{
    body::Body,
    extract::{FromRequest, FromRequestParts, Request},
    http::{StatusCode, header::CONTENT_TYPE},
    response::IntoResponse
};
use serde::Deserialize;
use unipept_api::controllers::request::{GetContent, PostContent};

#[derive(Debug, Deserialize, PartialEq)]
struct Parameters {
    #[serde(default)]
    input: Vec<String>,
    #[serde(default)]
    equate_il: bool
}

async fn get(query: &str) -> Result<Parameters, StatusCode> {
    let request = Request::builder().uri(format!("/pept2lca?{query}")).body(Body::empty()).unwrap();
    let (mut parts, _) = request.into_parts();

    GetContent::<Parameters>::from_request_parts(&mut parts, &())
        .await
        .map(|GetContent(parameters)| parameters)
        .map_err(|(status, _)| status)
}

async fn post(content_type: &str, body: &'static str) -> Result<Parameters, StatusCode> {
    let request = Request::builder()
        .method("POST")
        .uri("/pept2lca")
        .header(CONTENT_TYPE, content_type)
        .body(Body::from(body))
        .unwrap();

    PostContent::<Parameters>::from_request(request, &())
        .await
        .map(|PostContent(parameters)| parameters)
        .map_err(|rejection| rejection.into_response().status())
}

#[tokio::test]
async fn a_query_string_parses() {
    let parameters = get("input[]=AALTER&input[]=AAKNER&equate_il=true").await.expect("a valid query string parses");

    assert_eq!(parameters.input, vec!["AALTER", "AAKNER"]);
    assert!(parameters.equate_il);
}

/// The panic this change removes.
///
/// `%FF` is a syntactically valid percent escape whose byte is not valid UTF-8. Decoding it failed,
/// the failure was unwrapped, and the request took the handler down instead of returning a status.
/// It is reachable by anyone who can type a URL.
#[tokio::test]
async fn a_query_string_that_is_not_utf8_is_rejected_not_a_panic() {
    assert_eq!(get("input=%FF").await, Err(StatusCode::BAD_REQUEST));
    assert_eq!(get("input[]=%C3%28").await, Err(StatusCode::BAD_REQUEST));
}

#[tokio::test]
async fn an_empty_query_string_yields_the_defaults() {
    let parameters = get("").await.expect("an empty query string is not an error");

    assert!(parameters.input.is_empty());
    assert!(!parameters.equate_il);
}

#[tokio::test]
async fn json_form_and_multipart_bodies_all_parse_to_the_same_parameters() {
    let expected = Parameters { input: vec!["AALTER".to_string()], equate_il: true };

    let json = post("application/json", r#"{"input":["AALTER"],"equate_il":true}"#).await;
    assert_eq!(json.expect("json parses"), expected);

    let form = post("application/x-www-form-urlencoded", "input[]=AALTER&equate_il=true").await;
    assert_eq!(form.expect("urlencoded parses"), expected);

    let multipart = post(
        "multipart/form-data; boundary=X",
        "--X\r\nContent-Disposition: form-data; name=\"input[]\"\r\n\r\nAALTER\r\n\
         --X\r\nContent-Disposition: form-data; name=\"equate_il\"\r\n\r\ntrue\r\n\
         --X--\r\n"
    )
    .await;
    assert_eq!(multipart.expect("multipart parses"), expected);
}

/// A body that ends mid-part used to unwrap inside the field loop.
///
/// The status is 422 rather than the 400 the multipart extractor itself produces: `PostContent`
/// maps every rejection from its three branches to one code. That flattening is left alone here —
/// this change is about the request not killing the handler, not about the status taxonomy.
#[tokio::test]
async fn a_truncated_multipart_body_is_rejected_not_a_panic() {
    let truncated =
        post("multipart/form-data; boundary=X", "--X\r\nContent-Disposition: form-data; name=\"input[]\"\r\n").await;

    assert_eq!(truncated, Err(StatusCode::UNPROCESSABLE_ENTITY));
}

/// A part with no `name` cannot contribute to the query string the extractor builds, and dropping
/// it silently would lose data the client sent. It is rejected instead.
#[tokio::test]
async fn a_multipart_field_without_a_name_is_rejected() {
    let unnamed =
        post("multipart/form-data; boundary=X", "--X\r\nContent-Disposition: form-data\r\n\r\nAALTER\r\n--X--\r\n")
            .await;

    assert_eq!(unnamed, Err(StatusCode::UNPROCESSABLE_ENTITY));
}

#[tokio::test]
async fn an_unparseable_body_is_rejected() {
    assert_eq!(post("application/json", "{not json").await, Err(StatusCode::UNPROCESSABLE_ENTITY));
}

#[tokio::test]
async fn a_content_type_the_extractor_does_not_handle_is_unsupported_media_type() {
    assert_eq!(post("text/plain", "input[]=AALTER").await, Err(StatusCode::UNSUPPORTED_MEDIA_TYPE));
    assert_eq!(post("", "input[]=AALTER").await, Err(StatusCode::UNSUPPORTED_MEDIA_TYPE));
}

/// A separator encoded inside a value stays inside that value.
///
/// The query string used to be percent-decoded in full before `serde_qs` split it, so an encoded
/// `&` became a real one and everything after it turned into parameters of its own. A request
/// sending a single value could set flags it never mentioned.
#[tokio::test]
async fn an_encoded_separator_does_not_become_a_parameter() {
    let parameters = get("input[]=A%26equate_il%3Dtrue").await.expect("the value parses");

    assert_eq!(parameters.input, vec!["A&equate_il=true"]);
    assert!(!parameters.equate_il, "a flag the request never sent must not be set");
}

/// The same, on the form-encoded body path, which decoded its bytes the same way.
#[tokio::test]
async fn an_encoded_separator_in_a_form_body_does_not_become_a_parameter() {
    let parameters = post("application/x-www-form-urlencoded", "input[]=A%26equate_il%3Dtrue")
        .await
        .expect("the value parses");

    assert_eq!(parameters.input, vec!["A&equate_il=true"]);
    assert!(!parameters.equate_il);
}

/// Ordinary encodings are unaffected: `%20` and `+` are still spaces, `%25` still a percent sign.
#[tokio::test]
async fn ordinary_percent_encoding_still_decodes() {
    assert_eq!(get("input[]=a%20b").await.expect("parses").input, vec!["a b"]);
    assert_eq!(get("input[]=a+b").await.expect("parses").input, vec!["a b"]);
    assert_eq!(get("input[]=100%25").await.expect("parses").input, vec!["100%"]);
}

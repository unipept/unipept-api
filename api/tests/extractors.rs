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

// `filter` mirrors the private-api filters, which take a `String` where an empty one is a real
// request. It is here so the empty-value rule can be pinned on both a boolean and a string.
#[derive(Debug, Deserialize, PartialEq)]
struct Parameters {
    #[serde(default)]
    input: Vec<String>,
    #[serde(default)]
    equate_il: bool,
    #[serde(default)]
    filter: String
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
    post_bytes(content_type, body.as_bytes().to_vec()).await
}

async fn post_bytes(content_type: &str, body: Vec<u8>) -> Result<Parameters, StatusCode> {
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

/// All three body encodings, with more than one peptide.
///
/// Two values matter here rather than one: `input[]` repeats, and the multipart branch rebuilds a
/// query string by concatenating parts, so a repeated name is the case where that reconstruction
/// could collapse two values into one.
#[tokio::test]
async fn json_form_and_multipart_bodies_all_parse_to_the_same_parameters() {
    let expected = Parameters {
        input: vec!["AALTER".to_string(), "MKAAGGK".to_string()],
        equate_il: true,
        filter: String::new()
    };

    let json = post("application/json", r#"{"input":["AALTER","MKAAGGK"],"equate_il":true}"#).await;
    assert_eq!(json.expect("json parses"), expected);

    let form = post("application/x-www-form-urlencoded", "input[]=AALTER&input[]=MKAAGGK&equate_il=true").await;
    assert_eq!(form.expect("urlencoded parses"), expected);

    let multipart = post(
        "multipart/form-data; boundary=X",
        "--X\r\nContent-Disposition: form-data; name=\"input[]\"\r\n\r\nAALTER\r\n\
         --X\r\nContent-Disposition: form-data; name=\"input[]\"\r\n\r\nMKAAGGK\r\n\
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

/// Brackets a client encoded still spell an array.
///
/// `URLSearchParams`, browser forms and `$.param` all percent-encode `[` and `]`, so `input[]`
/// arrives as `input%5B%5D`. Unless the key is decoded first that names no field: the peptides are
/// dropped and the caller is answered `200` with an empty result.
#[tokio::test]
async fn an_encoded_bracket_is_still_an_array() {
    let many = get("input%5B%5D=AALTER&input%5B%5D=AAKNER").await.expect("parses");
    assert_eq!(many.input, vec!["AALTER", "AAKNER"]);

    let one = get("input%5B%5D=AALTER").await.expect("parses");
    assert_eq!(one.input, vec!["AALTER"]);
}

#[tokio::test]
async fn an_encoded_bracket_in_a_form_body_is_still_an_array() {
    let parameters = post("application/x-www-form-urlencoded", "input%5B%5D=AALTER&input%5B%5D=AAKNER")
        .await
        .expect("parses");

    assert_eq!(parameters.input, vec!["AALTER", "AAKNER"]);
}

/// Decoding the key early does not decode the value early: a separator inside a value still stays
/// there.
#[tokio::test]
async fn an_encoded_separator_survives_an_encoded_bracket() {
    let parameters = get("input%5B%5D=A%26equate_il%3Dtrue").await.expect("parses");

    assert_eq!(parameters.input, vec!["A&equate_il=true"]);
    assert!(!parameters.equate_il, "a flag the request never sent must not be set");
}

/// A separator inside a multipart value stays inside that value.
///
/// The multipart branch rebuilds a query string from parts `text()` has already decoded, so an `&`
/// in a value used to become a real separator: one field in, two parameters out.
#[tokio::test]
async fn an_encoded_separator_in_a_multipart_body_does_not_become_a_parameter() {
    let body = b"--X\r\nContent-Disposition: form-data; name=\"input[]\"\r\n\r\nA&equate_il=true\r\n--X--\r\n".to_vec();

    let parameters = post_bytes("multipart/form-data; boundary=X", body).await.expect("the value parses");

    assert_eq!(parameters.input, vec!["A&equate_il=true"]);
    assert!(!parameters.equate_il, "a flag the request never sent must not be set");
}

/// A `+` in a multipart value is a plus, not a space.
#[tokio::test]
async fn a_plus_in_a_multipart_value_is_not_a_space() {
    let body = b"--X\r\nContent-Disposition: form-data; name=\"filter\"\r\n\r\na+b\r\n--X--\r\n".to_vec();

    let parameters = post_bytes("multipart/form-data; boundary=X", body).await.expect("the value parses");

    assert_eq!(parameters.filter, "a+b");
}

/// The characters a query string gives meaning survive the rebuild.
///
/// `%41` is the quiet one: a valid escape, so it used to be decoded on the way back in and a filter
/// reading `%41` silently became `A`. `%FF` is the same shape invalid, and failed loudly instead.
#[tokio::test]
async fn multipart_values_survive_the_querystring_rebuild() {
    for value in ["a b", "100%", "a=b", "a&b", "é", "a[0]", "a+b", "%41", "%FF", "%"] {
        let body =
            format!("--X\r\nContent-Disposition: form-data; name=\"filter\"\r\n\r\n{value}\r\n--X--\r\n").into_bytes();

        let parameters = post_bytes("multipart/form-data; boundary=X", body)
            .await
            .unwrap_or_else(|status| panic!("{value:?} was rejected: {status}"));

        assert_eq!(parameters.filter, value, "{value:?} did not survive the rebuild");
    }
}

/// A name carrying a separator cannot introduce a parameter either.
#[tokio::test]
async fn a_multipart_field_name_cannot_inject_a_parameter() {
    let body = b"--X\r\nContent-Disposition: form-data; name=\"filter&equate_il\"\r\n\r\nx\r\n--X--\r\n".to_vec();

    let parameters = post_bytes("multipart/form-data; boundary=X", body).await.expect("an odd name is not an error");

    assert!(!parameters.equate_il, "a flag the request never sent must not be set");
}

/// `input[]` still spells a repeated value: the encoder writes the brackets percent-encoded, and
/// `QS` reads them back as array syntax.
#[tokio::test]
async fn a_multipart_bracket_name_is_still_an_array() {
    let mut body = b"--X\r\nContent-Disposition: form-data; name=\"input[]\"\r\n\r\nAALTER\r\n".to_vec();
    body.extend_from_slice(b"--X\r\nContent-Disposition: form-data; name=\"input[]\"\r\n\r\nMKAAGGK\r\n--X--\r\n");

    let parameters = post_bytes("multipart/form-data; boundary=X", body).await.expect("parses");

    assert_eq!(parameters.input, vec!["AALTER", "MKAAGGK"]);
}

/// The same for a map key, which is what taxa2tree's POST half sends.
///
/// `counts[8501]=3` reaches its `HashMap` only because the key is decoded before its brackets are
/// read. Without that, `counts` deserialises empty and the caller gets a tree built from nothing.
#[tokio::test]
async fn a_multipart_map_key_survives_the_querystring_rebuild() {
    #[derive(Debug, serde::Deserialize)]
    struct Counted {
        #[serde(default)]
        counts: std::collections::HashMap<u32, usize>
    }

    let mut body = b"--X\r\nContent-Disposition: form-data; name=\"counts[8501]\"\r\n\r\n3\r\n".to_vec();
    body.extend_from_slice(b"--X\r\nContent-Disposition: form-data; name=\"counts[8502]\"\r\n\r\n5\r\n--X--\r\n");

    let request = Request::builder()
        .method("POST")
        .uri("/taxa2tree")
        .header(CONTENT_TYPE, "multipart/form-data; boundary=X")
        .body(Body::from(body))
        .unwrap();

    let PostContent(parameters) = PostContent::<Counted>::from_request(request, &())
        .await
        .unwrap_or_else(|_| panic!("the counts parse"));

    assert_eq!(parameters.counts.get(&8501), Some(&3));
    assert_eq!(parameters.counts.get(&8502), Some(&5));
}

/// Ordinary encodings are unaffected: `%20` and `+` are still spaces, `%25` still a percent sign.
#[tokio::test]
async fn ordinary_percent_encoding_still_decodes() {
    assert_eq!(get("input[]=a%20b").await.expect("parses").input, vec!["a b"]);
    assert_eq!(get("input[]=a+b").await.expect("parses").input, vec!["a b"]);
    assert_eq!(get("input[]=100%25").await.expect("parses").input, vec!["100%"]);
}

/// Reading a part's data was the fourth panic, and the only one no test reached.
///
/// Not through encoding: `Field::text()` decodes lossily, so a stray `0xFF` arrives as U+FFFD and
/// never errors. It fails when the body ends part-way through a field's data, which is a stream
/// error rather than a decoding one — and that used to be unwrapped.
#[tokio::test]
async fn a_body_truncated_inside_a_field_is_rejected_not_a_panic() {
    let body = b"--X\r\nContent-Disposition: form-data; name=\"input[]\"\r\n\r\nAALTER".to_vec();

    assert_eq!(post_bytes("multipart/form-data; boundary=X", body).await, Err(StatusCode::UNPROCESSABLE_ENTITY));
}

/// A stray byte is replaced rather than rejected, which is worth stating since it is not obvious
/// and it is why the test above reaches for a truncated body instead.
#[tokio::test]
async fn a_non_utf8_multipart_field_is_decoded_lossily() {
    let mut body = b"--X\r\nContent-Disposition: form-data; name=\"input[]\"\r\n\r\n".to_vec();
    body.push(0xFF);
    body.extend_from_slice(b"\r\n--X--\r\n");

    let parameters = post_bytes("multipart/form-data; boundary=X", body).await.expect("lossy, not rejected");
    assert_eq!(parameters.input, vec!["\u{FFFD}"]);
}

/// A form body that reaches `serde_qs` and cannot be parsed is a rejection, not a panic.
#[tokio::test]
async fn a_form_body_that_cannot_be_parsed_is_rejected() {
    assert_eq!(post("application/x-www-form-urlencoded", "%FF=1").await, Err(StatusCode::UNPROCESSABLE_ENTITY));
}

/// A multipart name that looks like an escape is a name, not an escape.
///
/// A part's name arrives already decoded, so `%FF` is five characters a client chose: it matches no
/// field and is ignored. The form body above is refused instead, because there the client really
/// did send an invalid escape. This used to be a 422, which was the bug rather than the contract.
#[tokio::test]
async fn a_multipart_field_name_that_looks_like_an_escape_is_taken_literally() {
    let body = b"--X\r\nContent-Disposition: form-data; name=\"%FF\"\r\n\r\nv\r\n--X--\r\n".to_vec();

    let parameters = post_bytes("multipart/form-data; boundary=X", body).await.expect("an odd name is not an error");

    assert_eq!(parameters, Parameters { input: vec![], equate_il: false, filter: String::new() });
}

// The tests above only send well-formed input. These cover the boundary: query strings that parse
// structurally but do not describe a request this API serves.

/// A flag sent twice is refused rather than resolved to one of the two.
///
/// `serde_qs` would otherwise take the last value, so this holds only because `request::QS` sets
/// `DuplicateKeyBehavior::Error`.
#[tokio::test]
async fn a_flag_sent_twice_is_rejected() {
    assert_eq!(get("equate_il=true&equate_il=false").await, Err(StatusCode::BAD_REQUEST));
}

/// The other two encodings reach different parsers, so each gets its own assertion.
#[tokio::test]
async fn a_flag_sent_twice_in_a_form_body_is_rejected() {
    assert_eq!(
        post("application/x-www-form-urlencoded", "equate_il=true&equate_il=false").await,
        Err(StatusCode::UNPROCESSABLE_ENTITY)
    );
}

#[tokio::test]
async fn a_flag_sent_twice_in_a_multipart_body_is_rejected() {
    let body = b"--X\r\nContent-Disposition: form-data; name=\"equate_il\"\r\n\r\ntrue\r\n\
--X\r\nContent-Disposition: form-data; name=\"equate_il\"\r\n\r\nfalse\r\n--X--\r\n"
        .to_vec();

    assert_eq!(post_bytes("multipart/form-data; boundary=X", body).await, Err(StatusCode::UNPROCESSABLE_ENTITY));
}

/// A flag named without a value reads as set.
///
/// The sharp end of this is a flag defaulting to false — `?tryptic=` switches on behaviour the
/// caller never spelled out, and raises no error.
#[tokio::test]
async fn a_flag_with_an_empty_value_reads_as_set() {
    assert!(get("equate_il=").await.expect("accepted").equate_il);
    assert!(get("equate_il").await.expect("accepted").equate_il);
}

/// Only a boolean turns an empty value into `true`; a string field keeps the empty string, which
/// the private-api filters send as a real request.
#[tokio::test]
async fn an_empty_value_on_a_string_field_stays_empty() {
    let parameters = get("filter=").await.expect("an empty filter is a real request");

    assert_eq!(parameters.filter, "");
    assert!(!parameters.equate_il, "an unrelated flag is untouched");
}

/// `input=A` repeated reaches the same list as `input[]=A`, so both spellings work.
#[tokio::test]
async fn a_repeated_key_without_brackets_collects_into_the_list() {
    let parameters = get("input=AALTER&input=AAKNER").await.expect("parses");

    assert_eq!(parameters.input, vec!["AALTER", "AAKNER"]);
}

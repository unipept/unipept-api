//! Every boolean parameter, driven through the router with a value it must refuse.
//!
//! `strict_bool` is applied per field, so the guarantee is only as complete as the annotations:
//! one `Parameters` field missing `deserialize_with` is a flag that silently reads `?flag=` as
//! `true` again, and nothing about that fails to compile. This walks the endpoints instead.
//!
//! The extractor rejects before any handler runs, so none of these requests reach the index or
//! OpenSearch — the corpus is built once and every case is a parse.

mod common;

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode}
};
use tower::ServiceExt;
use unipept_api::{middleware::normalize_path::NormalizePath, routes::create_app};

/// Every boolean an endpoint accepts: the route, the flag, and whatever else that route requires.
///
/// The third column matters. The filter endpoints take a mandatory `start` and `end`, and without
/// them every request is a 400 regardless of the flag — which would let the refusal tests below
/// pass without ever reaching `strict_bool`.
///
/// This list is maintained by hand. It was seeded by reading the `deserialize_with =
/// "strict_bool"` annotations out of `controllers`, but nothing regenerates it and nothing checks
/// it is complete: a new boolean parameter has to be added here as well as to its struct, or it
/// simply is not covered.
const BOOLEAN_PARAMETERS: &[(&str, &str, &str)] = &[
    ("/mpa/pept2data", "equate_il", ""),
    ("/mpa/pept2data", "report_taxa", ""),
    ("/mpa/pept2data", "tryptic", ""),
    ("/mpa/pept2data", "validate_taxa", ""),
    ("/api/v2/pept2ec", "equate_il", ""),
    ("/api/v2/pept2ec", "extra", ""),
    ("/api/v2/pept2funct", "domains", ""),
    ("/api/v2/pept2funct", "equate_il", ""),
    ("/api/v2/pept2funct", "extra", ""),
    ("/api/v2/pept2go", "domains", ""),
    ("/api/v2/pept2go", "equate_il", ""),
    ("/api/v2/pept2go", "extra", ""),
    ("/api/v2/pept2interpro", "domains", ""),
    ("/api/v2/pept2interpro", "equate_il", ""),
    ("/api/v2/pept2interpro", "extra", ""),
    ("/api/v2/pept2lca", "equate_il", ""),
    ("/api/v2/pept2lca", "extra", ""),
    ("/api/v2/pept2lca", "names", ""),
    ("/api/v2/pept2lca", "validate_taxa", ""),
    ("/api/v2/pept2prot", "equate_il", ""),
    ("/api/v2/pept2prot", "extra", ""),
    ("/api/v2/pept2prot", "tryptic", ""),
    ("/api/v2/pept2taxa", "compact", ""),
    ("/api/v2/pept2taxa", "equate_il", ""),
    ("/api/v2/pept2taxa", "extra", ""),
    ("/api/v2/pept2taxa", "names", ""),
    ("/api/v2/pept2taxa", "tryptic", ""),
    ("/api/v2/peptinfo", "domains", ""),
    ("/api/v2/peptinfo", "equate_il", ""),
    ("/api/v2/peptinfo", "extra", ""),
    ("/api/v2/peptinfo", "names", ""),
    ("/api/v2/peptinfo", "validate_taxa", ""),
    ("/api/v2/protinfo", "domains", ""),
    ("/api/v2/protinfo", "extra", ""),
    ("/api/v2/protinfo", "names", ""),
    ("/api/v2/taxa2lca", "extra", ""),
    ("/api/v2/taxa2lca", "names", ""),
    ("/api/v2/taxa2lca", "validate_taxa", ""),
    ("/api/v2/taxa2tree", "link", ""),
    ("/api/v2/taxonomy", "descendants", ""),
    ("/api/v2/taxonomy", "extra", ""),
    ("/api/v2/taxonomy", "names", ""),
    ("/private_api/proteomes/filter", "sort_descending", "start=0&end=10"),
    ("/private_api/taxa/filter", "sort_descending", "start=0&end=10")
];

/// Builds `path?extra&fragment`, leaving out the `&` when a route needs nothing extra.
fn query(path: &str, extra: &str, fragment: &str) -> String {
    if extra.is_empty() { format!("{path}?{fragment}") } else { format!("{path}?{extra}&{fragment}") }
}

async fn post_json(app: &NormalizePath<Router>, path: &str, body: &'static str) -> StatusCode {
    let request = Request::post(path)
        .header(axum::http::header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();

    app.clone().oneshot(request).await.expect("the app responds").status()
}

async fn status_of(app: &NormalizePath<Router>, path: &str) -> StatusCode {
    app.clone()
        .oneshot(Request::get(path).body(Body::empty()).unwrap())
        .await
        .expect("the app responds")
        .status()
}

/// `?flag=` is refused everywhere.
///
/// This is the case the parser reads as `true`. On a flag defaulting to false it would switch on
/// behaviour the caller never spelled out and answer 200, which is the whole reason `strict_bool`
/// exists.
#[tokio::test(flavor = "multi_thread")]
async fn every_boolean_refuses_an_empty_value() {
    let (dir, state) = common::offline_state();
    let app = create_app(state);

    let mut accepted = Vec::new();
    for (path, flag, extra) in BOOLEAN_PARAMETERS {
        if status_of(&app, &query(path, extra, &format!("{flag}="))).await != StatusCode::BAD_REQUEST {
            accepted.push(query(path, extra, &format!("{flag}=")));
        }
    }
    drop(dir);

    assert!(accepted.is_empty(), "these read an empty value as a flag: {accepted:#?}");
}

/// A bare key carries no `=` at all, and is refused the same way.
#[tokio::test(flavor = "multi_thread")]
async fn every_boolean_refuses_a_bare_key() {
    let (dir, state) = common::offline_state();
    let app = create_app(state);

    let mut accepted = Vec::new();
    for (path, flag, extra) in BOOLEAN_PARAMETERS {
        if status_of(&app, &query(path, extra, flag)).await != StatusCode::BAD_REQUEST {
            accepted.push(query(path, extra, flag));
        }
    }
    drop(dir);

    assert!(accepted.is_empty(), "these read a bare key as a flag: {accepted:#?}");
}

/// A value that is neither `true` nor `false` is refused rather than guessed at.
#[tokio::test(flavor = "multi_thread")]
async fn every_boolean_refuses_a_value_that_is_not_a_boolean() {
    let (dir, state) = common::offline_state();
    let app = create_app(state);

    let mut accepted = Vec::new();
    for (path, flag, extra) in BOOLEAN_PARAMETERS {
        if status_of(&app, &query(path, extra, &format!("{flag}=yes"))).await != StatusCode::BAD_REQUEST {
            accepted.push(query(path, extra, &format!("{flag}=yes")));
        }
    }
    drop(dir);

    assert!(accepted.is_empty(), "these accepted a non-boolean: {accepted:#?}");
}

/// The spellings that are meant to work still do.
///
/// Without this the suite would pass just as well if `strict_bool` refused everything. These are
/// not asserted to be `OK` — several of these endpoints reach OpenSearch, which is not running —
/// only to have got past the extractor.
#[tokio::test(flavor = "multi_thread")]
async fn every_boolean_still_accepts_true_and_false() {
    let (dir, state) = common::offline_state();
    let app = create_app(state);

    let mut refused = Vec::new();
    for (path, flag, extra) in BOOLEAN_PARAMETERS {
        for value in ["true", "false"] {
            if status_of(&app, &query(path, extra, &format!("{flag}={value}"))).await == StatusCode::BAD_REQUEST {
                refused.push(query(path, extra, &format!("{flag}={value}")));
            }
        }
    }
    drop(dir);

    assert!(refused.is_empty(), "these refused a valid boolean: {refused:#?}");
}

/// A JSON body carries a real boolean, and it is read as one.
///
/// The two body encodings reach different parsers: a query string and a form body carry text, a
/// JSON body carries a typed `true`. An implementation that only understood the text form refused
/// every JSON request in the suite — this is the assertion that says so directly rather than
/// leaving it to be rediscovered from thirteen failing endpoint tests.
#[tokio::test(flavor = "multi_thread")]
async fn a_json_body_carries_a_real_boolean() {
    let (dir, state) = common::offline_state();
    let app = create_app(state);

    let status = post_json(&app, "/api/v2/pept2lca", r#"{"input":["AALTER"],"equate_il":true}"#).await;
    drop(dir);

    assert_eq!(status, StatusCode::OK);
}

/// An empty string in a JSON body is still refused, so the two encodings agree.
#[tokio::test(flavor = "multi_thread")]
async fn a_json_body_still_refuses_an_empty_string() {
    let (dir, state) = common::offline_state();
    let app = create_app(state);

    let status = post_json(&app, "/api/v2/pept2lca", r#"{"input":["AALTER"],"equate_il":""}"#).await;
    drop(dir);

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

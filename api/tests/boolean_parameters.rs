//! Every boolean parameter, driven through the router with a value it must refuse.
//!
//! `strict_bool` goes on each field, so the guarantee is only as complete as the annotations and a
//! missing one still compiles. Both halves of every route are walked because `taxa2tree` has two
//! structs, `GetParameters` and `PostParameters`, each with their own `link`.
//!
//! Nothing here reaches the index or OpenSearch: the extractor refuses before any handler runs.

mod common;

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header}
};
use tower::ServiceExt;
use unipept_api::{middleware::normalize_path::NormalizePath, routes::create_app};

/// Every boolean an endpoint accepts: the route, the flag, and whatever else that route requires.
///
/// The third column carries the filter endpoints' mandatory `start` and `end`. Without them those
/// requests are refused for the missing parameter and never reach `strict_bool` at all.
///
/// Maintained by hand: a new boolean parameter belongs here as well as on its struct, and nothing
/// checks that it is.
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

/// The same query as a form body, so the POST handler's own struct is the one parsed. `PostContent`
/// maps a `Form` rejection to `422` where `GetContent` answers `400`; both are refusals.
async fn post_status_of(app: &NormalizePath<Router>, path: &str, query: &str) -> StatusCode {
    let request = Request::post(path)
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(Body::from(query.to_owned()))
        .unwrap();

    app.clone().oneshot(request).await.expect("the app responds").status()
}

/// `?flag=` is refused everywhere. This is the case the parser reads as `true`, and on a flag
/// defaulting to false it would switch on behaviour the caller never spelled out.
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

/// The spellings that are meant to work still do — without this the suite would pass just as well
/// if `strict_bool` refused everything. Only that they got past the extractor is asserted; several
/// of these endpoints reach OpenSearch, which is not running.
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

/// The POST half too: `taxa2tree`'s `PostParameters::link` is a field the GET pass never touches,
/// and without this, dropping its annotation leaves the whole suite green.
#[tokio::test(flavor = "multi_thread")]
async fn every_boolean_refuses_an_empty_value_on_the_post_half() {
    let (dir, state) = common::offline_state();
    let app = create_app(state);

    let mut accepted = Vec::new();
    for (path, flag, extra) in BOOLEAN_PARAMETERS {
        let body = query("", extra, &format!("{flag}=")).trim_start_matches('?').to_owned();
        if post_status_of(&app, path, &body).await != StatusCode::UNPROCESSABLE_ENTITY {
            accepted.push(format!("POST {path} {body}"));
        }
    }
    drop(dir);

    assert!(accepted.is_empty(), "these read an empty value as a flag on POST: {accepted:#?}");
}

/// A JSON body carries a typed boolean where the other encodings carry text. An implementation
/// that understood only text refused every JSON request in the suite.
#[tokio::test(flavor = "multi_thread")]
async fn a_json_body_carries_a_real_boolean() {
    let (dir, state) = common::offline_state();
    let app = create_app(state);

    let status = post_json(&app, "/api/v2/pept2lca", r#"{"input":["AALTER"],"equate_il":true}"#).await;
    drop(dir);

    assert_eq!(status, StatusCode::OK);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_json_body_still_refuses_an_empty_string() {
    let (dir, state) = common::offline_state();
    let app = create_app(state);

    let status = post_json(&app, "/api/v2/pept2lca", r#"{"input":["AALTER"],"equate_il":""}"#).await;
    drop(dir);

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

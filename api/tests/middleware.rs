//! The middleware stack `create_app` wraps every route in.
//!
//! These layers answer before any controller runs, so none of the endpoint suites reach them: a
//! request that a layer refuses never becomes a handler call, and one it lets through carries
//! headers no handler sets. They are tested here, against the whole stack, for that reason.
//!
//! The body-limit tests POST a form body whose only field the endpoint ignores, so the body is
//! read in full and parsed while the search itself stays empty — the limit is what is under test,
//! not the handler behind it.

mod common;

use std::time::Duration;

use axum::{
    body::Body,
    http::{Request, StatusCode, header}
};
use tower::ServiceExt;
use unipept_api::routes::{create_app, create_app_with_timeout};

use crate::common::{offline_state, request_parts};

/// 50 MiB, the ceiling `create_router` installs.
const BODY_LIMIT: usize = 50 * 1024 * 1024;

async fn send(request: Request<Body>) -> (StatusCode, header::HeaderMap) {
    let (dir, state) = common::offline_state();
    let response = create_app(state).oneshot(request).await.expect("the app responds");
    let answered = (response.status(), response.headers().clone());
    drop(dir);
    answered
}

/// POSTs a form body of roughly `size` bytes in a field `pept2lca` does not read.
///
/// The peptide list stays empty, so the request costs a body read and a parse rather than a
/// search, and the status reflects the middleware rather than the corpus.
async fn post_form_of(size: usize) -> (StatusCode, header::HeaderMap) {
    let mut body = String::from("junk=");
    body.extend(std::iter::repeat_n('A', size.saturating_sub(body.len())));

    let request = Request::post("/api/v2/pept2lca")
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(Body::from(body))
        .unwrap();

    send(request).await
}

/// A cross-origin GET comes back usable by the browser that sent it.
///
/// The API is called from pages the project does not serve, so the allow-origin header is not
/// decoration: without it the response reaches the browser and the browser discards it.
#[tokio::test(flavor = "multi_thread")]
async fn a_cross_origin_request_is_allowed() {
    let request = Request::get("/").header(header::ORIGIN, "https://unipept.ugent.be").body(Body::empty()).unwrap();

    let (status, headers) = send(request).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers.get(header::ACCESS_CONTROL_ALLOW_ORIGIN).map(|v| v.to_str().unwrap()), Some("*"));
}

/// `ETag` is exposed, which is what lets a caller read it at all.
///
/// A cross-origin response hides every header outside the CORS safelist, and `ETag` is not on it.
/// Conditional requests depend on the client having seen the tag it is echoing back.
#[tokio::test(flavor = "multi_thread")]
async fn the_etag_header_is_exposed_to_cross_origin_callers() {
    let request = Request::get("/").header(header::ORIGIN, "https://unipept.ugent.be").body(Body::empty()).unwrap();

    let (_, headers) = send(request).await;
    let exposed = headers
        .get(header::ACCESS_CONTROL_EXPOSE_HEADERS)
        .expect("expose-headers should be set")
        .to_str()
        .unwrap()
        .to_ascii_lowercase();

    assert!(exposed.contains("etag"), "etag should be exposed, got: {exposed}");
}

/// The preflight answers with the methods and headers the API actually takes.
///
/// A browser sends this before any POST carrying `Content-Type: application/json`, and refuses to
/// send the real request unless the method and the header both come back allowed.
#[tokio::test(flavor = "multi_thread")]
async fn a_preflight_allows_the_methods_and_headers_the_api_uses() {
    let request = Request::builder()
        .method("OPTIONS")
        .uri("/api/v2/pept2lca")
        .header(header::ORIGIN, "https://unipept.ugent.be")
        .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
        .header(header::ACCESS_CONTROL_REQUEST_HEADERS, "content-type")
        .body(Body::empty())
        .unwrap();

    let (status, headers) = send(request).await;
    assert!(status.is_success(), "a preflight should succeed, got {status}");

    let methods = headers.get(header::ACCESS_CONTROL_ALLOW_METHODS).expect("allow-methods").to_str().unwrap();
    assert!(methods.contains("GET"), "GET should be allowed, got: {methods}");
    assert!(methods.contains("POST"), "POST should be allowed, got: {methods}");

    let allowed = headers
        .get(header::ACCESS_CONTROL_ALLOW_HEADERS)
        .expect("allow-headers")
        .to_str()
        .unwrap()
        .to_ascii_lowercase();
    assert!(allowed.contains("content-type"), "content-type should be allowed, got: {allowed}");
}

/// A body larger than the framework default is accepted.
///
/// axum refuses bodies over 2 MiB unless told otherwise, and this API raises that to 50 MiB
/// because a peptide list is genuinely that big. Losing the raise would refuse real requests, and
/// a test that only checked the ceiling would not notice: it is the floor that moved.
#[tokio::test(flavor = "multi_thread")]
async fn a_body_over_the_framework_default_is_accepted() {
    let (status, _) = post_form_of(4 * 1024 * 1024).await;

    assert_eq!(status, StatusCode::OK, "4 MiB is under the 50 MiB limit and should be read");
}

/// A body just under the ceiling is still read.
///
/// Paired with the test below, this is what places the limit at 50 MiB rather than merely
/// somewhere: a bump that quietly lowered it would turn this green test red.
#[tokio::test(flavor = "multi_thread")]
async fn a_body_just_under_the_limit_is_accepted() {
    let (status, _) = post_form_of(BODY_LIMIT - 4096).await;

    assert_eq!(status, StatusCode::OK);
}

/// A body past the ceiling is refused rather than read into memory.
///
/// The status is `422`, not the `413` the limit itself raises: `Form::from_request` maps every
/// `RawForm` extraction failure to `UNPROCESSABLE_ENTITY`, so the payload-too-large answer is
/// swallowed on its way out. That is worth a look on its own, and is asserted here as it stands so
/// that changing it is a decision rather than an accident.
#[tokio::test(flavor = "multi_thread")]
async fn a_body_over_the_limit_is_refused() {
    let (status, _) = post_form_of(BODY_LIMIT + 4096).await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

/// A timed-out request still carries the CORS headers.
///
/// `TimeoutLayer` answers above the router, so this response is built by a layer rather than by a
/// handler. If the CORS layer sits below the timeout, the 408 goes out bare and the browser
/// discards it before any code sees it — the caller gets an opaque network error instead of the
/// status that says what happened.
///
/// Driven with a body that never finishes arriving, because a timeout cannot be provoked any other
/// way here: `Timeout` polls the inner service before its own sleep, and the corpus answers
/// immediately however low the duration goes. Awaiting the body is the one part of the request
/// this test can hold open.
#[tokio::test(flavor = "multi_thread")]
async fn a_timed_out_request_still_carries_the_cors_headers() {
    let stalled = Body::from_stream(futures_util::stream::pending::<Result<bytes::Bytes, std::io::Error>>());

    let (dir, state) = common::offline_state();
    let app = create_app_with_timeout(state, Duration::from_millis(50));

    let request = Request::post("/api/v2/pept2lca")
        .header(header::ORIGIN, "https://unipept.ugent.be")
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(stalled)
        .unwrap();

    let response = app.oneshot(request).await.expect("the app responds");
    let (status, headers) = (response.status(), response.headers().clone());
    drop(dir);

    assert_eq!(status, StatusCode::REQUEST_TIMEOUT);
    assert_eq!(
        headers.get(header::ACCESS_CONTROL_ALLOW_ORIGIN).map(|v| v.to_str().unwrap()),
        Some("*"),
        "a 408 the browser cannot read is a 408 nobody receives"
    );
}

/// A request refused for its declared size still carries the CORS headers.
///
/// `RequestBodyLimitLayer` reads the content-length and answers 413 without waiting for the body,
/// so like the timeout above this response never reaches the router.
///
/// This is the reachable half of the pair: the body-limit path that `a_body_over_the_limit_is_refused`
/// drives ends in a 422 from the extractor, which is built inside the router and therefore already
/// passed through the CORS layer whichever order the two were in. Only the declared-length refusal
/// exercises the layer itself.
#[tokio::test(flavor = "multi_thread")]
async fn a_body_refused_for_its_declared_size_still_carries_the_cors_headers() {
    let request = Request::post("/api/v2/pept2lca")
        .header(header::ORIGIN, "https://unipept.ugent.be")
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .header(header::CONTENT_LENGTH, (BODY_LIMIT + 1).to_string())
        .body(Body::from("junk=A"))
        .unwrap();

    let (status, headers) = send(request).await;

    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(
        headers.get(header::ACCESS_CONTROL_ALLOW_ORIGIN).map(|v| v.to_str().unwrap()),
        Some("*"),
        "a 413 the browser cannot read is a 413 nobody receives"
    );
}

/// Every failure answers as JSON, whether it comes from a handler or from an extractor that
/// rejected the request before one ran.
///
/// A client parses one shape either way; a bare text body meant it had to know which happened.
#[tokio::test(flavor = "multi_thread")]
async fn a_failure_answers_as_json() {
    let cases = [
        // Rejected by an extractor, before any handler.
        ("/private_api/taxa/filter?start=notanumber", StatusCode::BAD_REQUEST, "invalid query string"),
        // Refused by a handler.
        ("/private_api/taxa/filter?start=5&end=1", StatusCode::BAD_REQUEST, "end (1) must be at least start (5)"),
        // A rank no lineage column names.
        (
            "/api/v2/taxonomy?input[]=1&descendants=true&descendants_ranks[]=nonsense",
            StatusCode::BAD_REQUEST,
            "An unknown rank has been passed for the `descendant_rank` parameter."
        )
    ];

    for (path, expected_status, expected_message) in cases {
        let (dir, state) = offline_state();
        let (status, content_type, body) = request_parts(state, Request::get(path).body(Body::empty()).unwrap()).await;
        drop(dir);

        assert_eq!(status, expected_status, "{path}");
        assert!(content_type.starts_with("application/json"), "{path}: content type was {content_type:?}");

        let parsed: serde_json::Value =
            serde_json::from_str(&body).unwrap_or_else(|err| panic!("{path}: body was not JSON ({err}): {body}"));
        assert_eq!(parsed["error"], expected_message, "{path}");
    }
}

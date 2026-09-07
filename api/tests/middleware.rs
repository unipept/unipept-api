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

use axum::{
    body::Body,
    http::{Request, StatusCode, header}
};
use tower::ServiceExt;
use unipept_api::routes::create_app;

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

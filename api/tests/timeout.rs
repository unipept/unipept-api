//! The request timeout, and the status a timed-out request answers with.
//!
//! `HandleErrorLayer` hands the handler a `BoxError`, so `routes::timeout_status` has to ask what
//! the error is by downcasting. That question is settled at run time and nothing about it is
//! checked when the code compiles: if `Elapsed` were to become a different type, every timeout
//! would start answering 500 and the build would stay green.

mod common;

use std::time::Duration;

use axum::{
    BoxError,
    body::Body,
    http::{Request, StatusCode}
};
use tower::{Service, ServiceBuilder, ServiceExt, timeout::TimeoutLayer};
use unipept_api::routes::{create_app_with_timeout, timeout_status};

/// An elapsed timeout maps to 408, by way of the downcast production relies on.
#[test]
fn an_elapsed_timeout_is_a_request_timeout() {
    let err: BoxError = Box::new(tower::timeout::error::Elapsed::new());

    assert_eq!(timeout_status(err), StatusCode::REQUEST_TIMEOUT);
}

/// Anything else is a 500, so the downcast is doing work rather than always matching.
#[test]
fn any_other_error_is_an_internal_server_error() {
    let err: BoxError = Box::new(std::io::Error::other("something else went wrong"));

    assert_eq!(timeout_status(err), StatusCode::INTERNAL_SERVER_ERROR);
}

/// `TimeoutLayer` raises the error `timeout_status` recognises.
///
/// The tests above pin what `timeout_status` does with an `Elapsed` it was handed. This one pins
/// that the layer is what hands it one — were `TimeoutLayer` to start raising some other error,
/// they would go on passing while every real timeout answered 500.
///
/// Driven against a service that never finishes rather than through the app: the corpus answers
/// immediately, and `Timeout` polls the inner service before its own sleep, so a real endpoint
/// cannot be made to elapse by lowering the duration.
#[tokio::test(flavor = "multi_thread")]
async fn the_timeout_layer_raises_the_error_the_handler_recognises() {
    let never_finishes = tower::service_fn(|_: ()| std::future::pending::<Result<(), BoxError>>());

    let mut service = ServiceBuilder::new().layer(TimeoutLayer::new(Duration::from_millis(10))).service(never_finishes);

    let err = service.ready().await.expect("ready").call(()).await.expect_err("the call should elapse");

    assert_eq!(timeout_status(err), StatusCode::REQUEST_TIMEOUT);
}

/// A request inside the timeout is untouched by the layer.
#[tokio::test(flavor = "multi_thread")]
async fn a_request_within_the_timeout_is_unaffected() {
    let (dir, state) = common::offline_state();
    let app = create_app_with_timeout(state, Duration::from_secs(30));

    let request = Request::get("/api/v2/pept2lca?input[]=AALTER").body(Body::empty()).unwrap();
    let response = app.oneshot(request).await.expect("the app responds");

    assert_eq!(response.status(), StatusCode::OK);
    drop(dir);
}

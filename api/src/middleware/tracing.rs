use std::time::Duration;

use axum::extract::MatchedPath;
use http::{Request, Response};
use tower_http::trace::{MakeSpan, OnResponse};
use tracing::Span;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Installs the global tracing subscriber, once.
///
/// `try_init` rather than `init`: a subscriber can only be set once per process, and `init`
/// panics on the second attempt. Nothing here needs to be the one that succeeded — anything that
/// calls this wants tracing configured, not exclusive ownership of it — so a second call is a
/// no-op instead of an abort.
pub fn init_tracing_subscriber() {
    let _ = tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            // axum logs rejections from built-in extractors with the `axum::rejection`
            // target, at `TRACE` level. `axum::rejection=trace` enables showing those events
            "example_tracing_aka_logging=debug,tower_http=debug,axum::rejection=trace".into()
        }))
        .with(tracing_subscriber::fmt::layer())
        .try_init();
}

/// Builds the span every request runs in, carrying its method, matched route and request id.
///
/// A named type rather than a closure, so the layer can be assembled without naming
/// `TraceLayer`'s seven type parameters.
#[derive(Clone, Copy)]
pub struct RequestSpan;

impl<B> MakeSpan<B> for RequestSpan {
    fn make_span(&mut self, request: &Request<B>) -> Span {
        // The matched route rather than the raw path, so query strings and path parameters don't
        // turn one endpoint into a thousand distinct span names. Falls back to the raw path for
        // requests no route matched.
        let route = request
            .extensions()
            .get::<MatchedPath>()
            .map(MatchedPath::as_str)
            .unwrap_or_else(|| request.uri().path());

        let span = tracing::info_span!(
            "request",
            method = %request.method(),
            route,
            request_id = tracing::field::Empty
        );

        // Honours an id HAProxy already set rather than inventing a competing one; absent or
        // non-UTF-8 is left unrecorded rather than treated as an error.
        if let Some(request_id) = request.headers().get("x-request-id").and_then(|value| value.to_str().ok()) {
            span.record("request_id", request_id);
        }

        span
    }
}

/// Logs the outcome of a completed request, in this service's own vocabulary rather than
/// tower-http's.
///
/// A named type rather than a closure, so the layer can be assembled without naming
/// `TraceLayer`'s seven type parameters. `on_response` takes `self` by value, which is why the
/// struct is `Copy` rather than borrowed.
#[derive(Clone, Copy)]
pub struct LogResponse;

impl<B> OnResponse<B> for LogResponse {
    fn on_response(self, response: &Response<B>, latency: Duration, _span: &Span) {
        let status = response.status();
        let latency_ms = latency.as_millis();

        if status.is_server_error() {
            tracing::error!(target: "unipept_api", status = status.as_u16(), latency_ms, "request");
        } else if status.is_client_error() {
            tracing::warn!(target: "unipept_api", status = status.as_u16(), latency_ms, "request");
        } else {
            tracing::info!(target: "unipept_api", status = status.as_u16(), latency_ms, "request");
        }
    }
}

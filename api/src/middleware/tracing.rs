use std::{io::IsTerminal, time::Duration};

use http::{Request, Response};
use tower_http::trace::{MakeSpan, OnResponse};
use tracing::Span;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// The bare `warn` is the base every other target falls back to, so a crate that is not named here
// still reports its problems. Without it, a `warn!` added to `database` or `datastore` would go
// nowhere until somebody remembered to add a directive for it.
//
// axum logs rejections from built-in extractors with the `axum::rejection` target, at `TRACE`
// level. `axum::rejection=trace` enables showing those events.
const DEFAULT_FILTER: &str = "warn,unipept_api=info,datastore=info,index=info,tower_http=info,axum::rejection=trace";

/// Installs the global tracing subscriber, once.
///
/// `try_init` rather than `init`: a subscriber can only be set once per process, and `init`
/// panics on the second attempt. Nothing here needs to be the one that succeeded — anything that
/// calls this wants tracing configured, not exclusive ownership of it — so a second call is a
/// no-op instead of an abort.
pub fn init_tracing_subscriber() {
    let _ = tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| DEFAULT_FILTER.into()))
        .with(tracing_subscriber::fmt::layer().with_ansi(std::io::stdout().is_terminal()))
        .try_init();
}

/// Builds the span every request runs in, carrying its method, matched route and request id.
///
/// A named type rather than a closure, because the impl has to be generic over the body. The
/// request body at this layer is `tower_http::limit`'s wrapper, which that crate does not export,
/// and a closure would have to name it.
#[derive(Clone, Copy)]
pub struct RequestSpan;

impl<B> MakeSpan<B> for RequestSpan {
    fn make_span(&mut self, request: &Request<B>) -> Span {
        // The path as asked for, without the query string. This layer sits above the router, so
        // `MatchedPath` is not resolved yet and cannot be used: the span has to exist before the
        // timeout and the body limit answer, or the requests those refuse are never logged at all.
        //
        // No route in this API takes a path parameter, so the two are the same string for every
        // request that matches one. Add a route that does, and this field becomes as many values
        // as that parameter has, and wants recording from inside the router instead.
        let span = tracing::info_span!(
            "request",
            method = %request.method(),
            route = request.uri().path(),
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
/// tower-http's. `DefaultOnResponse` carries one fixed level for every response; varying the level
/// by status class is the point.
///
/// A closure would compile here, unlike for [`RequestSpan`] — the response body is nameable. It is
/// a named type to match its sibling, and to keep the layer in `routes.rs` to one line.
#[derive(Clone, Copy)]
pub struct LogResponse;

impl<B> OnResponse<B> for LogResponse {
    fn on_response(self, response: &Response<B>, latency: Duration, _span: &Span) {
        let status = response.status();
        // Milliseconds to three places. Whole milliseconds round every request this API answers
        // from memory down to the same `0`, and a bare `as_secs_f64` prints the binary remainder.
        let latency_ms = (latency.as_secs_f64() * 1_000_000.0).round() / 1000.0;

        // The target is left to default to the module path, as it is everywhere else in this
        // crate: `unipept_api` matches it by prefix, and it says which module spoke.
        if status.is_server_error() {
            tracing::error!(status = status.as_u16(), latency_ms, "request");
        } else if status.is_client_error() {
            tracing::warn!(status = status.as_u16(), latency_ms, "request");
        } else {
            tracing::info!(status = status.as_u16(), latency_ms, "request");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::DEFAULT_FILTER;

    /// A typo here would otherwise be silent: `EnvFilter` parsing only ever runs against
    /// whatever `RUST_LOG` happens to be set to, so a broken default would go unnoticed.
    #[test]
    fn the_default_filter_parses() {
        DEFAULT_FILTER.parse::<tracing_subscriber::EnvFilter>().expect("DEFAULT_FILTER is valid");
    }
}

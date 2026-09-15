use std::io::IsTerminal;

use tower_http::{
    classify::{ServerErrorsAsFailures, SharedClassifier},
    trace::{DefaultMakeSpan, DefaultOnBodyChunk, DefaultOnEos, DefaultOnRequest, DefaultOnResponse, TraceLayer}
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// The bare `warn` is the base every other target falls back to, so a crate that is not named here
// still reports its problems. Without it, a `warn!` added to `database` or `datastore` would go
// nowhere until somebody remembered to add a directive for it.
//
// axum logs rejections from built-in extractors with the `axum::rejection` target, at `TRACE`
// level. `axum::rejection=trace` enables showing those events.
const DEFAULT_FILTER: &str = "warn,unipept_api=info,index=info,tower_http=info,axum::rejection=trace";

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

/// The per-request tracing layer, with the failure line turned off.
///
/// `DefaultOnFailure` writes an `ERROR` for every 5xx, naming the status and the latency but not
/// the cause. `ApiError` already writes one for the same response, and that one carries the
/// `source()` chain. `()` is the no-op `OnFailure`, so a failed request produces one line, not two.
pub fn create_tracing_layer() -> TraceLayer<
    SharedClassifier<ServerErrorsAsFailures>,
    DefaultMakeSpan,
    DefaultOnRequest,
    DefaultOnResponse,
    DefaultOnBodyChunk,
    DefaultOnEos,
    ()
> {
    TraceLayer::new_for_http().on_failure(())
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

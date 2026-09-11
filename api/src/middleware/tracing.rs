use std::io::IsTerminal;

use tower_http::{
    classify::{ServerErrorsAsFailures, SharedClassifier},
    trace::TraceLayer
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// axum logs rejections from built-in extractors with the `axum::rejection` target, at `TRACE`
// level. `axum::rejection=trace` enables showing those events.
const DEFAULT_FILTER: &str = "unipept_api=info,tower_http=info,axum::rejection=trace";

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

pub fn create_tracing_layer() -> TraceLayer<SharedClassifier<ServerErrorsAsFailures>> {
    TraceLayer::new_for_http()
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

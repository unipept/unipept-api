use tower_http::{
    classify::{ServerErrorsAsFailures, SharedClassifier},
    trace::TraceLayer
};
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

pub fn create_tracing_layer() -> TraceLayer<SharedClassifier<ServerErrorsAsFailures>> {
    TraceLayer::new_for_http()
}

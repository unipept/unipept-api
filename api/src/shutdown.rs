//! When the process stops serving.
//!
//! systemd sends SIGTERM on `systemctl restart` and waits `TimeoutStopSec` before it kills what is
//! left. The default disposition for SIGTERM ends the process immediately, so without a handler
//! every request still being answered is lost. Endpoints here run to a 150-second timeout, and a
//! rolling deploy restarts each server in turn, so that is real work rather than a rare edge.

use tokio::signal::unix::{SignalKind, signal};

/// Resolves when the process is asked to stop, naming the signal that asked.
///
/// SIGTERM is what systemd sends, SIGINT what a terminal sends, and either one means the same
/// thing here.
pub async fn requested() {
    let signal = tokio::select! {
        name = wait_for(SignalKind::terminate(), "SIGTERM") => name,
        name = wait_for(SignalKind::interrupt(), "SIGINT") => name
    };

    tracing::info!(signal, "stopping, and waiting for the requests in flight");
}

/// Resolves with `name` the first time `kind` arrives.
///
/// A signal that cannot be listened for never resolves, rather than resolving at once: resolving
/// would stop a server nobody asked to stop. The process then ends on that signal by its default
/// disposition, without the wait, which is the outcome this module exists to avoid — so it is
/// logged at ERROR.
async fn wait_for(kind: SignalKind, name: &'static str) -> &'static str {
    match signal(kind) {
        Ok(mut stream) => {
            stream.recv().await;
            name
        }
        Err(error) => {
            tracing::error!(
                error = &error as &dyn std::error::Error,
                signal = name,
                "cannot listen for this signal, so a stop will drop the requests in flight"
            );
            std::future::pending().await
        }
    }
}

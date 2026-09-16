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
///
/// `axum::serve` polls this from a task it spawns, so the handlers are installed when serving
/// begins, not when `start` is called. A signal arriving during the index load therefore ends the
/// process by its default disposition — which loses nothing, because nothing is being served yet.
///
/// Only the first signal is acted on. Once this resolves the streams are dropped, and tokio leaves
/// its own handler installed process-wide, so a second SIGINT does not force an immediate exit the
/// way it would without this module: a drain has to finish, or be ended with SIGKILL.
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
        // `None` means the stream closed rather than that a signal arrived, so it is not a request
        // to stop and must not be treated as one, for the same reason a failed registration is not.
        Ok(mut stream) => match stream.recv().await {
            Some(()) => name,
            None => std::future::pending().await
        },
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

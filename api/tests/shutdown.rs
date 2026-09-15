//! What SIGTERM does to a request that is already being answered.
//!
//! Its own test binary, deliberately. The test raises SIGTERM at its own process, and a signal is
//! delivered to the process rather than to a test, so every other test sharing the binary would
//! see it too.
//!
//! The route is a local one rather than the real router: what is under test is the shutdown
//! future and the wait it causes, neither of which reads the request.

use std::{process::Command, sync::Arc, time::Duration};

use axum::{Router, routing::get};
use tokio::{net::TcpListener, sync::Notify};
use unipept_api::shutdown;

/// How long the handler stays in the request. Long enough that the signal lands well inside it.
const HANDLER_DURATION: Duration = Duration::from_millis(500);

/// A request already in a handler is answered, and only then does the server return.
#[tokio::test(flavor = "multi_thread")]
async fn a_request_in_flight_outlives_sigterm() {
    // `Notify` rather than a channel: axum needs the handler to be `Fn`, and it keeps one permit,
    // so the wait below still returns when the handler got there first.
    let entered = Arc::new(Notify::new());
    let in_a_handler = Arc::clone(&entered);

    let app = Router::new().route(
        "/slow",
        get(move || {
            let entered = Arc::clone(&entered);
            async move {
                // Reports that the request reached a handler, which is also what proves the
                // shutdown future has been polled: `serve` polls it before it accepts.
                entered.notify_one();

                tokio::time::sleep(HANDLER_DURATION).await;
                "answered"
            }
        })
    );

    let listener = TcpListener::bind("127.0.0.1:0").await.expect("an ephemeral port should bind");
    let address = listener.local_addr().expect("the listener should report its address");

    let server =
        tokio::spawn(async move { axum::serve(listener, app).with_graceful_shutdown(shutdown::requested()).await });

    let request = tokio::spawn(async move { reqwest::get(format!("http://{address}/slow")).await });

    in_a_handler.notified().await;

    let killed = Command::new("kill")
        .args(["-TERM", &std::process::id().to_string()])
        .status()
        .expect("kill should run");
    assert!(killed.success(), "kill did not signal this process");

    let response = request.await.expect("the request task should not panic").expect("the request should not fail");
    assert_eq!(response.status(), 200);
    assert_eq!(response.text().await.expect("the body should read"), "answered");

    // Generous: the assertions above already prove the handler finished, so this only has to
    // outlast the return itself.
    tokio::time::timeout(Duration::from_secs(5), server)
        .await
        .expect("the server should return after the signal")
        .expect("the server task should not panic")
        .expect("serving should end without an error");
}

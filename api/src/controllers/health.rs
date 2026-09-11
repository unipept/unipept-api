//! `/health` and `/health/database` — what a deployment probes.
//!
//! `/health` answers as soon as the process is serving, for a load balancer deciding whether to
//! route to it. `/health/database` answers for OpenSearch specifically, so an outage there is
//! reported without evicting a server that still answers every route that does not need it.

use std::time::Duration;

use axum::{extract::State, http::StatusCode};

use crate::AppState;

/// The client's own timeout is 120 seconds, far past any sensible poll interval, so the probe
/// carries a much shorter one of its own.
const DATABASE_PROBE_TIMEOUT: Duration = Duration::from_secs(2);

/// A probe asks one way: no `generate_handlers!`, no POST half, no `.json` twin.
pub async fn get_handler() -> StatusCode {
    StatusCode::OK
}

/// As [`get_handler`], but for OpenSearch: 503 if it fails to answer within
/// `DATABASE_PROBE_TIMEOUT`, OK otherwise.
pub async fn get_database_handler(State(AppState { database, .. }): State<AppState>) -> StatusCode {
    match tokio::time::timeout(DATABASE_PROBE_TIMEOUT, database::ping(database.get_conn())).await {
        Ok(Ok(())) => StatusCode::OK,
        _ => StatusCode::SERVICE_UNAVAILABLE
    }
}

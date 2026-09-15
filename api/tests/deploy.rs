//! What the checked-in unit file has to agree with the code about.
//!
//! The unit is data, not code, so nothing else would notice it drifting. One invariant matters
//! enough to assert: systemd has to wait for a drain that the API is still allowed to be doing.

use unipept_api::routes::REQUEST_TIMEOUT_DURATION;

const UNIT: &str = include_str!("../../.deploy/server/unipept-api.service");

/// Reads a `Key=value` setting out of the unit.
fn setting(key: &str) -> &'static str {
    UNIT.lines()
        .find_map(|line| line.trim().strip_prefix(&format!("{key}=")))
        .unwrap_or_else(|| panic!("the unit has no {key}"))
}

/// Raise the request timeout past `TimeoutStopSec` and a restart starts killing requests that were
/// still allowed to be running. The failure is silent — a rollout simply drops work — so it is
/// asserted here rather than left to be noticed in production.
#[test]
fn the_unit_waits_for_a_request_to_finish() {
    let stop_timeout: u64 = setting("TimeoutStopSec").parse().expect("TimeoutStopSec should be seconds");

    assert!(
        stop_timeout > REQUEST_TIMEOUT_DURATION,
        "TimeoutStopSec is {stop_timeout}s, which does not outlast the {REQUEST_TIMEOUT_DURATION}s request timeout"
    );
}

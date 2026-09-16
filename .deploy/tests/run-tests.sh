#!/usr/bin/env bash
#
# Runs the deploy suites. From anywhere: `.deploy/tests/run-tests.sh`
#
#   run-tests.sh                  every suite
#   run-tests.sh server           install.sh and deploy.sh, against a real systemd user manager
#
# A container rather than a mock, because what these scripts get wrong is exactly what a mock gets
# wrong too: `systemctl --user` without an init, lingering, a user unit that cannot drop capabilities.
# The image is built from the version production runs.
#
# Needs Docker, and --privileged so systemd can boot.

set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly HERE
readonly DEPLOY="${HERE}/.."
readonly SERVER_IMAGE=unipept-deploy-test-server
readonly CONTAINER=unipept-deploy-test

log() { printf '\n\033[1m%s\033[0m\n' "$*"; }

command -v docker >/dev/null || { echo "docker is not installed" >&2; exit 1; }

# The systemd container is long-lived: booting it takes a few seconds, and every server case runs
# against the same one, reset between suites rather than rebuilt.
start_server_container() {
    docker rm -f "$CONTAINER" >/dev/null 2>&1 || true
    docker run -d --name "$CONTAINER" --privileged --cgroupns=host \
        -v /sys/fs/cgroup:/sys/fs/cgroup:rw "$SERVER_IMAGE" >/dev/null

    local waited=0
    until [ "$(docker exec "$CONTAINER" systemctl is-system-running 2>/dev/null)" = running ]; do
        sleep 1
        waited=$((waited + 1))
        [ "$waited" -lt 60 ] || { echo "systemd did not come up in the container" >&2; exit 1; }
    done
}

run_server_suite() {
    log "Building the server image"
    docker build -q -t "$SERVER_IMAGE" "${HERE}/server" >/dev/null

    log "Booting systemd"
    start_server_container

    docker cp "$DEPLOY" "${CONTAINER}:/deploy" >/dev/null
    docker cp "${HERE}/server/swap-probe.sh" "${CONTAINER}:/swap-probe.sh" >/dev/null

    log "Server suite: install.sh and deploy.sh"
    docker exec "$CONTAINER" bash /deploy/tests/server/suite.sh
}

cleanup() { docker rm -f "$CONTAINER" >/dev/null 2>&1 || true; }
trap cleanup EXIT

case "${1:-all}" in
    server | all) run_server_suite ;;
    *) sed -n '2,10p' "${BASH_SOURCE[0]}" >&2; exit 2 ;;
esac

log "Every suite passed"

#!/usr/bin/env bash
#
# Runs the deploy suites. From anywhere: `.deploy/tests/run-tests.sh`
#
#   run-tests.sh                  every suite
#   run-tests.sh server           install.sh and deploy.sh, against a real systemd user manager
#   run-tests.sh haproxy          haproxy.sh, against a real HAProxy
#   run-tests.sh rollout          rollout.sh, against a real HAProxy with two backends
#   run-tests.sh lb-install       loadbalancer/install.sh, against a real HAProxy
#
# Containers rather than mocks, because what these scripts get wrong is exactly what a mock gets wrong
# too: `systemctl --user` without an init, a `show stat` field layout, a status that reads "UP 1/100"
# while a server is being checked back in. Both images are built from the versions production runs.
#
# Needs Docker, and the server suite needs to run a container with --privileged so systemd can boot.

set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly HERE
readonly DEPLOY="${HERE}/.."
readonly SERVER_IMAGE=unipept-deploy-test-server
readonly LB_IMAGE=unipept-deploy-test-loadbalancer
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

run_lb_suite() {
    local suite=$1 title=$2

    log "Building the load balancer image"
    docker build -q -t "$LB_IMAGE" "${HERE}/loadbalancer" >/dev/null

    log "$title"
    docker run --rm --user root \
        -v "${DEPLOY}:/deploy:ro" \
        -v "${HERE}/loadbalancer:/test:ro" \
        --entrypoint bash "$LB_IMAGE" -c \
        "mkdir -p /etc/haproxy && cp /test/haproxy.cfg /etc/haproxy/haproxy.cfg && bash /deploy/tests/loadbalancer/${suite}"
}

cleanup() { docker rm -f "$CONTAINER" >/dev/null 2>&1 || true; }
trap cleanup EXIT

case "${1:-all}" in
    server)  run_server_suite ;;
    haproxy) run_lb_suite haproxy-suite.sh "HAProxy suite: haproxy.sh" ;;
    rollout) run_lb_suite rollout-suite.sh "Rollout suite: rollout.sh" ;;
    lb-install) run_lb_suite install-suite.sh "Load balancer install suite" ;;
    all)
        run_server_suite
        run_lb_suite haproxy-suite.sh "HAProxy suite: haproxy.sh"
        run_lb_suite rollout-suite.sh "Rollout suite: rollout.sh"
        run_lb_suite install-suite.sh "Load balancer install suite"
        ;;
    *) sed -n '2,12p' "${BASH_SOURCE[0]}" >&2; exit 2 ;;
esac

log "Every suite passed"

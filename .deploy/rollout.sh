#!/usr/bin/env bash
#
# Updates the API servers one at a time. Run on the load balancer.
#
# Each server leaves rotation, finishes what it was answering, takes the new binary, and goes back
# only after it answers /health itself. The load balancer's own view is never the readiness signal:
# this backend has `fall 100` with a 2-second interval, so HAProxy takes around 200 seconds to
# notice a server that stopped working.
#
# The rollout never installs anything itself. It delivers a binary and calls deploy.sh on the
# server, which is the same path a person takes there by hand.

set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly HERE

# shellcheck source-path=SCRIPTDIR source=lib.sh
source "${HERE}/lib.sh"

readonly HAPROXY="${HERE}/loadbalancer/haproxy.sh"
readonly REPOSITORY=unipept/unipept-api

# Defaults, overridden by rollout.conf beside this script.
INVENTORY="${HERE}/servers.conf"
HAPROXY_SOCKET=/run/haproxy/haproxy.sock
# The service user owns the binary directory and restarts its own user unit, so a deploy needs no
# privilege and the rollout carries no sudo.
SSH_USER=unipept
REMOTE_DEPLOY=/opt/unipept-api/lib/deploy.sh
# Above the API's own 150-second request timeout, so a draining server finishes or gives up first.
DRAIN_TIMEOUT=240
# The preloaded and hybrid builds read the index into memory before they answer.
READY_TIMEOUT=900

if [ -f "${HERE}/rollout.conf" ]; then
    # shellcheck source=/dev/null  # written on the load balancer, not in this repository.
    source "${HERE}/rollout.conf"
fi

export HAPROXY_SOCKET

usage() {
    cat >&2 <<'EOF'
usage: rollout.sh --version <tag> [options]

  --version <tag>     release to install, for example v2.6.0
  --only <name>       one server from the inventory, rather than all of them
  --dry-run           say what would happen, change nothing
  --allow-downtime    proceed even when draining leaves the backend with no server UP
  --inventory <path>  inventory file, default servers.conf beside this script

A server is drained from every backend its inventory line names, so routing the database
endpoints to their own backend needs no change here beyond that list.

Draining and restoring a server changes its HAProxy state, and this backend has email-alert
enabled, so each transition sends mail.
EOF
    exit 2
}

VERSION=''
ONLY=''
DRY_RUN=false
ALLOW_DOWNTIME=false

while [ $# -gt 0 ]; do
    case $1 in
        --version) VERSION=${2:-}; shift 2 ;;
        --only) ONLY=${2:-}; shift 2 ;;
        --inventory) INVENTORY=${2:-}; shift 2 ;;
        --dry-run) DRY_RUN=true; shift ;;
        --allow-downtime) ALLOW_DOWNTIME=true; shift ;;
        *) usage ;;
    esac
done

[ -n "$VERSION" ] || usage
[ -f "$INVENTORY" ] || die "no inventory at $INVENTORY"
require_cmd curl sha256sum socat ssh scp

# One server per line: name host port haproxy_backends haproxy_server
#
# haproxy_backends is comma-separated, because a server can sit in more than one backend: routing
# the database endpoints separately puts every server in two. A drain has to cover all of them, or
# the server keeps taking the traffic of the one that was missed.
#
# Comments and blank lines are ignored. The variant is not here: the server owns that, in its
# environment file, so adding a server is one edit rather than two.
read_inventory() {
    local name host port backends server
    while read -r name host port backends server _; do
        case ${name:-} in ''|\#*) continue ;; esac
        [ -n "$server" ] || die "inventory line for '${name}' has too few fields"
        [ -z "$ONLY" ] || [ "$ONLY" = "$name" ] || continue
        printf '%s %s %s %s %s\n' "$name" "$host" "$port" "$backends" "$server"
    done < "$INVENTORY"
}

# The backends one server sits in, one per line.
backends_of() {
    printf '%s\n' "${1//,/ }" | tr ' ' '\n' | grep -v '^$'
}

ssh_target() {
    if [ -n "$SSH_USER" ]; then printf '%s@%s\n' "$SSH_USER" "$1"; else printf '%s\n' "$1"; fi
}

on_server() {
    local host=$1; shift
    # shellcheck disable=SC2029  # the command is built here on purpose, not on the server.
    ssh -o BatchMode=yes "$(ssh_target "$host")" "${REMOTE_DEPLOY} $*"
}

# Downloads the release once, for every server to be fed from.
fetch_release() {
    local directory=$1 base="https://github.com/${REPOSITORY}/releases/download/${VERSION}"

    log "fetching ${VERSION}"
    curl -fsSL --retry 3 -o "${directory}/SHA256SUMS" "${base}/SHA256SUMS"

    local version=${VERSION#v} asset
    # Only the variants the inventory's servers actually ask for.
    while read -r name host _ _ _; do
        local variant
        variant=$(on_server "$host" status | sed -n 's/^variant=//p')
        [ -n "$variant" ] || die "${name} does not report a variant; check VARIANT in its environment file"
        asset="unipept-api-${version}-x86_64-linux-gnu-${variant}"
        if [ ! -f "${directory}/${asset}" ]; then
            log "fetching ${asset} for ${name}"
            curl -fsSL --retry 3 -o "${directory}/${asset}" "${base}/${asset}"
            verify_sha256 "${directory}/${asset}" "${directory}/SHA256SUMS"
        fi
        printf '%s %s\n' "$name" "$asset" >> "${directory}/variants"
    done < <(read_inventory)
}

# Refuses to start from a fleet that is already not well: a rollout is not the time to discover it.
preflight() {
    local name host port backends server failures=0

    while read -r name host port backends server; do
        local state backend
        while read -r backend; do
            state=$("$HAPROXY" state "${backend}/${server}")
            if [ "${state%% *}" != "UP" ]; then
                log "preflight: ${name} is ${state} in ${backend}, not UP"
                failures=$((failures + 1))
            fi
        done < <(backends_of "$backends")
        if [ "$(curl -s -o /dev/null -w '%{http_code}' --max-time 5 "http://${host}:${port}/health")" != "200" ]; then
            log "preflight: ${name} does not answer /health"
            failures=$((failures + 1))
        fi
    done < <(read_inventory)

    [ "$failures" -eq 0 ] || die "${failures} preflight problem(s); fix the fleet before rolling out"
    log "preflight passed"
}

# One server, start to finish. Anything that fails here rolls that server back and stops.
update_server() {
    local name=$1 host=$2 port=$3 backends=$4 server=$5 asset=$6 directory=$7
    local backend up

    log "=== ${name} ==="

    # Every backend it sits in has to keep a server, or that backend has nothing to answer with.
    while read -r backend; do
        up=$("$HAPROXY" up-count "$backend")
        if [ "$up" -le 1 ] && [ "$ALLOW_DOWNTIME" != true ]; then
            die "${server} is the only server UP in ${backend}; draining it is an outage. Pass --allow-downtime to accept that."
        fi
    done < <(backends_of "$backends")

    while read -r backend; do
        "$HAPROXY" drain "${backend}/${server}"
    done < <(backends_of "$backends")

    while read -r backend; do
        "$HAPROXY" wait-empty "${backend}/${server}" "$DRAIN_TIMEOUT"
        # Out of rotation entirely while it restarts, so the checks that must fail during startup
        # do not count towards `fall` and do not email twice.
        "$HAPROXY" maint "${backend}/${server}"
    done < <(backends_of "$backends")

    if ! rollout_binary "$name" "$host" "$port" "$asset" "$directory"; then
        log "${name} failed; rolling it back and leaving it out of rotation"
        on_server "$host" rollback --timeout "$READY_TIMEOUT" || log "${name} could not be rolled back either"
        die "stopped at ${name}; the servers after it were not touched"
    fi

    while read -r backend; do
        "$HAPROXY" ready "${backend}/${server}"
        "$HAPROXY" wait-up "${backend}/${server}" 60
    done < <(backends_of "$backends")

    log "${name} is back in rotation"
}

# Delivers the binary and lets deploy.sh install it. Returns non-zero for the caller to handle.
rollout_binary() {
    local name=$1 host=$2 port=$3 asset=$4 directory=$5
    local remote="/tmp/unipept-api-rollout.$$"

    ssh -o BatchMode=yes "$(ssh_target "$host")" "mkdir -p ${remote}" || return 1
    scp -q "${directory}/${asset}" "${directory}/SHA256SUMS" "$(ssh_target "$host"):${remote}/" || return 1

    on_server "$host" deploy --from "${remote}/${asset}" --timeout "$READY_TIMEOUT" || return 1
    ssh -o BatchMode=yes "$(ssh_target "$host")" "rm -rf ${remote}" || true

    # deploy.sh already waited for /health on the server itself. This asks from the load balancer,
    # which is the path that matters, and checks the database separately.
    wait_for_http "http://${host}:${port}/health" 60 || return 1
    if [ "$(curl -s -o /dev/null -w '%{http_code}' --max-time 5 "http://${host}:${port}/health/database")" != "200" ]; then
        log "${name} serves but its OpenSearch does not answer"
        return 1
    fi

    local installed
    installed=$(on_server "$host" status | sed -n 's/^version=//p')
    [ "$installed" = "${VERSION#v}" ] || {
        log "${name} reports ${installed:-nothing}, expected ${VERSION#v}"
        return 1
    }

    log "${name} is serving ${installed}"
}

main() {
    local directory
    directory=$(mktemp -d)
    # shellcheck disable=SC2064  # $directory is wanted now, not at trap time.
    trap "rm -rf '$directory'" EXIT

    [ -n "$(read_inventory)" ] || die "the inventory selects no server"

    if [ "$DRY_RUN" = true ]; then
        log "dry run: nothing is changed"
        while read -r name host port backends server; do
            local states=''
            while read -r backend; do
                states="${states}${backend}=$("$HAPROXY" state "${backend}/${server}") "
            done < <(backends_of "$backends")
            printf '%-10s %-22s server=%s %shealth=%s variant=%s\n' \
                "$name" "${host}:${port}" "$server" "$states" \
                "$(curl -s -o /dev/null -w '%{http_code}' --max-time 5 "http://${host}:${port}/health")" \
                "$(on_server "$host" status | sed -n 's/^variant=//p')"
        done < <(read_inventory)
        return 0
    fi

    preflight
    fetch_release "$directory"

    local name host port backends server asset
    while read -r name host port backends server; do
        asset=$(sed -n "s/^${name} //p" "${directory}/variants")
        update_server "$name" "$host" "$port" "$backends" "$server" "$asset" "$directory"
    done < <(read_inventory)

    log "rollout of ${VERSION} finished"
    while read -r name host port _ _; do
        printf '%-10s %s\n' "$name" "$(on_server "$host" status | tr '\n' ' ')"
    done < <(read_inventory)
}

main

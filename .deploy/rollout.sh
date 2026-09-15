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

# Defaults, overridden by rollout.conf beside this script.
INVENTORY="${HERE}/servers.conf"
HAPROXY_SOCKET=/run/haproxy/haproxy.sock
# The service user owns the binary directory and restarts its own user unit, so a deploy needs no
# privilege and the rollout carries no sudo.
SSH_USER=unipept
REMOTE_DEPLOY=/opt/unipept-api/lib/deploy.sh
DRAIN_TIMEOUT=$DEFAULT_DRAIN_TIMEOUT
READY_TIMEOUT=$DEFAULT_READY_TIMEOUT

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

# name -> the status line that server reported after its deploy, for the closing summary.
declare -A STATUS=()

while [ $# -gt 0 ]; do
    # Two arguments or usage: `shift 2` with one left fails, and set -e would exit before the
    # check below, saying nothing at all.
    case $1 in
        --version) [ $# -ge 2 ] || usage; VERSION=$2; shift 2 ;;
        --only) [ $# -ge 2 ] || usage; ONLY=$2; shift 2 ;;
        --inventory) [ $# -ge 2 ] || usage; INVENTORY=$2; shift 2 ;;
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

ssh_target() {
    if [ -n "$SSH_USER" ]; then printf '%s@%s\n' "$SSH_USER" "$1"; else printf '%s\n' "$1"; fi
}

# -n throughout: ssh reads its standard input to forward it, and these run inside `while read`
# loops whose standard input is the inventory. Without it the first call swallows the rest of the
# fleet and the rollout silently stops after one server.
on_server() {
    local host=$1; shift
    # shellcheck disable=SC2029  # the command is built here on purpose, not on the server.
    ssh -n -o BatchMode=yes "$(ssh_target "$host")" "${REMOTE_DEPLOY} $*"
}

# Downloads the release once, for every server to be fed from, and prints "name asset" per server.
#
# Servers can share a variant, so an asset already fetched is not fetched again.
fetch_release() {
    local directory=$1 name host variant asset

    log "fetching ${VERSION}"
    curl -fsSL --retry 3 -o "${directory}/SHA256SUMS" "$(release_url "$VERSION" SHA256SUMS)"

    while read -r name host _ _ _; do
        variant=$(on_server "$host" status | env_value variant)
        [ -n "$variant" ] || die "${name} does not report a variant; check VARIANT in its environment file"

        asset=$(asset_name "$VERSION" "$variant")
        if [ ! -f "${directory}/${asset}" ]; then
            log "fetching ${asset} for ${name}"
            curl -fsSL --retry 3 -o "${directory}/${asset}" "$(release_url "$VERSION" "$asset")"
            verify_sha256 "${directory}/${asset}" "${directory}/SHA256SUMS"
        fi
        printf '%s %s\n' "$name" "$asset"
    done < <(read_inventory)
}

# Refuses to start from a fleet that is already not well: a rollout is not the time to discover it.
preflight() {
    local name host port backends server failures=0

    while read -r name host port backends server; do
        local states
        states=$("$HAPROXY" states "${backends}/${server}")
        # Every backend has to read UP, possibly with a check counter after it.
        if printf '%s' "$states" | grep -qvE '^([[:alnum:]_.-]+=UP[^=]*)+$'; then
            log "preflight: ${name} is ${states}"
            failures=$((failures + 1))
        fi
        local route
        for route in /health /health/database; do
            if [ "$(http_code "http://${host}:${port}${route}")" != "200" ]; then
                log "preflight: ${name} does not answer ${route}"
                failures=$((failures + 1))
            fi
        done
    done < <(read_inventory)

    [ "$failures" -eq 0 ] || die "${failures} preflight problem(s); fix the fleet before rolling out"
    log "preflight passed"
}

# One server, start to finish. Anything that fails here rolls that server back and stops.
update_server() {
    local name=$1 host=$2 port=$3 backends=$4 server=$5 asset=$6 directory=$7
    local target="${backends}/${server}" thinnest

    log "=== ${name} ==="

    # Every backend it sits in has to keep a server, or that backend has nothing to answer with.
    thinnest=$("$HAPROXY" least-up "$backends")
    if [ "$thinnest" -le 1 ] && [ "$ALLOW_DOWNTIME" != true ]; then
        die "${server} is the only server UP in one of ${backends//,/, }; draining it is an outage. Pass --allow-downtime to accept that."
    fi

    # Until the binary is touched, nothing has changed on the server, so a failure here — a drain
    # that does not empty, say — has to put it back rather than leave it out of rotation for a
    # deploy that never happened.
    restore_on_failure() { "$HAPROXY" ready "$target" || log "could not restore ${target}; do it by hand"; }
    trap restore_on_failure ERR
    "$HAPROXY" drain "$target"
    "$HAPROXY" wait-empty "$target" "$DRAIN_TIMEOUT"
    # Out of rotation entirely while it restarts, so the checks that must fail during startup do
    # not count towards `fall` and do not email twice.
    "$HAPROXY" maint "$target"
    trap - ERR

    if ! rollout_binary "$name" "$host" "$port" "$asset" "$directory"; then
        log "${name} failed; rolling it back and leaving it out of rotation"
        on_server "$host" rollback --timeout "$READY_TIMEOUT" || log "${name} could not be rolled back either"
        die "stopped at ${name}; the servers after it were not touched"
    fi

    "$HAPROXY" ready "$target"
    "$HAPROXY" wait-up "$target" 60

    log "${name} is back in rotation"
}

# Delivers the binary and lets deploy.sh install it. Returns non-zero for the caller to handle.
rollout_binary() {
    local name=$1 host=$2 port=$3 asset=$4 directory=$5
    local remote="/tmp/unipept-api-rollout.$$"

    ssh -n -o BatchMode=yes "$(ssh_target "$host")" "mkdir -p ${remote}" || return 1

    # Cleared however this returns: a failed attempt would otherwise leave a release binary behind,
    # and the next attempt picks a new name rather than reusing it.
    clear_remote() { ssh -n -o BatchMode=yes "$(ssh_target "$host")" "rm -rf ${remote}" || true; }

    if ! scp -q "${directory}/${asset}" "${directory}/SHA256SUMS" "$(ssh_target "$host"):${remote}/"; then
        clear_remote
        return 1
    fi

    if ! on_server "$host" deploy --from "${remote}/${asset}" --timeout "$READY_TIMEOUT"; then
        clear_remote
        return 1
    fi
    clear_remote

    # deploy.sh already waited for /health on the server itself. This asks from the load balancer,
    # which is the path that matters, and checks the database separately.
    wait_for_http "http://${host}:${port}/health" 60 || return 1
    # Polled, not asked once: the health route gives OpenSearch 2 seconds, and a process that has
    # just restarted can miss that on its first connection without anything being wrong.
    if ! wait_for_http "http://${host}:${port}/health/database" 30; then
        log "${name} serves but its OpenSearch does not answer"
        return 1
    fi

    # Kept for the closing summary, so the rollout does not ask a second time for the same answer.
    STATUS[$name]=$(on_server "$host" status)

    local installed
    installed=$(printf '%s\n' "${STATUS[$name]}" | env_value version)
    [ "$installed" = "${VERSION#v}" ] || {
        log "${name} reports ${installed:-nothing}, expected ${VERSION#v}"
        return 1
    }

    log "${name} is serving ${installed}"
}

main() {
    local directory line name host port backends server

    # Once, rather than on every loop that wants it. Through a variable rather than a process
    # substitution: `read_inventory` dies on a malformed line, and a subshell's exit would leave
    # mapfile reporting success with a short fleet and the rollout calling that a finished run.
    local -a servers=()
    local inventory
    inventory=$(read_inventory)
    mapfile -t servers <<<"$inventory"
    if [ "${#servers[@]}" -eq 0 ] || [ -z "${servers[0]}" ]; then
        die "the inventory selects no server"
    fi

    if [ "$DRY_RUN" = true ]; then
        log "dry run: nothing is changed"
        for line in "${servers[@]}"; do
            read -r name host port backends server <<<"$line"
            printf '%-10s %-22s server=%s %shealth=%s variant=%s\n' \
                "$name" "${host}:${port}" "$server" \
                "$("$HAPROXY" states "${backends}/${server}")" \
                "$(http_code "http://${host}:${port}/health")" \
                "$(on_server "$host" status | env_value variant)"
        done
        return 0
    fi

    directory=$(mktemp -d)
    # shellcheck disable=SC2064  # $directory is wanted now, not at trap time.
    trap "rm -rf '$directory'" EXIT

    preflight

    # name -> asset, so the release is downloaded once however many servers share a variant. Read
    # into a variable first: a failure inside fetch_release has to stop the rollout before any
    # server is drained, and a process substitution would hide it.
    local fetched
    fetched=$(fetch_release "$directory")

    local -A asset_of=()
    while read -r name asset; do
        [ -n "$name" ] && asset_of[$name]=$asset
    done <<<"$fetched"

    for line in "${servers[@]}"; do
        read -r name host port backends server <<<"$line"
        update_server "$name" "$host" "$port" "$backends" "$server" \
            "${asset_of[$name]:?no asset resolved for ${name}}" "$directory"
    done

    log "rollout of ${VERSION} finished"
    for line in "${servers[@]}"; do
        read -r name _ <<<"$line"
        printf '%-10s %s\n' "$name" "$(printf '%s' "${STATUS[$name]:-unknown}" | tr '\n' ' ')"
    done
}

main

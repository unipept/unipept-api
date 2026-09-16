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
LOCK_FILE=/tmp/unipept-rollout.lock
# Empty means nobody is emailed; rollout.conf sets it.
NOTIFY_TO=''
NOTIFY_SMTP=127.0.0.1:25

if [ -f "${HERE}/rollout.conf" ]; then
    # shellcheck source=/dev/null  # written on the load balancer, not in this repository.
    source "${HERE}/rollout.conf"
fi

# What `die` raises when it is called from inside a subshell.
trap 'exit 1' USR1

export HAPROXY_SOCKET NOTIFY_TO NOTIFY_SMTP

usage() {
    cat >&2 <<'EOF'
usage: rollout.sh --version <tag> [options]
       rollout.sh status

  --version <tag>          release to install, for example v2.6.0
  --only <name>            one server from the inventory, rather than all of them
  --dry-run                say what would happen, change nothing
  --allow-downtime         proceed even when draining leaves a backend with no server UP
  --allow-index-mismatch   proceed even when the fleet does not agree on an index version
  --inventory <path>       inventory file, default servers.conf beside this script

  status                   read the fleet and change nothing: HAProxy state, version, variant and
                           index version per server. What to run after a rollout stopped part way.

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
ALLOW_INDEX_MISMATCH=false
STATUS_ONLY=false

# name -> the status line that server reported after its deploy, for the closing summary.
declare -A STATUS=()
# name -> the release asset that server takes, resolved once in phase 0.
declare -A ASSET_OF=()
# Hosts that have a staging directory, so the cleanup reaches every one of them.
STAGED_ON=''
# Servers this run left out of the pool or down, which is what the team is told about.
FAILED_UPDATE=''
NEEDS_ATTENTION=''
# Who to name in the record and the mail. SUDO_USER first, so a run through sudo names the person.
readonly RUN_BY="${SUDO_USER:-$(id -un)}"
# The load balancer's own scratch directory, and where a run stages on a server.
TMP_DIR=''
readonly REMOTE_STAGING="/tmp/unipept-api-rollout.$$"

while [ $# -gt 0 ]; do
    # Two arguments or usage: `shift 2` with one left fails, and set -e would exit before the
    # check below, saying nothing at all.
    case $1 in
        status) STATUS_ONLY=true; shift ;;
        --version) [ $# -ge 2 ] || usage; VERSION=$2; shift 2 ;;
        --only) [ $# -ge 2 ] || usage; ONLY=$2; shift 2 ;;
        --inventory) [ $# -ge 2 ] || usage; INVENTORY=$2; shift 2 ;;
        --dry-run) DRY_RUN=true; shift ;;
        --allow-downtime) ALLOW_DOWNTIME=true; shift ;;
        --allow-index-mismatch) ALLOW_INDEX_MISMATCH=true; shift ;;
        *) usage ;;
    esac
done

# `status` reads the fleet and needs no release to do it.
[ "$STATUS_ONLY" = true ] || [ -n "$VERSION" ] || usage
[ -f "$INVENTORY" ] || die "no inventory at $INVENTORY"
require_cmd curl sha256sum socat ssh scp flock logger

# One rollout at a time. Two runs would each read capacity before the other drained, so both would
# believe the backend could spare a server and between them empty it.
#
# flock releases when this process dies, however it dies, so an uncatchable death leaves no stale
# lock to clear by hand.
exec 9> "$LOCK_FILE"
flock -n 9 || die "another rollout holds ${LOCK_FILE}; wait for it, or check 'rollout.sh status'"

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

# The inventory is hand-edited, and two of its mistakes are silent: a repeated name makes one line
# unreachable through the asset map, and a repeated haproxy_server drains one host while updating
# another.
validate_inventory() {
    local name host port backends server seen_names='' seen_servers=''
    local lines=$1

    while read -r name host port backends server; do
        [ -n "$name" ] || continue
        case " ${seen_names} " in *" ${name} "*) die "the inventory names '${name}' twice" ;; esac
        case " ${seen_servers} " in *" ${server} "*) die "the inventory uses HAProxy server '${server}' twice" ;; esac
        seen_names="${seen_names} ${name}"
        seen_servers="${seen_servers} ${server}"

        case $port in
            '' | *[!0-9]*) die "${name} has port '${port}', which is not a number" ;;
        esac
    done <<<"$lines"
}

# Primaries first, backups last.
#
# A backup takes no traffic while the primaries are up, so updating it first would put the fleet's
# fallback on an unproven binary before anything had exercised it. Read from HAProxy rather than the
# inventory, so the order follows the configuration that is actually loaded.
ordered_servers() {
    local lines=$1 name host port backends server primaries='' backups=''

    while read -r name host port backends server; do
        [ -n "$name" ] || continue
        if "$HAPROXY" is-backup "${backends}/${server}"; then
            backups="${backups}${name} ${host} ${port} ${backends} ${server}"$'\n'
        else
            primaries="${primaries}${name} ${host} ${port} ${backends} ${server}"$'\n'
        fi
    done <<<"$lines"

    printf '%s%s' "$primaries" "$backups" | grep -v '^$' || true
}

# A dead connection has to fail rather than block: without these, a partitioned server holds the run
# open indefinitely with that server out of the pool.
readonly SSH_OPTIONS=(-n -o BatchMode=yes -o ConnectTimeout=10 -o ServerAliveInterval=15 -o ServerAliveCountMax=4)
readonly SCP_OPTIONS=(-q -o BatchMode=yes -o ConnectTimeout=10 -o ServerAliveInterval=15 -o ServerAliveCountMax=4)

ssh_target() {
    if [ -n "$SSH_USER" ]; then printf '%s@%s\n' "$SSH_USER" "$1"; else printf '%s\n' "$1"; fi
}

# -n throughout: ssh reads its standard input to forward it, and these run inside `while read`
# loops whose standard input is the inventory. Without it the first call swallows the rest of the
# fleet and the rollout silently stops after one server.
on_server() {
    local host=$1; shift
    # shellcheck disable=SC2029  # the command is built here on purpose, not on the server.
    ssh "${SSH_OPTIONS[@]}" "$(ssh_target "$host")" "${REMOTE_DEPLOY} $*"
}

# Downloads the release once, for every server to be fed from, and prints "name asset" per server.
#
# Servers can share a variant, so an asset already fetched is not fetched again.
fetch_release() {
    local directory=$1 lines=$2 name host variant asset report

    log "fetching ${VERSION}"
    curl -fsSL --retry 3 -o "${directory}/SHA256SUMS" "$(release_url "$VERSION" SHA256SUMS)"

    while read -r name host _ _ _; do
        [ -n "$name" ] || continue
        # An unreachable host and a host with no VARIANT are different problems and used to share a
        # message, which sent the reader looking in the wrong place.
        report=$(on_server "$host" status 2>/dev/null) ||
            die "cannot reach ${name} over ssh as ${SSH_USER:-the configured user}"

        variant=$(printf '%s\n' "$report" | env_value variant)
        if [ -z "$variant" ] || [ "$variant" = unknown ]; then
            die "${name} reports no variant; check VARIANT in its environment file"
        fi

        asset=$(asset_name "$VERSION" "$variant")
        if [ ! -f "${directory}/${asset}" ]; then
            log "fetching ${asset} for ${name}"
            curl -fsSL --retry 3 -o "${directory}/${asset}" "$(release_url "$VERSION" "$asset")"
            verify_sha256 "${directory}/${asset}" "${directory}/SHA256SUMS"
        fi
        ASSET_OF[$name]=$asset
    done <<<"$lines"
}

# Refuses to start from a fleet that cannot take the change, before anything is drained.
#
# Everything is collected rather than stopped at, so one run tells the operator the whole story. The
# binary is staged here too, and `deploy.sh check --from` executes it on each host: a build for the
# wrong architecture matches its checksum and still cannot run, and learning that from the first
# server means that server is already out of the pool.
preflight() {
    local lines=$1 name host port backends server failures=0 versions=''

    while read -r name host port backends server; do
        [ -n "$name" ] || continue
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

        # The asset this host asks for, delivered now and kept for phase 2, so the bytes that were
        # verified here are the bytes that get installed.
        local asset report
        asset=${ASSET_OF[$name]}
        if ! stage_on "$host" "$asset"; then
            log "preflight: could not reach ${name} over ssh"
            failures=$((failures + 1))
            continue
        fi

        if ! report=$(on_server "$host" check --from "${REMOTE_STAGING}/${asset}" 2>&1); then
            log "preflight: ${name} is not ready:"
            printf '%s\n' "$report" | sed 's/^/    /' >&2
            failures=$((failures + 1))
            continue
        fi

        local index_version
        index_version=$(printf '%s\n' "$report" | env_value index_version)
        versions="${versions}${name}=${index_version} "
    done <<<"$lines"

    [ "$failures" -eq 0 ] || die "${failures} preflight problem(s); nothing was touched"

    # One fleet, one index. Two servers on different UniProt versions answer the same request
    # differently depending on which one the load balancer picked, and every health check still
    # passes, so nothing else would ever notice.
    local distinct
    distinct=$(printf '%s' "$versions" | tr ' ' '\n' | sed 's/^[^=]*=//' | grep -v '^$' | sort -u | wc -l)
    if [ "$distinct" -gt 1 ]; then
        if [ "$ALLOW_INDEX_MISMATCH" != true ]; then
            die "the fleet does not agree on an index: ${versions}; pass --allow-index-mismatch to accept that"
        fi
        log "the fleet does not agree on an index: ${versions}"
    fi

    log "preflight passed: ${versions}"
}

# Puts the asset on a server, in a directory this run owns. Recorded so the cleanup can find it even
# if the run is interrupted between hosts.
stage_on() {
    local host=$1 asset=$2

    # shellcheck disable=SC2029  # the path is built here on purpose: this run owns it.
    ssh "${SSH_OPTIONS[@]}" "$(ssh_target "$host")" "mkdir -p ${REMOTE_STAGING}" || return 1
    STAGED_ON="${STAGED_ON} ${host}"
    scp "${SCP_OPTIONS[@]}" "${TMP_DIR}/${asset}" "${TMP_DIR}/SHA256SUMS" \
        "$(ssh_target "$host"):${REMOTE_STAGING}/" || return 1
}

# One server, start to finish. A failure here stops the run: the servers after it are never
# attempted, so a bad release can never take the whole fleet down.
update_server() {
    local name=$1 host=$2 port=$3 backends=$4 server=$5 asset=$6
    local target="${backends}/${server}" thinnest

    log "=== ${name} ==="

    # Preflight was minutes ago and capacity can move. Every backend it sits in has to keep a server.
    thinnest=$("$HAPROXY" least-up "$backends")
    if [ "$thinnest" -le 1 ] && [ "$ALLOW_DOWNTIME" != true ]; then
        die "${server} is the only server UP in one of ${backends//,/, }; draining it is an outage. Pass --allow-downtime to accept that."
    fi

    # Until the binary is touched nothing has changed on the server, so a failure here — or an
    # interrupt — puts it back rather than leaving it out for a deploy that never happened.
    restore_on_failure() { "$HAPROXY" ready "$target" || log "could not restore ${target}; do it by hand"; }
    trap restore_on_failure ERR INT TERM
    "$HAPROXY" drain "$target"
    "$HAPROXY" wait-empty "$target" "$DRAIN_TIMEOUT"
    # Out of rotation entirely while it restarts, so the checks that must fail during startup do not
    # count towards `fall` and do not email twice.
    "$HAPROXY" maint "$target"
    trap - ERR INT TERM

    if install_on "$name" "$host" "$port" "$asset"; then
        "$HAPROXY" ready "$target"
        "$HAPROXY" wait-up "$target" 60
        log "${name} is back in rotation"
        return 0
    fi

    # --no-rollback means the server has done nothing about the failure, so this decides. Ask it what
    # is actually true first: ssh failing is not the same as the deploy failing, and a partition can
    # leave a deploy that succeeded looking like one that did not.
    resolve_failure "$name" "$host" "$port" "$target"
}

# Decides what a failed deploy left behind, and acts once.
resolve_failure() {
    local name=$1 host=$2 port=$3 target=$4
    local report installed

    report=$(on_server "$host" status 2>/dev/null || true)
    installed=$(printf '%s\n' "$report" | env_value version)

    if [ "$installed" = "${VERSION#v}" ] && [ "$(http_code "http://${host}:${port}/health")" = "200" ]; then
        # The deploy worked; only the connection to it failed.
        log "${name} is serving ${installed} after all, so only the connection failed"
        STATUS[$name]=$report
        "$HAPROXY" ready "$target"
        "$HAPROXY" wait-up "$target" 60
        log "${name} is back in rotation"
        return 0
    fi

    if [ -n "$installed" ] && [ "$installed" != "${VERSION#v}" ]; then
        # Nothing was installed, so there is nothing to undo.
        log "${name} is still on ${installed}; nothing was installed"
        "$HAPROXY" ready "$target"
        note_failed_update "$name" "$installed"
        die "stopped at ${name}; the servers after it were not touched"
    fi

    log "${name} is not serving ${VERSION#v}; rolling it back"
    if on_server "$host" rollback --timeout "$READY_TIMEOUT"; then
        # It is serving a binary that was known good, and the load balancer has just watched it come
        # up, so keeping it out of the pool would cost capacity for nothing.
        if [ "$(http_code "http://${host}:${port}/health")" = "200" ] &&
           [ "$(http_code "http://${host}:${port}/health/database")" = "200" ]; then
            "$HAPROXY" ready "$target"
            "$HAPROXY" wait-up "$target" 60
            log "${name} was rolled back and is serving again"
            note_failed_update "$name" "$(on_server "$host" status 2>/dev/null | env_value version)"
            die "stopped at ${name}; the servers after it were not touched"
        fi
        log "${name} rolled back but does not answer; leaving it out of the pool"
    else
        log "${name} could not be rolled back"
    fi

    note_down "$name" "$target"
    die "stopped at ${name}; the servers after it were not touched"
}

# An update that failed but left the fleet serving. Worth an email, not an alarm.
note_failed_update() {
    FAILED_UPDATE="${FAILED_UPDATE}${1} (serving ${2:-unknown}) "
}

# A server nobody can route to. This is the case that has to reach a person.
note_down() {
    NEEDS_ATTENTION="${NEEDS_ATTENTION}${1} [${2}] "
}

# Installs the already-staged binary and verifies the result from here, rather than trusting what the
# server said about itself. Returns non-zero for the caller to resolve.
install_on() {
    local name=$1 host=$2 port=$3 asset=$4

    # --no-rollback: this side decides what a failure means, so the two of them cannot each roll back.
    on_server "$host" deploy --from "${REMOTE_STAGING}/${asset}" \
        --timeout "$READY_TIMEOUT" --no-rollback || return 1

    # deploy.sh already waited for /health on the server itself. This asks over the network, which is
    # the path that matters, and checks the database separately.
    wait_for_http "http://${host}:${port}/health" 60 || return 1
    # Polled, not asked once: the health route gives OpenSearch 2 seconds, and a process that has just
    # restarted can miss that on its first connection without anything being wrong.
    if ! wait_for_http "http://${host}:${port}/health/database" 30; then
        log "${name} serves but its OpenSearch does not answer"
        return 1
    fi

    # Kept for the closing summary, so the rollout does not ask twice for the same answer.
    STATUS[$name]=$(on_server "$host" status)

    local installed
    installed=$(printf '%s\n' "${STATUS[$name]}" | env_value version)
    [ "$installed" = "${VERSION#v}" ] || {
        log "${name} reports ${installed:-nothing}, expected ${VERSION#v}"
        return 1
    }

    log "${name} is serving ${installed}"
}

# Runs on every exit, including a signal, so nothing is left behind and nothing goes unreported.
finish() {
    local status=$? host

    for host in $STAGED_ON; do
        # shellcheck disable=SC2029  # the path is built here on purpose: this run owns it.
        ssh "${SSH_OPTIONS[@]}" "$(ssh_target "$host")" "rm -rf ${REMOTE_STAGING}" 2>/dev/null || \
            log "could not clear ${REMOTE_STAGING} on ${host}"
    done
    [ -n "$TMP_DIR" ] && rm -rf "$TMP_DIR"

    record_run "$status"

    if [ -n "$NEEDS_ATTENTION" ]; then
        notify "[unipept-rollout] a server needs attention on $(hostname -s)" \
"A rollout of ${VERSION} left a server that cannot be routed to.

  ${NEEDS_ATTENTION}

The servers after it were not attempted, so the rest of the fleet is untouched.

To see the fleet:      ${HERE}/rollout.sh status
To return a server:    ${HERE}/loadbalancer/haproxy.sh ready <backends>/<server>
On the server itself:  ${REMOTE_DEPLOY} status

Run by ${RUN_BY} on $(hostname -f 2>/dev/null || hostname)."
    elif [ -n "$FAILED_UPDATE" ]; then
        notify "[unipept-rollout] ${VERSION} was rolled back on $(hostname -s)" \
"A rollout of ${VERSION} stopped and the fleet is serving its previous version.

  ${FAILED_UPDATE}

Every server is in the pool. The servers after the failure were not attempted, so the fleet is
consistent only if this was the first one. Check with:

  ${HERE}/rollout.sh status

Run by ${RUN_BY} on $(hostname -f 2>/dev/null || hostname)."
    fi

    return "$status"
}

# One journal line per server, so "who deployed what, when" has an answer that outlives a terminal.
record_run() {
    local status=$1 name

    for name in "${!STATUS[@]}"; do
        logger -t unipept-rollout -- \
            "version=${VERSION} server=${name} by=${RUN_BY} outcome=deployed $(printf '%s' "${STATUS[$name]}" | tr '\n' ' ')"
    done
    for name in $FAILED_UPDATE; do
        case $name in \(*) continue ;; esac
        logger -t unipept-rollout -- "version=${VERSION} server=${name} by=${RUN_BY} outcome=rolled-back"
    done
    for name in $NEEDS_ATTENTION; do
        case $name in \[*) continue ;; esac
        logger -t unipept-rollout -- "version=${VERSION} server=${name} by=${RUN_BY} outcome=needs-attention"
    done
    logger -t unipept-rollout -- "version=${VERSION} by=${RUN_BY} exit=${status}"
}

# Reads the fleet without changing any of it. What to reach for after a run stopped part way, or
# when "which server is still out of the pool" needs an answer.
do_status() {
    local name host port backends server report inventory
    inventory=$(read_inventory) || exit 1

    printf '%-10s %-22s %-28s %-10s %-10s %s\n' SERVER ADDRESS HAPROXY VERSION VARIANT INDEX
    while read -r name host port backends server; do
        [ -n "$name" ] || continue
        report=$(on_server "$host" status 2>/dev/null || true)
        printf '%-10s %-22s %-28s %-10s %-10s %s\n' \
            "$name" "${host}:${port}" \
            "$("$HAPROXY" states "${backends}/${server}" 2>/dev/null || echo unreachable)" \
            "$(printf '%s\n' "$report" | env_value version)" \
            "$(printf '%s\n' "$report" | env_value variant)" \
            "$(on_server "$host" check 2>/dev/null | env_value index_version || echo '-')"
    done <<<"$inventory"
}

main() {
    # Read once and passed around, rather than re-read by each phase that wants it.
    local inventory
    inventory=$(read_inventory) || exit 1
    validate_inventory "$inventory"

    local -a servers=()
    mapfile -t servers < <(ordered_servers "$inventory")
    if [ "${#servers[@]}" -eq 0 ] || [ -z "${servers[0]}" ]; then
        die "the inventory selects no server"
    fi

    local line name host port backends server

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

    TMP_DIR=$(mktemp -d)

    # Phase 0: the release, once, for the whole fleet. No disk check here on purpose: a full disk
    # makes curl fail before anything is touched, and a truncated download fails its checksum. The
    # server side does check, because there a failed write lands in the middle of a swap.
    fetch_release "$TMP_DIR" "$inventory"

    # Phase 1: every server checked, and the binary staged, before one is drained.
    preflight "$inventory"

    # Phase 2: one at a time, the next only after this one is back in the pool.
    for line in "${servers[@]}"; do
        read -r name host port backends server <<<"$line"
        update_server "$name" "$host" "$port" "$backends" "$server" "${ASSET_OF[$name]}"
    done

    log "rollout of ${VERSION} finished"
    for line in "${servers[@]}"; do
        read -r name _ <<<"$line"
        printf '%-10s %s\n' "$name" "$(printf '%s' "${STATUS[$name]:-unknown}" | tr '\n' ' ')"
    done
}

# Phase 3 runs whatever happens, including on a signal.
trap finish EXIT
trap 'exit 130' INT TERM HUP

if [ "$STATUS_ONLY" = true ]; then
    do_status
else
    main
fi

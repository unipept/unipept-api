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
#
# Flow:
#   1. Load rollout.conf, parse the arguments, and take a lock, so only one rollout runs at a time.
#   2. A subcommand instead of a rollout: `status` prints what a run in progress is doing and then
#      the fleet; `abort` stops a run and waits for it to put its server back; `ready` returns
#      servers to the pool. None needs a release. `status` and `abort` take no lock, because both
#      are for running while a rollout is.
#   3. Read and validate the inventory, and order the servers primaries first, backups last.
#   4. --dry-run prints what each server holds and how the load balancer sees it, and stops.
#   5. Phase 0: download the release on the load balancer, once per variant the fleet asks for, and
#      say what the fleet is running now — a split fleet is reported, never refused, because
#      rolling out again is how it is put right.
#   6. Phase 1: preflight. Every server has to be UP in each backend it names and answer /health
#      and /health/database; its asset is delivered to it and `deploy.sh check --from` runs there;
#      and the fleet has to agree on one index version. A problem here stops the run, with nothing
#      drained and nothing installed.
#   7. Phase 2: one server at a time, in that order. Check the backends can spare it, drain it,
#      wait for its connections to end, put it in maintenance, install the staged binary through
#      `deploy.sh deploy --no-rollback` on the deadline that server asked for, confirm /health and
#      /health/database over the network, and return it to the pool only then. A failure asks the
#      server what actually happened, acts on the answer, and stops the run, so the servers after
#      it are never touched.
#   8. Phase 3: on every exit, including a signal. Clear the staging directory on every server it
#      reached, write one journal line per server, and email if an update failed or a server is out
#      of the pool — a server this run drained and never put back included, which is what an
#      interrupted install leaves behind.

set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly HERE

# shellcheck source-path=SCRIPTDIR source=lib.sh
source "${HERE}/lib.sh"

readonly HAPROXY="${HERE}/loadbalancer/haproxy.sh"

# This host's own settings live outside the checkout, because the inventory names the fleet and the
# configuration names where failures are emailed. Beside the script is the fallback, so running from
# a checkout still works while developing.
CONFIG_DIR=/etc/unipept-rollout
[ -d "$CONFIG_DIR" ] || CONFIG_DIR=$HERE

# Defaults, overridden by rollout.conf.
INVENTORY="${CONFIG_DIR}/servers.conf"
HAPROXY_SOCKET=/run/haproxy/haproxy.sock
# The service user owns the binary directory and restarts its own user unit, so a deploy needs no
# privilege and the rollout carries no sudo.
SSH_USER=unipept
REMOTE_DEPLOY=/opt/unipept-api/lib/deploy.sh
DRAIN_TIMEOUT=$DEFAULT_DRAIN_TIMEOUT
# Stands in for a server that reports no deadline of its own, which is one still running an older
# deploy.sh. Each server's own READY_TIMEOUT is what a rollout uses where it has one.
READY_TIMEOUT=$DEFAULT_READY_TIMEOUT
# Seconds to give a server to look healthy once its deploy is finished: on its own two health routes,
# and then in HAProxy. Separate from READY_TIMEOUT, which covers a service reading its index. By here
# it is already answering, and this is only the time to confirm it.
HEALTH_TIMEOUT=60
LOCK_FILE=/tmp/unipept-rollout.lock
# What the run in progress is doing, for `status` to read and `abort` to signal. Beside the lock
# rather than in it: the lock is opened with `exec 9>`, which truncates, and rewriting through a
# held descriptor needs seeking this has no reason to do.
RUN_STATE=
# Empty means nobody is emailed; rollout.conf sets it.
NOTIFY_TO=''
NOTIFY_SMTP=127.0.0.1:25

if [ -f "${CONFIG_DIR}/rollout.conf" ]; then
    # shellcheck source=/dev/null  # written on the load balancer, not in this repository.
    source "${CONFIG_DIR}/rollout.conf"
fi

# After the configuration, so a LOCK_FILE set there takes its state file with it.
[ -n "$RUN_STATE" ] || RUN_STATE="${LOCK_FILE%.lock}.state"

# What `die` raises when it is called from inside a subshell.
trap 'exit 1' USR1

export HAPROXY_SOCKET NOTIFY_TO NOTIFY_SMTP

usage() {
    cat >&2 <<'EOF'
usage: rollout.sh --version <tag> [options]
       rollout.sh status
       rollout.sh abort
       rollout.sh ready [<name> ...]

  --version <tag>          release to install, for example v2.6.0
  --only <name>            one server from the inventory, rather than all of them
  --dry-run                say what would happen, change nothing
  --allow-downtime         proceed even when draining leaves a backend with no server UP
  --allow-index-mismatch   proceed even when the fleet does not agree on an index version
  --inventory <path>       inventory file, default servers.conf beside this script

  status                   read the fleet and change nothing: HAProxy state, version, variant and
                           index version per server, and what a rollout in progress is doing. What
                           to run after a rollout stopped part way.
  abort                    stop the rollout that is running and wait for it to put its server back.
  ready [<name> ...]       return servers to the pool, named ones or every one that is out. Each
                           has to answer both health routes first, which is what makes this
                           different from calling haproxy.sh by hand.

A server is drained from every backend its inventory line names, so routing the database
endpoints to their own backend needs no change here beyond that list.

Draining and restoring a server changes its HAProxy state, and this backend has email-alert
enabled, so each transition sends mail.
EOF
    exit 2
}

# The subcommand, if one was given. Empty means a rollout.
COMMAND=''
VERSION=''
ONLY=''
DRY_RUN=false
ALLOW_DOWNTIME=false
ALLOW_INDEX_MISMATCH=false

# name -> the status line that server reported after its deploy, for the closing summary.
declare -A STATUS=()
# name -> the release asset that server takes, resolved once in phase 0.
declare -A ASSET_OF=()
# name -> the seconds that server asks to be given to answer /health, read in phase 1. A host whose
# index is not resident needs far longer than the rest, and it is the one that knows how long.
declare -A TIMEOUT_OF=()
# name -> the version that server was running before this rollout touched it, read in phase 0.
declare -A VERSION_BEFORE=()
# Hosts that have a staging directory, so the cleanup reaches every one of them.
STAGED_ON=''
# Servers this run left out of the pool or down, which is what the team is told about.
FAILED_UPDATE=''
NEEDS_ATTENTION=''
# The same servers, names only, for the record to iterate over.
FAILED_NAMES=''
DOWN_NAMES=''
# Who to name in the record and the mail. SUDO_USER first, so a run through sudo names the person.
readonly RUN_BY="${SUDO_USER:-$(id -un)}"
STARTED_AT="$(date -u '+%Y-%m-%dT%H:%M:%SZ')"
readonly STARTED_AT
# Everyone logs in as the same account, so the name alone cannot say who ran this. The address they
# came from is not attribution, but it is the difference between "someone" and "someone at that
# machine" when a record is read back months later.
ssh_connection=${SSH_CONNECTION:-}
readonly RUN_FROM="${ssh_connection%% *}"
# The load balancer's own scratch directory, and where a run stages on a server.
TMP_DIR=''
# The server this run has taken out of the pool, for as long as it is out. `finish` is all that still
# runs after a signal, and without this it cannot tell that a server was left in maintenance: an
# interrupt during an install emailed nobody and recorded nothing.
CURRENT_NAME=''
CURRENT_TARGET=''
readonly REMOTE_STAGING="/tmp/unipept-api-rollout.$$"

while [ $# -gt 0 ]; do
    # Two arguments or usage: `shift 2` with one left fails, and set -e would exit before the
    # check below, saying nothing at all.
    case $1 in
        status | abort | ready) COMMAND=$1; shift; break ;;
        --version) [ $# -ge 2 ] || usage; VERSION=$2; shift 2 ;;
        --only) [ $# -ge 2 ] || usage; ONLY=$2; shift 2 ;;
        --inventory) [ $# -ge 2 ] || usage; INVENTORY=$2; shift 2 ;;
        --dry-run) DRY_RUN=true; shift ;;
        --allow-downtime) ALLOW_DOWNTIME=true; shift ;;
        --allow-index-mismatch) ALLOW_INDEX_MISMATCH=true; shift ;;
        *) usage ;;
    esac
done

# Only a rollout needs a release. The subcommands read the fleet, or act on what a run left.
[ -n "$COMMAND" ] || [ -n "$VERSION" ] || usage
[ -f "$INVENTORY" ] || die "no inventory at $INVENTORY"
require_cmd curl sha256sum socat ssh scp flock logger

# One rollout at a time. Two runs would each read capacity before the other drained, so both would
# believe the backend could spare a server and between them empty it. `ready` takes it too, because
# it moves servers in and out of the pool and a rollout is counting them.
#
# flock releases when this process dies, however it dies, so an uncatchable death leaves no stale
# lock to clear by hand.
#
# `status` and `abort` take nothing. Both exist to be run while a rollout is going: one reads, and
# the other has to find the lock held to have anything to do. Taking it here meant `status` failed
# during a rollout, which is the one time it is worth running — and the message below said to run
# it.
#
# Opened for reading, which is all flock needs and is what makes the lock usable by both the
# operator and root. Opened for writing, a file root created is refused to the operator — and bash
# reports that itself and carries on with the descriptor unopened, so `flock` then failed on a bad
# descriptor and this said another rollout was holding a lock that nobody held.
case $COMMAND in
    status | abort) ;;
    *)
        if [ ! -e "$LOCK_FILE" ]; then
            : > "$LOCK_FILE" 2>/dev/null ||
                die "cannot create ${LOCK_FILE}; set LOCK_FILE in rollout.conf to a path this account can write"
        fi
        exec 9< "$LOCK_FILE" ||
            die "cannot read ${LOCK_FILE}, which belongs to $(stat -c %U "$LOCK_FILE" 2>/dev/null || echo someone)"
        flock -n 9 || die "another rollout holds ${LOCK_FILE}; wait for it, or run 'rollout.sh status'"
        ;;
esac

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
# open indefinitely with that server out of the pool. The bounds are in lib.sh, so the load
# balancer's install audit reaches a server on the same ones.
readonly SSH_OPTIONS=(-n "${SSH_CONNECTION_BOUNDS[@]}")
readonly SCP_OPTIONS=(-q "${SSH_CONNECTION_BOUNDS[@]}")

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
    curl "${CURL_DOWNLOAD[@]}" -o "${directory}/SHA256SUMS" "$(release_url "$VERSION" SHA256SUMS)"

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

        # What it is running now, for the report below. Free here: this is the same answer the
        # variant came out of.
        VERSION_BEFORE[$name]=$(printf '%s\n' "$report" | env_value version)

        asset=$(asset_name "$VERSION" "$variant")
        if [ ! -f "${directory}/${asset}" ]; then
            log "fetching ${asset} for ${name}"
            curl "${CURL_DOWNLOAD[@]}" -o "${directory}/${asset}" "$(release_url "$VERSION" "$asset")"
            verify_sha256 "${directory}/${asset}" "${directory}/SHA256SUMS"
        fi
        ASSET_OF[$name]=$asset
    done <<<"$lines"

    report_fleet_versions
}

# How many different values a run of `name=value ` entries holds.
#
# Two callers ask it of two different things — the version each server is serving, and the index
# each one reads — and both only ever compare the answer against 1. Counted here rather than in each
# of them, so a change to how those entries are built cannot leave one caller reading them the old
# way and quietly agreeing that a split fleet is on one version.
distinct_values() {
    printf '%s' "$1" | tr ' ' '\n' | sed 's/^[^=]*=//' | grep -v '^$' | sort -u | wc -l
}

# What the fleet is running before anything is installed.
#
# Reported, never refused. A fleet that disagrees is what a run which stopped part way leaves
# behind, and rolling out again is how that is put right — so refusing here would block the recovery
# rather than protect anything. The index check is the one that refuses, because two servers on
# different index versions answer the same request differently and no health check ever notices.
#
# Said out loud because nothing else says it. After a run stopped at the second of three servers,
# the next rollout used to start with no mention that the fleet was split.
report_fleet_versions() {
    local name versions='' distinct target="${VERSION#v}" already=''

    for name in "${!VERSION_BEFORE[@]}"; do
        versions="${versions}${name}=${VERSION_BEFORE[$name]:-unknown} "
        [ "${VERSION_BEFORE[$name]}" = "$target" ] && already="${already}${name} "
    done

    distinct=$(distinct_values "$versions")
    if [ "$distinct" -gt 1 ]; then
        log "the fleet is not on one version: ${versions}"
        log "rolling out ${VERSION} to all of it is what puts that right"
    else
        log "the fleet is on ${versions}"
    fi

    # Not a reason to stop: a server that already has the version still has to be proven to serve
    # it, and re-installing the same binary is what the rest of this run does anyway.
    [ -z "$already" ] || log "already on ${target}: ${already}"
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

        # What this host asks to be given to answer /health. A server still running an older
        # deploy.sh reports none, and then this load balancer's own setting stands in.
        local asked
        asked=$(printf '%s\n' "$report" | env_value ready_timeout)
        case ${asked:-} in
            '' | *[!0-9]*) asked=$READY_TIMEOUT ;;
        esac
        TIMEOUT_OF[$name]=$asked
    done <<<"$lines"

    [ "$failures" -eq 0 ] || die "${failures} preflight problem(s); nothing was touched"

    # One fleet, one index. Two servers on different UniProt versions answer the same request
    # differently depending on which one the load balancer picked, and every health check still
    # passes, so nothing else would ever notice.
    local distinct
    distinct=$(distinct_values "$versions")
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

# Puts a server back where a failure or a signal arrived before its binary was touched. Nothing has
# changed on it at that point, so it belongs in the pool. A restore that fails leaves the run's own
# record of it standing, so `finish` still reports what is out there.
restore_target() {
    local target=$1

    if "$HAPROXY" ready "$target"; then
        CURRENT_TARGET=''
    else
        log "could not restore ${target}; do it by hand"
    fi
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

    # From the drain until the server is back, this is the one server outside the pool, and after a
    # signal `finish` is the only thing left to say so.
    CURRENT_NAME=$name
    CURRENT_TARGET=$target

    # Until the binary is touched nothing has changed on the server, so a failure here — or an
    # interrupt — puts it back rather than leaving it out for a deploy that never happened.
    #
    # The handler exits. Restoring and then carrying on would drain, put the server back, and go on
    # to install anyway, which is the opposite of what an interrupt asks for. 130 is what the global
    # handler uses, so the two agree.
    restore_on_failure() {
        restore_target "$target"
        exit 130
    }
    trap restore_on_failure INT TERM
    trap 'restore_target "$target"' ERR

    "$HAPROXY" drain "$target"
    "$HAPROXY" wait-empty "$target" "$DRAIN_TIMEOUT"
    # Out of rotation entirely while it restarts, so the checks that must fail during startup do not
    # count towards `fall` and do not email twice.
    "$HAPROXY" maint "$target"

    # Back to the global handlers rather than to none: `trap -` would leave the rest of the run with
    # no INT or TERM handler at all, so a signal would reach `finish` with a zero status and the
    # journal would record an aborted rollout as a clean one.
    trap - ERR
    trap 'exit 130' INT TERM

    if install_on "$name" "$host" "$port" "$asset"; then
        "$HAPROXY" ready "$target"
        "$HAPROXY" wait-up "$target" "$HEALTH_TIMEOUT"
        CURRENT_TARGET=''
        log "${name} is back in rotation"
        return 0
    fi

    # --no-rollback means the server has done nothing about the failure, so this decides. Ask it what
    # is actually true first: ssh failing is not the same as the deploy failing, and a partition can
    # leave a deploy that succeeded looking like one that did not.
    resolve_failure "$name" "$host" "$port" "$target"
}

# Whether a server answers on both routes. A server is only worth returning to the pool when it does:
# db_handlers routes to it as well, so serving /health alone is not enough.
#
# Polled rather than asked once. A process that has just been restarted can miss a first connection
# without anything being wrong — the database route the more easily, since the health route gives
# OpenSearch only two seconds of its own. A single sample was leaving a server that is fine out of
# the pool, and mailing about it.
#
# The deadline is the caller's, because the two ask different questions. Returning a server to the
# pool is a gate and waits; asking whether a failed deploy is serving after all is a diagnosis, and a
# long wait there only delays the rollback that the answer leads to.
serving() {
    local host=$1 port=$2 timeout=$3
    wait_for_http "http://${host}:${port}/health" "$timeout" &&
        wait_for_http "http://${host}:${port}/health/database" "$timeout"
}

# Puts a server back, or says why it is being left out. Every path back into the pool goes through
# here, so none of them can forget to look first.
return_to_pool() {
    local name=$1 host=$2 port=$3 target=$4

    if ! serving "$host" "$port" "$HEALTH_TIMEOUT"; then
        log "${name} does not answer both health routes; leaving it out of the pool"
        note_down "$name" "$target"
        return 1
    fi

    # Checked rather than left to `set -e`, which is not in force here: bash turns it off for the
    # whole body of a function called as a condition, and three of the four callers do exactly that
    # (`|| die`, `|| true`, `if return_to_pool`). A failing `ready` used to carry on to the log line
    # below and return 0, so a server the load balancer never took back was reported as in rotation
    # — the one path that ended with a server out of the pool and nobody told.
    if ! "$HAPROXY" ready "$target" || ! "$HAPROXY" wait-up "$target" "$HEALTH_TIMEOUT"; then
        log "${name} answers both health routes, but the load balancer did not take it back"
        note_down "$name" "$target"
        return 1
    fi

    CURRENT_TARGET=''
    log "${name} is back in rotation"
}

# Decides what a failed deploy left behind, and acts once.
resolve_failure() {
    local name=$1 host=$2 port=$3 target=$4
    local report installed

    # An unreachable server reports nothing, and nothing is not evidence: rolling back over the same
    # dead connection would be guessing, and could undo a deploy that worked.
    if ! report=$(on_server "$host" status 2>/dev/null); then
        log "${name} cannot be reached to ask what happened; leaving it out of the pool"
        note_down "$name" "$target"
        die "stopped at ${name}; the servers after it were not touched"
    fi
    installed=$(printf '%s\n' "$report" | env_value version)

    # install_on has just polled both routes for HEALTH_TIMEOUT and they failed, so this is not
    # asking again — it is asking whether the answer came over a connection that died. Ten seconds
    # covers one missed connection; anything longer is a rollback held up for nothing.
    if [ "$installed" = "${VERSION#v}" ] && serving "$host" "$port" 10; then
        # The deploy worked; only the connection to it failed.
        log "${name} is serving ${installed} after all, so only the connection failed"
        STATUS[$name]=$report
        return_to_pool "$name" "$host" "$port" "$target" ||
            die "stopped at ${name}; the servers after it were not touched"
        return 0
    fi

    if [ -n "$installed" ] && [ "$installed" != "${VERSION#v}" ]; then
        # Nothing was installed, so there is nothing to undo — but it still has to be serving before
        # it goes back into the pool.
        log "${name} is still on ${installed}; nothing was installed"
        return_to_pool "$name" "$host" "$port" "$target" || true
        note_failed_update "$name" "$installed"
        die "stopped at ${name}; the servers after it were not touched"
    fi

    log "${name} is not serving ${VERSION#v}; rolling it back"
    if on_server "$host" rollback --timeout "${TIMEOUT_OF[$name]:-$READY_TIMEOUT}"; then
        # It is serving a binary that was known good, and the load balancer has just watched it come
        # up, so keeping it out of the pool would cost capacity for nothing.
        if return_to_pool "$name" "$host" "$port" "$target"; then
            log "${name} was rolled back and is serving again"
            note_failed_update "$name" "$(on_server "$host" status 2>/dev/null | env_value version || true)"
        fi
        # A rollback that came up but could not be returned has already been reported: every path
        # return_to_pool refuses goes through note_down first. Saying it again here named the server
        # twice in the same mail and wrote two needs-attention lines for the one host.
    else
        log "${name} could not be rolled back"
        note_down "$name" "$target"
    fi

    die "stopped at ${name}; the servers after it were not touched"
}

# An update that failed but left the fleet serving. Worth an email, not an alarm.
#
# Two lists: one to read, one to iterate. Splitting "patty (serving 2.5.3)" on whitespace was logging
# a journal line that claimed the server was called "2.5.3)".
note_failed_update() {
    FAILED_UPDATE="${FAILED_UPDATE}${1} (serving ${2:-unknown}) "
    FAILED_NAMES="${FAILED_NAMES}${1} "
}

# A server nobody can route to. This is the case that has to reach a person.
note_down() {
    NEEDS_ATTENTION="${NEEDS_ATTENTION}${1} [${2}] "
    DOWN_NAMES="${DOWN_NAMES}${1} "
    # Left out of the pool, but said so. `finish` reports only what nothing else did.
    CURRENT_TARGET=''
}

# Installs the already-staged binary and verifies the result from here, rather than trusting what the
# server said about itself. Returns non-zero for the caller to resolve.
install_on() {
    local name=$1 host=$2 port=$3 asset=$4

    # --no-rollback: this side decides what a failure means, so the two of them cannot each roll back.
    on_server "$host" deploy --from "${REMOTE_STAGING}/${asset}" \
        --timeout "${TIMEOUT_OF[$name]:-$READY_TIMEOUT}" --no-rollback || return 1

    # deploy.sh already waited for /health on the server itself. This asks over the network, which is
    # the path that matters, and checks the database separately.
    wait_for_http "http://${host}:${port}/health" "$HEALTH_TIMEOUT" || return 1
    # Polled, not asked once: the health route gives OpenSearch 2 seconds, and a process that has just
    # restarted can miss that on its first connection without anything being wrong.
    if ! wait_for_http "http://${host}:${port}/health/database" "$HEALTH_TIMEOUT"; then
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

    # A server this run drained and never put back. Only a signal during the install arrives here
    # with this still set: every decided failure has already reported itself through note_down.
    #
    # Reported, not returned to the pool. deploy.sh has its own HUP handler and may be rolling the
    # binary back at this moment, so what the server is serving is not known from here, and putting
    # it back could route traffic at a host that is about to restart. Saying so is what was missing:
    # an interrupt used to leave a server in maintenance with no mail and no journal line naming it.
    if [ -n "$CURRENT_TARGET" ]; then
        log "${CURRENT_NAME} was left out of the pool by a run that did not finish"
        note_down "$CURRENT_NAME" "$CURRENT_TARGET"
    fi

    for host in $STAGED_ON; do
        # shellcheck disable=SC2029  # the path is built here on purpose: this run owns it.
        ssh "${SSH_OPTIONS[@]}" "$(ssh_target "$host")" "rm -rf ${REMOTE_STAGING}" 2>/dev/null || \
            log "could not clear ${REMOTE_STAGING} on ${host}"
    done
    [ -n "$TMP_DIR" ] && rm -rf "$TMP_DIR"
    # Only a run that wrote one clears it, so a recovery command cannot delete the state of a
    # rollout that is still going. Quietly, because a file this run could not own is already
    # reported where it is prepared, and failing to clear it must not be the last word of a rollout
    # that worked.
    if [ -n "$VERSION" ] && [ -n "$RUN_STATE" ]; then
        rm -f "$RUN_STATE" 2>/dev/null || true
    fi

    record_run "$status"

    # Two texts for the one state, because a subcommand reaches it too. `ready` takes no --version
    # and attempts no fleet, so the rollout wording mailed "A rollout of  left a server" and sent
    # the operator looking for servers after it that were never part of the run. Keyed the way
    # record_run is keyed: a run that set out to change something has a VERSION, and nothing else
    # does.
    if [ -n "$NEEDS_ATTENTION" ] && [ -n "$VERSION" ]; then
        notify "[unipept-rollout] a server needs attention on $(hostname -s)" \
"A rollout of ${VERSION} left a server that cannot be routed to.

  ${NEEDS_ATTENTION}

The servers after it were not attempted, so the rest of the fleet is untouched.

To see the fleet:      ${HERE}/rollout.sh status
To return a server:    ${HERE}/loadbalancer/haproxy.sh ready <backends>/<server>
On the server itself:  ${REMOTE_DEPLOY} status

Run by ${RUN_BY} on $(hostname -f 2>/dev/null || hostname)."
    elif [ -n "$NEEDS_ATTENTION" ]; then
        notify "[unipept-rollout] a server needs attention on $(hostname -s)" \
"'rollout.sh ${COMMAND}' could not return a server to the pool.

  ${NEEDS_ATTENTION}

No rollout was running, so nothing else on the fleet was touched.

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
#
# Only for a run that set out to change something. `status` and the recovery commands take no
# --version, so recording them wrote `version= ... exit=0` and read back as a rollout of nothing.
record_run() {
    local status=$1 name

    [ -n "$VERSION" ] || return 0

    for name in "${!STATUS[@]}"; do
        logger -t unipept-rollout -- \
            "version=${VERSION} server=${name} by=${RUN_BY} from=${RUN_FROM:-local} outcome=deployed $(printf '%s' "${STATUS[$name]}" | tr '\n' ' ')"
    done
    for name in $FAILED_NAMES; do
        logger -t unipept-rollout -- "version=${VERSION} server=${name} by=${RUN_BY} from=${RUN_FROM:-local} outcome=rolled-back"
    done
    for name in $DOWN_NAMES; do
        logger -t unipept-rollout -- "version=${VERSION} server=${name} by=${RUN_BY} from=${RUN_FROM:-local} outcome=needs-attention"
    done
    logger -t unipept-rollout -- "version=${VERSION} by=${RUN_BY} from=${RUN_FROM:-local} exit=${status}"
}

# Makes the state file writable by this run before anything depends on it.
#
# Unlike the lock, this one is written, and the operator and root take it in turns: a file either
# leaves behind at the default mode is one the other cannot rewrite. `note_phase` tolerates a failed
# write so a rollout is never lost to one, which is exactly why it has to be settled here instead —
# a silent failure there leaves `status` and `abort` reading a phase that has moved on.
prepare_run_state() {
    [ -n "$RUN_STATE" ] || return 0

    if [ -e "$RUN_STATE" ] && [ ! -w "$RUN_STATE" ]; then
        # Naming the owner because they are the only one who can clear it: /tmp is sticky, so this
        # account cannot remove a file it does not own however writable the directory looks.
        die "${RUN_STATE} belongs to $(stat -c %U "$RUN_STATE" 2>/dev/null || echo someone), who has to remove it, or set LOCK_FILE in rollout.conf to a path $(id -un) owns"
    fi
    # 0666 on creation, because the next run is as likely to be the other account. The load balancer
    # carries operator logins only, and /tmp is sticky, so nobody else can replace it.
    if [ ! -e "$RUN_STATE" ]; then
        (umask 0 && : > "$RUN_STATE") 2>/dev/null ||
            die "cannot create ${RUN_STATE}; set LOCK_FILE in rollout.conf to a path this account can write"
    fi
}

# Says what this run is doing, for `status` to read and `abort` to signal.
#
# Rewritten whole each time rather than appended to, so reading it never has to decide which of two
# phases is the current one.
note_phase() {
    local phase=$1 server=${2:-}

    [ -n "$RUN_STATE" ] || return 0
    {
        printf 'pid=%s\n' "$$"
        printf 'version=%s\n' "$VERSION"
        printf 'phase=%s\n' "$phase"
        printf 'server=%s\n' "$server"
        printf 'started=%s\n' "$STARTED_AT"
        printf 'by=%s\n' "$RUN_BY"
    } > "$RUN_STATE" 2>/dev/null || true
}

# Whether a rollout is running, decided by the lock rather than by the state file.
#
# A run killed uncatchably leaves its state file behind, and nothing in the file can say so. The
# lock cannot outlive the process that held it, so taking it is the test: if it can be taken, the
# file is leftovers.
a_run_is_in_progress() {
    # No lock file, no run — and said without creating one. `flock` would make it, and a file this
    # leaves behind as root is one the operator's next rollout has to work around.
    [ -e "$LOCK_FILE" ] || return 1

    # `flock <file> <command>` opens the file itself, so this needs no descriptor of its own. An
    # `exec` to get one would redirect this shell for good rather than for the call: `exec 8> file
    # 2>/dev/null` sends stderr to /dev/null permanently, and every message after it disappears.
    #
    # Taking the lock and letting go is the whole test. A failure for any other reason reads as a
    # run in progress, which is the answer that refuses to act.
    ! flock -n "$LOCK_FILE" true
}

# Reads the fleet without changing any of it. What to reach for after a run stopped part way, or
# when "which server is still out of the pool" needs an answer.
do_status() {
    local name host port backends server report inventory

    # Before the fleet, because it changes what the fleet below means: a server out of the pool is
    # expected while a rollout is working on it, and needs attention once nothing is.
    if a_run_is_in_progress; then
        if [ -f "$RUN_STATE" ]; then
            log "a rollout of $(env_value version "$RUN_STATE") is $(env_value phase "$RUN_STATE")$(
                s=$(env_value server "$RUN_STATE"); [ -n "$s" ] && printf ' %s' "$s")"
            log "started $(env_value started "$RUN_STATE") by $(env_value by "$RUN_STATE"), pid $(env_value pid "$RUN_STATE")"
            log "to stop it: ${HERE}/rollout.sh abort"
        else
            log "a rollout holds ${LOCK_FILE} but wrote no state file"
        fi
    else
        # Leftovers from a run that was killed uncatchably. Said rather than deleted: it names what
        # was going on when the machine stopped, and the fleet below is what it left.
        [ -f "$RUN_STATE" ] && log "no rollout is running; ${RUN_STATE} is from one that did not finish"
        log "no rollout is running"
    fi
    printf '\n' >&2

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

# Stops a rollout that is running, from anywhere.
#
# The run restores the server it drained through its own handlers, so this only has to reach them.
# Ctrl-C cannot, from another terminal: the signal has to go to that process, and until the state
# file existed there was nothing that said which one it is.
do_abort() {
    local pid

    a_run_is_in_progress || die "no rollout is running; ${LOCK_FILE} is free"
    [ -f "$RUN_STATE" ] || die "a rollout holds ${LOCK_FILE} but wrote no ${RUN_STATE}; find it with 'ps'"

    pid=$(env_value pid "$RUN_STATE")
    case ${pid:-} in
        '' | *[!0-9]*) die "${RUN_STATE} names no pid to stop" ;;
    esac
    kill -0 "$pid" 2>/dev/null || die "pid ${pid} is not running, but the lock is held; find it with 'ps'"

    log "stopping the rollout of $(env_value version "$RUN_STATE") started by $(env_value by "$RUN_STATE")"
    kill -TERM "$pid" 2>/dev/null || die "could not signal ${pid}"

    # Its children too, and this is the part that makes the signal land. A shell runs a trap when
    # the command it is waiting on returns, and that command is an ssh running a deploy, which can
    # be an hour on a host that reads its index. Ending the connection is what Ctrl-C does by
    # signalling the whole foreground group: deploy.sh takes the HUP it is written for and rolls the
    # server back, ssh returns, and the run's own handler puts the server in the pool.
    local child
    for child in $(ps -o pid= --ppid "$pid" 2>/dev/null); do
        kill -TERM "$child" 2>/dev/null || true
    done

    # Until the lock is free, because that is when the run's own cleanup has finished. A server it
    # had drained is put back by its handlers, not by this.
    local waited=0
    while a_run_is_in_progress; do
        sleep 1
        waited=$((waited + 1))
        if [ "$waited" -ge 120 ]; then
            die "the rollout has not stopped after ${waited}s; check 'rollout.sh status'"
        fi
    done
    log "the rollout stopped. What it left is in 'rollout.sh status'"
}

# Returns named servers to the pool, or every server that can be.
#
# What the mail after a failure asks for. It names `haproxy.sh ready <backends>/<server>`, which
# means reading the backend list out of the inventory by hand and, worse, going round
# `return_to_pool`: that is the only thing holding a server to answering both routes, and a server
# put back without it can take database traffic it cannot serve.
do_ready() {
    local wanted=("$@") inventory name host port backends server chosen=0 restored=0 refused=0

    inventory=$(read_inventory) || exit 1
    while read -r name host port backends server; do
        [ -n "$name" ] || continue

        if [ "${#wanted[@]}" -gt 0 ]; then
            case " ${wanted[*]} " in *" ${name} "*) ;; *) continue ;; esac
        fi
        chosen=$((chosen + 1))

        # Already serving traffic, so there is nothing to put back.
        case $("$HAPROXY" states "${backends}/${server}") in
            *MAINT* | *DRAIN*) ;;
            *) log "${name} is already in the pool"; continue ;;
        esac

        if return_to_pool "$name" "$host" "$port" "${backends}/${server}"; then
            restored=$((restored + 1))
        else
            refused=$((refused + 1))
        fi
    done <<<"$inventory"

    if [ "${#wanted[@]}" -gt 0 ] && [ "$chosen" -ne "${#wanted[@]}" ]; then
        die "the inventory does not name every one of: ${wanted[*]}"
    fi

    log "${restored} server(s) returned to the pool, ${refused} still out"
    [ "$refused" -eq 0 ]
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

    prepare_run_state
    note_phase "fetching the release"

    # Phase 0: the release, once, for the whole fleet. No disk check here on purpose: a full disk
    # makes curl fail before anything is touched, and a truncated download fails its checksum. The
    # server side does check, because there a failed write lands in the middle of a swap.
    fetch_release "$TMP_DIR" "$inventory"

    # Phase 1: every server checked, and the binary staged, before one is drained.
    note_phase "checking the fleet"
    preflight "$inventory"

    # Phase 2: one at a time, the next only after this one is back in the pool.
    for line in "${servers[@]}"; do
        read -r name host port backends server <<<"$line"
        note_phase "updating" "$name"
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

case $COMMAND in
    status) do_status ;;
    abort) do_abort ;;
    ready) do_ready "$@" ;;
    *) main ;;
esac

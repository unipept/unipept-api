#!/usr/bin/env bash
#
# Installs, or puts back, the API binary on this server. Run as the unipept service user.
#
# Nothing here needs root. The service user owns the binary directory and restarts its own user
# unit, so a rollout needs no sudo and no polkit rule.
#
# The only thing that installs a binary. A person on the server runs it with a version and it
# downloads its own; the rollout on the load balancer runs it with --from and a binary it has
# already delivered. Both then take the same path.
#
# A deploy that does not come back healthy rolls itself back.
#
# Flow:
#   The first argument selects what runs. The signal handlers are installed before it is read, so an
#   interrupt always clears what this run staged, and one after the swap rolls back.
#
#   deploy, rollback, stop and start change the service, and take the API lock before anything else,
#   refusing rather than waiting where something else holds it. A caller that holds it already, for a change
#   to the index this host serves, hands it down on descriptor 7 (see take_api_lock). check and
#   status change nothing and take no lock, so both answer while something else holds it.
#
#   deploy:
#     1. Parse the flags. --timeout defaults to READY_TIMEOUT in the environment file, so a host
#        that loads slowly carries its own deadline; a value that is not seconds is refused.
#     2. Run the same checks as `check`. A host that is not ready installs nothing.
#     3. Point `systemctl --user` at the user manager through XDG_RUNTIME_DIR.
#     4. Take the binary: verify the checksum of the one at --from, or download the asset for this
#        tag and variant into a temporary directory and verify that one.
#     5. Copy it to bin/unipept-api.new, run --version on the copy, keep the binary in place as
#        .previous, and rename the copy over it.
#     6. Restart the unit, then wait for /health on 127.0.0.1 and this host's PORT. The wait ends
#        early when the process is replaced, which is what a binary that exits at once does.
#     7. Healthy: clear the staged files and report the deploy. Not healthy: with --no-rollback,
#        leave it in place for the caller to decide; on a first install, report that there is
#        nothing to go back to; otherwise roll back and then fail.
#
#   rollback: keep the binary in place as .failed, put .previous back, restart, and wait for
#   /health. .previous is only removed once it serves.
#
#   stop and start: for a change that is not a new binary, such as another INDEX_LOCATION. The
#   process reads that, and the OpenSearch index of its version, only when it starts. `start` refuses
#   a service that is still running, runs the same checks as `check` and requires OpenSearch to
#   answer, then waits for /health and /health/database. Nothing is undone on a failure: the binary
#   did not change, and putting back what did is for whoever changed it.
#
#   check: collect every problem instead of stopping at the first — the commands, the user, the
#   runtime directory, the values in the environment file, the index files, memory for the variant,
#   the OpenSearch index of their version, the port redirect, free space, and with --from the
#   checksum and that the binary runs here. --index checks another directory in place of
#   INDEX_LOCATION, before anything points the service at it. Print key=value for a caller to read,
#   and exit 1 if anything is wrong.
#
#   status: print the format of what follows, the installed version, the previous one, the variant,
#   the port, whether the unit is active, the index it serves with that index's version and
#   OpenSearch index, and the path of the API lock.

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly HERE

[ -r "${HERE}/../lib.sh" ] || { echo "Error: there is no ${HERE}/../lib.sh to load." 1>&2; exit 2; }
# shellcheck source=../lib.sh
source "${HERE}/../lib.sh"
# The checks a server is held to, beside this script in a checkout and on a server alike. Missing
# only where this script was copied on its own, which install.sh never does.
[ -f "${HERE}/checks.sh" ] || die "there is no checks.sh beside ${HERE}/deploy.sh; run server/install.sh again"
# shellcheck source=checks.sh
source "${HERE}/checks.sh"

readonly SERVICE=unipept-api
readonly SERVICE_USER=unipept
readonly ROOT=/opt/unipept-api
readonly BINARY="${ROOT}/bin/unipept-api"
readonly PREVIOUS="${ROOT}/bin/unipept-api.previous"
readonly STAGED="${BINARY}.new"
readonly REJECTED="${BINARY}.failed"
readonly ENV_FILE="${ROOT}/etc/unipept-api.env"

# Seconds for /health/database to answer once /health does. The process is up by then and only asks
# OpenSearch, so this is short.
readonly DATABASE_READY_TIMEOUT=60

# Seconds for OpenSearch to answer the search `check` makes. Longer than a health probe: an index
# that was just opened can take a while to answer its first one.
readonly OPENSEARCH_TIMEOUT=30

usage() {
    cat >&2 <<'EOF'
usage:
  deploy.sh check [--from <path>] [--index <dir>]
  deploy.sh deploy [--version <tag>] [--variant <name>] [--from <path>] [--timeout <seconds>]
                   [--no-rollback]
  deploy.sh rollback [--timeout <seconds>]
  deploy.sh stop
  deploy.sh start [--timeout <seconds>]
  deploy.sh status

  --version       release tag to download, for example v2.6.0. Required without --from.
  --variant       storage backend build. Defaults to VARIANT in the environment file.
  --from          install this binary instead of downloading, or with check, validate it.
                  SHA256SUMS must sit beside it.
  --index         with check, the index directory to check in place of INDEX_LOCATION.
  --timeout       seconds to wait for /health. Defaults to READY_TIMEOUT in the environment
                  file, or 900 where that is unset.
  --no-rollback   report a failure instead of rolling back, for a caller that decides.

Run as the unipept user. Nothing here needs root.
EOF
    exit 2
}

# `systemctl --user` reaches the user manager through XDG_RUNTIME_DIR, and a non-interactive SSH
# command does not always have it set. Lingering keeps the directory there for it to find.
prepare_user_manager() {
    [ "$(id -un)" = "$SERVICE_USER" ] || die "run this as ${SERVICE_USER}, not $(id -un)"
    : "${XDG_RUNTIME_DIR:=/run/user/$(id -u)}"
    export XDG_RUNTIME_DIR
    [ -d "$XDG_RUNTIME_DIR" ] || die "no ${XDG_RUNTIME_DIR}; is lingering enabled for ${SERVICE_USER}?"
}

# Seconds to give this host to answer /health, from its own environment file.
#
# A host whose index is not resident yet needs longer than one the whole fleet can share: the
# preloaded build reads the index before it answers, and that is minutes on a spinning disk. Reading
# it here rather than taking it from the caller keeps the deadline with the host it describes, the
# way VARIANT already is.
#
# A value that is not a number falls back to the shared default instead of stopping the run. `check`
# is what reports it, so the operator hears about it there rather than from a deploy that refuses.
ready_timeout() {
    local configured
    configured=$(env_value READY_TIMEOUT "$ENV_FILE" 2>/dev/null || true)
    case ${configured:-} in
        '' | *[!0-9]*) printf '%s\n' "$DEFAULT_READY_TIMEOUT" ;;
        *) printf '%s\n' "$configured" ;;
    esac
}

# The seconds a command waits for /health: its --timeout, or this host's own. Dies on a value that is
# not a number, before anything has changed: `$((SECONDS + timeout))` on one is fatal under `set -u`.
resolve_timeout() {
    local timeout=${1:-$(ready_timeout)}

    case $timeout in
        '' | *[!0-9]*) die "--timeout takes seconds, not '${timeout}'" ;;
    esac
    printf '%s\n' "$timeout"
}

# How many times systemd has restarted the unit on its own, as a number whatever it answers.
#
# A failure to read it counts as no restarts, so an unreadable manager delays this judgement rather
# than making it wrongly: the deadline is still there to end the wait.
restart_count() {
    local count
    count=$(systemctl --user show -p NRestarts --value "$SERVICE" 2>/dev/null) || count=''
    case ${count:-} in
        '' | *[!0-9]*) printf '0\n' ;;
        *) printf '%s\n' "$count" ;;
    esac
}

# What systemd makes of the unit: active, activating, failed, inactive.
unit_state() {
    systemctl --user is-active "$SERVICE" 2>/dev/null || true
}

# Waits for the service to answer its own health route, on this host rather than through the load
# balancer.
#
# The process is watched as well as the port, because a binary that exits at once looks exactly like
# one that is slowly loading. Type=exec reports the exec rather than readiness, so systemd calls both
# of them started, and /health answers for neither. What separates them is the pid: a loading service
# keeps the one it started with, and a failing one is replaced by Restart=on-failure and gets
# another. So a pid that changed ends the wait at once instead of at the deadline, which on a host
# with a long READY_TIMEOUT is an hour of waiting for a binary that already gave up.
wait_until_healthy() {
    local timeout=$1 port began_with restarts state deadline
    port=$(env_value PORT "$ENV_FILE") || die "cannot read ${ENV_FILE}"
    [ -n "$port" ] || die "PORT is not set in ${ENV_FILE}"

    began_with=$(restart_count)
    deadline=$((SECONDS + timeout))

    log "waiting for /health on port ${port}, up to ${timeout}s"
    while [ "$SECONDS" -lt "$deadline" ]; do
        if [ "$(http_code "http://127.0.0.1:${port}/health")" = "200" ]; then
            return 0
        fi

        # Restarted since this wait began, so the process is being replaced rather than working.
        #
        # The count, rather than the pid or the unit state, because neither of those can see this.
        # Measured on systemd 255: a binary that exits at once with `Restart=on-failure` and
        # `RestartSec=5` never trips the default start limit of five starts in ten seconds, so the
        # unit reads `activating` for ever and never `failed`; and its live window is shorter than
        # any poll, so `MainPID` reads 0 at every one of them. The count is the only thing that
        # moves.
        restarts=$(restart_count)
        if [ "$restarts" -gt "$began_with" ]; then
            log "${SERVICE} restarted $((restarts - began_with)) time(s) while starting: it is failing, not loading"
            return 1
        fi

        # And the state for the case the count cannot show: a unit that did reach its start limit
        # has given up, and is failed with nothing running.
        state=$(unit_state)
        case $state in
            failed | inactive)
                log "${SERVICE} is ${state} and not running: it is failing, not loading"
                return 1
                ;;
        esac

        sleep 2
    done

    log "${SERVICE} did not answer /health within ${timeout}s"
    return 1
}

# A second name for a file, so the original stays readable.
#
# A hard link when it can: it costs nothing and cannot half-finish. fs.protected_hardlinks is on by
# default and refuses a link to a file this user does not own, so a copy is the fallback — what
# matters is that both names exist before anything is renamed, not how.
keep_copy() {
    local source=$1 target=$2
    ln -f "$source" "$target" 2>/dev/null || cp -f "$source" "$target"
}

install_binary() {
    local staged=$1 reported

    # `install` sets the mode on the copy, so the staged file needs neither the executable bit nor
    # this user as its owner — the rollout delivers it, and a person may have downloaded it.
    install -m 0755 "$staged" "$STAGED"

    # On the copy, which is the file that will run. A binary for the wrong architecture matches its
    # checksum and still cannot execute, and this is where that is caught.
    reported=$("$STAGED" --version) || die "the new binary does not run on this host"
    log "installing ${reported}"

    # A link rather than a move, so the binary is readable under its own name at every instant. The
    # pair `mv BINARY PREVIOUS; mv STAGED BINARY` has a window between the two where nothing is at
    # BINARY at all: interrupted there, the running process survives on its inode but no restart can
    # ever exec again, and only a manual move recovers it.
    #
    # The rename is within one directory and over an existing path, so it is atomic: a reader sees
    # the old file or the new one, never neither.
    [ -f "$BINARY" ] && keep_copy "$BINARY" "$PREVIOUS"

    # Before the rename, not after the function returns: a signal in between would otherwise find
    # swapped=false, take the "nothing to undo" path, and leave the new binary in place.
    swapped=true
    mv "$STAGED" "$BINARY"
}

download_asset() {
    local tag=$1 variant=$2 directory=$3 asset
    asset=$(asset_name "$tag" "$variant")

    log "downloading ${asset}"
    curl "${CURL_DOWNLOAD[@]}" -o "${directory}/${asset}" "$(release_url "$tag" "$asset")"
    curl "${CURL_DOWNLOAD[@]}" -o "${directory}/SHA256SUMS" "$(release_url "$tag" SHA256SUMS)"

    printf '%s\n' "${directory}/${asset}"
}

# Whether the binary in place is the new one, which decides what an interrupt has to undo.
swapped=false

# Whether this run holds the API lock, and so may clear the staged file. Every deploy.sh on this host
# stages at the same path, and one refused the lock, or a check or a status run meanwhile, must not
# clear the file of the deploy that holds it.
locked=false

# Runs on every exit, including a signal.
#
# HUP is in the list because the rollout invokes this over ssh: interrupting the rollout closes the
# connection and sshd sends HUP to this process group. An interrupt after the swap has to roll back —
# no coordinator is left alive to decide — and before it, only the staged file needs clearing.
on_signal() {
    local signal=$1

    log "caught ${signal}"
    if [ "$swapped" = true ]; then
        if [ -f "$PREVIOUS" ]; then
            log "the binary was already swapped, rolling back"
            rollback_to_previous "$(ready_timeout)" ||
                log "could not roll back; ${PREVIOUS} is intact and the service needs attention"
        else
            log "the binary was already swapped and there is nothing to go back to; the service needs attention"
        fi
    fi
    clean_staging
    exit 130
}

clean_staging() {
    [ "$locked" = false ] || rm -f "$STAGED"
    [ -n "${DOWNLOAD_DIR:-}" ] && rm -rf "$DOWNLOAD_DIR"
    return 0
}

restart_service() {
    log "restarting ${SERVICE}"
    clear_start_limit
    systemctl --user restart "$SERVICE"
}

# A binary that exits at once is restarted every RestartSec, and once systemd's start limit trips it
# refuses every start with "Start request repeated too quickly", a rollback's included. The limit
# stays for the restarts systemd makes on its own; a deliberate start clears it first.
clear_start_limit() {
    systemctl --user reset-failed "$SERVICE" 2>/dev/null || true
}

# Everything that has to be true before this host is asked to install anything: the checks in
# checks.sh, each in turn, counting the ones that found a problem and the ones that warned.
# problems= is that count of checks, so several missing files are one problem; each is printed.
#
# Run by the rollout on every server before it drains the first one, by `deploy` on itself, and by an
# operator who wants to know. Runs every check rather than stopping at the first, because the point
# is to learn the whole story in one pass.
#
# Prints key=value for a caller to read, and returns 1 if any check found a problem.
do_check() {
    local from='' index_override='' problems=0 warnings=0

    while [ $# -gt 0 ]; do
        case $1 in
            --from) [ $# -ge 2 ] || usage; from=$2; shift 2 ;;
            --index) [ $# -ge 2 ] || usage; index_override=$2; shift 2 ;;
            *) usage ;;
        esac
    done

    check_commands || problems=$((problems + 1))
    check_user || problems=$((problems + 1))
    check_lingering || problems=$((problems + 1))
    check_env_file "$ENV_FILE" "$index_override" || problems=$((problems + 1))

    local index='' variant='' port='' database='' index_version='-' opensearch_index='-'
    if [ -r "$ENV_FILE" ]; then
        index=$(env_value INDEX_LOCATION "$ENV_FILE")
        variant=$(env_value VARIANT "$ENV_FILE")
        port=$(env_value PORT "$ENV_FILE")
        database=$(env_value DATABASE_ADDRESS "$ENV_FILE")
    fi
    # A directory about to be served is checked by the same rules before anything points at it.
    [ -z "$index_override" ] || index=$index_override
    valid_variant "$variant" || variant=''


    if [ -n "$index" ]; then
        if check_index_dir "$index"; then
            check_index_not_home "$index" || problems=$((problems + 1))
            if check_index_files "$index"; then
                index_version=$(version_in "${index}/.version")
                if check_index_version "$index" "$index_version"; then
                    opensearch_index=$(index_name "$index_version")
                else
                    # Not printed below: it may hold anything, a line of its own included.
                    index_version='-'
                    problems=$((problems + 1))
                fi
            else
                problems=$((problems + 1))
            fi
            check_index_optional_files "$index" || warnings=$((warnings + 1))
        else
            problems=$((problems + 1))
        fi
        # A directory its files can be measured in, even one that cannot be listed.
        local resident=0
        [ -z "$variant" ] || [ ! -d "$index" ] || resident=$(resident_bytes "$variant" "$index")
        if [ "$resident" -gt 0 ]; then
            if check_memory_fits "$variant" "$resident"; then
                check_memory_free "$variant" "$resident" || warnings=$((warnings + 1))
            else
                problems=$((problems + 1))
            fi
        fi
    fi

    # One search, whose answer both checks judge: nothing answering is a warning, an answer other
    # than 200 a problem. Whether it answered is kept for `start`, which will not start without it.
    OPENSEARCH_ANSWERED=false
    if [ "$opensearch_index" != '-' ] && [ -n "$database" ]; then
        local status
        status=$(search_status "$database" "$opensearch_index")
        if check_opensearch_answers "$database" "$opensearch_index" "$status"; then
            OPENSEARCH_ANSWERED=true
            check_opensearch_index "$opensearch_index" "$index_version" "$status" || problems=$((problems + 1))
        else
            warnings=$((warnings + 1))
        fi
    fi

    check_ports_redirect "$port" || problems=$((problems + 1))
    check_bin_writable || problems=$((problems + 1))
    check_bin_room "$from" || problems=$((problems + 1))
    [ -z "$from" ] || check_binary "$from" || problems=$((problems + 1))

    printf 'variant=%s\n' "${variant:-unknown}"
    printf 'port=%s\n' "${port:-unknown}"
    printf 'index_version=%s\n' "$index_version"
    printf 'opensearch_index=%s\n' "$opensearch_index"
    printf 'ready_timeout=%s\n' "$(ready_timeout)"
    printf 'problems=%s\n' "$problems"
    printf 'warnings=%s\n' "$warnings"

    [ "$problems" -eq 0 ] || return 1
}

do_deploy() {
    local tag='' variant='' from='' timeout='' no_rollback=false

    while [ $# -gt 0 ]; do
        # Two arguments or usage: `shift 2` with one left would fail, and under `set -e` that exits
        # before the check below, so a mistyped flag would say nothing at all.
        case $1 in
            --version) [ $# -ge 2 ] || usage; tag=$2; shift 2 ;;
            --variant) [ $# -ge 2 ] || usage; variant=$2; shift 2 ;;
            --from) [ $# -ge 2 ] || usage; from=$2; shift 2 ;;
            --timeout) [ $# -ge 2 ] || usage; timeout=$2; shift 2 ;;
            --no-rollback) no_rollback=true; shift ;;
            *) usage ;;
        esac
    done

    # Before anything else: a bad value is only used after the binary has been swapped, where
    # nothing would roll it back.
    timeout=$(resolve_timeout "$timeout")

    do_check ${from:+--from "$from"} >/dev/null || die "this host is not ready; run 'deploy.sh check' to see why"
    prepare_user_manager

    local staged directory

    if [ -n "$from" ]; then
        [ -f "$from" ] || die "no binary at $from"
        verify_sha256 "$from" "$(dirname "$from")/SHA256SUMS"
        staged=$from
    else
        [ -n "$tag" ] || usage
        [ -n "$variant" ] || variant=$(env_value VARIANT "$ENV_FILE")
        [ -n "$variant" ] || die "no variant given and no VARIANT in $ENV_FILE"

        # Only the download needs somewhere to put a file; --from installs one already delivered.
        DOWNLOAD_DIR=$(mktemp -d)
        directory=$DOWNLOAD_DIR

        staged=$(download_asset "$tag" "$variant" "$directory")
        verify_sha256 "$staged" "${directory}/SHA256SUMS"
    fi

    # A first deploy has no previous binary, so an unhealthy one cannot be undone. Worth saying
    # before the restart rather than reporting it afterwards as a rollback that failed.
    local first_install=false
    [ -f "$BINARY" ] || first_install=true

    install_binary "$staged"

    # From here the new binary is in place, so every failure has to be answered. Without this, `set
    # -e` would abort on a failed restart and leave the service down on a binary nobody chose, with
    # the one that worked sitting in .previous.
    if restart_service && wait_until_healthy "$timeout"; then
        clean_staging
        log "deployed"
        return 0
    fi

    # The rollout passes --no-rollback: it asks this host what actually happened and then decides,
    # so that a deploy which succeeded while the connection died is not undone. Run by hand, or
    # interrupted, there is nobody to ask and rolling back here is the only safe answer.
    if [ "$no_rollback" = true ]; then
        clean_staging
        die "the service is not serving the new binary; left in place for the caller to decide"
    fi

    if [ "$first_install" = true ]; then
        clean_staging
        die "the first binary on this host does not serve, and there is nothing to roll back to"
    fi

    log "the service is not serving the new binary, rolling back"
    do_rollback --timeout "$timeout"
    clean_staging
    die "deploy failed and was rolled back"
}

do_rollback() {
    local timeout=''

    while [ $# -gt 0 ]; do
        case $1 in
            --timeout) [ $# -ge 2 ] || usage; timeout=$2; shift 2 ;;
            *) usage ;;
        esac
    done

    timeout=$(resolve_timeout "$timeout")

    prepare_user_manager
    [ -f "$PREVIOUS" ] || die "no previous binary at $PREVIOUS"

    rollback_to_previous "$timeout" ||
        die "the previous binary is not healthy either; ${PREVIOUS} is intact and ${REJECTED} is what failed"

    log "rolled back"
}

# Puts the previous binary back and returns whether it serves, for a caller that has something else
# to do about it. `do_rollback` is the same thing for a caller that has not.
rollback_to_previous() {
    local timeout=$1

    log "putting back $("$PREVIOUS" --version)"

    # The binary being replaced is kept for diagnosis rather than overwritten: it is the one that
    # just failed, and it is the only copy on the host.
    [ -f "$BINARY" ] && keep_copy "$BINARY" "$REJECTED"

    # A second name again, so .previous survives a rollback that itself fails. With a move, a
    # rollback whose health check then failed left neither a working binary nor anything to retry.
    keep_copy "$PREVIOUS" "${BINARY}.rollback"
    mv "${BINARY}.rollback" "$BINARY"

    if ! restart_service || ! wait_until_healthy "$timeout"; then
        return 1
    fi

    # Only now, with the previous binary proven to serve, is the spare copy redundant.
    rm -f "$PREVIOUS"
}

do_stop() {
    prepare_user_manager
    log "stopping ${SERVICE}"
    systemctl --user stop "$SERVICE"
    log "stopped"
}

# Starts the stopped service on what its environment file names, and waits until it serves.
do_start() {
    local timeout='' port

    while [ $# -gt 0 ]; do
        case $1 in
            --timeout) [ $# -ge 2 ] || usage; timeout=$2; shift 2 ;;
            *) usage ;;
        esac
    done

    timeout=$(resolve_timeout "$timeout")
    prepare_user_manager

    # A running process would keep what it read when it started: `start` would change nothing.
    case $(unit_state) in
        active | activating | reloading) die "${SERVICE} is running; stop it first" ;;
    esac

    # Before the start, so a host whose index is missing or does not fit says so without a start
    # that fails. The proteins have to answer: a start without them serves /health and nothing else.
    do_check >/dev/null || die "this host is not ready; run 'deploy.sh check' to see why"
    [ "$OPENSEARCH_ANSWERED" = true ] || die "OpenSearch does not answer; start it first"

    log "starting ${SERVICE}"
    clear_start_limit
    systemctl --user start "$SERVICE" || die "systemd did not start ${SERVICE}"
    wait_until_healthy "$timeout" || die "${SERVICE} does not answer /health"

    # /health answers without OpenSearch, so the proteins are checked on their own.
    port=$(env_value PORT "$ENV_FILE")
    wait_for_http "http://127.0.0.1:${port}/health/database" "$DATABASE_READY_TIMEOUT" ||
        die "${SERVICE} answers /health, but not /health/database: the OpenSearch index of its version does not answer"

    log "started"
}

# What a binary calls itself, or `unknown` for one too old to answer --version, which is every
# release before #257. `|| true` throughout: `status` has to describe a broken host, not join it.
reported_version() {
    local binary=$1 reported=''

    [ -x "$binary" ] || { printf -- '-\n'; return 0; }
    reported=$("$binary" --version 2>/dev/null | awk '{ print $NF }') || true
    printf '%s\n' "${reported:-unknown}"
}

# key=value lines, for the rollout and for whatever changes the index this host serves, so neither
# reads this host's files itself.
#
# status_format comes first and says how to read the rest: it is raised only where a line changes
# meaning or goes, and a caller refuses a format it does not know. A line added keeps it.
#
# The index's version and its OpenSearch index are `-` where its .version cannot be read or names no
# index, so a .version of several lines never adds one here; `check` says why.
do_status() {
    local index version named index_version='-' opensearch_index='-'

    prepare_user_manager

    index=$(env_value INDEX_LOCATION "$ENV_FILE" || true)
    if [ -n "$index" ] && [ -f "${index}/.version" ] && [ -r "${index}/.version" ] &&
        version=$(version_in "${index}/.version") && named=$(index_name "$version"); then
        index_version=$version
        opensearch_index=$named
    fi

    printf 'status_format=1\n'
    printf 'version=%s\n' "$(reported_version "$BINARY")"
    printf 'previous=%s\n' "$(reported_version "$PREVIOUS")"
    printf 'variant=%s\n' "$(env_value VARIANT "$ENV_FILE" || true)"
    printf 'port=%s\n' "$(env_value PORT "$ENV_FILE" || true)"
    printf 'active=%s\n' "$(systemctl --user is-active "$SERVICE" || true)"
    printf 'index_location=%s\n' "$index"
    printf 'index_version=%s\n' "$index_version"
    printf 'opensearch_index=%s\n' "$opensearch_index"
    printf 'api_lock=%s\n' "$API_LOCK"
}

[ $# -gt 0 ] || usage
command=$1
shift

# Before anything is parsed, so even an early failure clears up after itself.
trap 'on_signal INT' INT
trap 'on_signal TERM' TERM
trap 'on_signal HUP' HUP
trap clean_staging EXIT

# The commands that change the service take the API lock before they read their flags, so one that is
# refused has done nothing.
case $command in
    deploy | rollback | stop | start)
        require flock
        take_api_lock || die "$(api_lock_refused $?)"
        locked=true
        ;;
esac

case $command in
    # A host with problems is a check's "no", exit 1. Said here, since do_check also answers the
    # callers that test it, and a function that returns 1 at the top level would trip the error trap.
    check) do_check "$@" || exit 1 ;;
    deploy) do_deploy "$@" ;;
    rollback) do_rollback "$@" ;;
    stop) do_stop "$@" ;;
    start) do_start "$@" ;;
    status) do_status "$@" ;;
    *) usage ;;
esac

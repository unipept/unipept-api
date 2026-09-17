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
#   interrupt always clears the staged file, and one after the swap rolls back.
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
#   check: collect every problem instead of stopping at the first — the commands, the user, the
#   runtime directory, the values in the environment file, the index files, memory for the variant,
#   the port redirect, free space, and with --from the checksum and that the binary runs here. Print
#   key=value for a caller to read, and exit non-zero if anything is wrong.
#
#   status: print the installed version, the previous one, the variant, the port, and whether the
#   unit is active.

set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly HERE

# Beside this script once install.sh has placed both in /opt/unipept-api/lib, one level up in a
# repository checkout.
# shellcheck source-path=SCRIPTDIR source=../lib.sh
if [ -f "${HERE}/lib.sh" ]; then
    source "${HERE}/lib.sh"
else
    source "${HERE}/../lib.sh"
fi

# What `die` raises when it is called from inside a subshell.
trap 'exit 1' USR1

readonly SERVICE=unipept-api
readonly SERVICE_USER=unipept
readonly ROOT=/opt/unipept-api
readonly BINARY="${ROOT}/bin/unipept-api"
readonly PREVIOUS="${ROOT}/bin/unipept-api.previous"
readonly STAGED="${BINARY}.new"
readonly REJECTED="${BINARY}.failed"
readonly ENV_FILE="${ROOT}/etc/unipept-api.env"

usage() {
    cat >&2 <<'EOF'
usage:
  deploy.sh check [--from <path>]
  deploy.sh deploy [--version <tag>] [--variant <name>] [--from <path>] [--timeout <seconds>]
                   [--no-rollback]
  deploy.sh rollback [--timeout <seconds>]
  deploy.sh status

  --version       release tag to download, for example v2.6.0. Required without --from.
  --variant       storage backend build. Defaults to VARIANT in the environment file.
  --from          install this binary instead of downloading, or with check, validate it.
                  SHA256SUMS must sit beside it.
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
    rm -f "$STAGED"
    [ -n "${DOWNLOAD_DIR:-}" ] && rm -rf "$DOWNLOAD_DIR"
    return 0
}

restart_service() {
    log "restarting ${SERVICE}"
    systemctl --user restart "$SERVICE"
}

# Everything that has to be true before this host is asked to install anything.
#
# Run by the rollout on every server before it drains the first one, by `deploy` on itself, and by an
# operator who wants to know. Collects every problem rather than stopping at the first, because the
# point is to learn the whole story in one pass.
#
# Prints key=value for a caller to read, and exits non-zero if anything is wrong.
do_check() {
    local from='' problems=0 warnings=0

    while [ $# -gt 0 ]; do
        case $1 in
            --from) [ $# -ge 2 ] || usage; from=$2; shift 2 ;;
            *) usage ;;
        esac
    done

    fail() { log "check: $*"; problems=$((problems + 1)); }
    warn() { log "check: $*"; warnings=$((warnings + 1)); }

    # Every command the update reaches for, not only the three the deploy used to name. A missing
    # `install` or `ln` would otherwise surface with the binary half replaced.
    local cmd
    for cmd in curl sha256sum install mktemp systemctl awk sed ln mv df; do
        command -v "$cmd" >/dev/null || fail "${cmd} is not installed"
    done

    [ "$(id -un)" = "$SERVICE_USER" ] || fail "running as $(id -un), not ${SERVICE_USER}"

    # systemctl --user talks to the user manager through this, and a non-interactive ssh command does
    # not always have it set. Lingering is what keeps it in place.
    local runtime="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}"
    [ -d "$runtime" ] || fail "no ${runtime}; enable lingering for ${SERVICE_USER}"

    local index='' variant='' port=''
    if [ -r "$ENV_FILE" ]; then
        index=$(env_value INDEX_LOCATION "$ENV_FILE")
        variant=$(env_value VARIANT "$ENV_FILE")
        port=$(env_value PORT "$ENV_FILE")

        [ -n "$index" ] || fail "INDEX_LOCATION is not set in ${ENV_FILE}"
        [ -n "$(env_value DATABASE_ADDRESS "$ENV_FILE")" ] || fail "DATABASE_ADDRESS is not set"
        case $port in
            '') fail "PORT is not set" ;;
            *[!0-9]*) fail "PORT is '${port}', which is not a number" ;;
        esac
        case $variant in
            mmap | preloaded | hybrid) ;;
            '') fail "VARIANT is not set; expected mmap, preloaded or hybrid" ;;
            *) fail "VARIANT is '${variant}'; expected mmap, preloaded or hybrid" ;;
        esac

        # Optional, so only a value that is set and wrong is a problem. Reported here because
        # `ready_timeout` falls back rather than refusing, and a host that silently kept the shared
        # 900 is the failure this whole setting exists to avoid.
        local configured_timeout
        configured_timeout=$(env_value READY_TIMEOUT "$ENV_FILE")
        case $configured_timeout in
            '') ;;
            *[!0-9]*) fail "READY_TIMEOUT is '${configured_timeout}', which is not a number of seconds" ;;
        esac
    else
        fail "cannot read ${ENV_FILE}"
    fi

    # The index, which is the setting most often wrong and the slowest to find out about: without
    # this the service simply never answers and the deploy waits out its whole timeout.
    local index_version='-'
    if [ -n "$index" ]; then
        if [ -d "$index" ] && [ -r "$index" ]; then
            # ProtectHome=yes hides /home from the unit, so an index there is readable now and gone
            # the moment systemd starts the service.
            case $index in
                /home/*) fail "INDEX_LOCATION is under /home, which ProtectHome=yes hides from the service" ;;
            esac

            local relative missing=0
            for relative in $INDEX_FILES; do
                [ -r "${index}/${relative}" ] || { fail "${index}/${relative} is missing or unreadable"; missing=1; }
            done
            for relative in $OPTIONAL_INDEX_FILES; do
                [ -r "${index}/${relative}" ] || warn "${index}/${relative} is missing or unreadable; the service runs without it, but searches are slower"
            done
            [ "$missing" -eq 0 ] && index_version=$(tr -d '[:space:]' < "${index}/.version")
        else
            fail "${index} is not a readable directory"
        fi
    fi

    # The backend is compiled in, so a host that cannot hold its variant cannot be corrected by a
    # restart — only by deploying a different build.
    case $variant in mmap | preloaded | hybrid) ;; *) variant='' ;; esac
    if [ -n "$variant" ] && [ -n "$index" ] && [ -d "$index" ]; then
        local needed=0 relative size
        for relative in $(files_resident_for "$variant"); do
            size=$(stat -c %s "${index}/${relative}" 2>/dev/null || echo 0)
            needed=$((needed + size))
        done

        if [ "$needed" -gt 0 ]; then
            local total available
            total=$(meminfo MemTotal)
            available=$(meminfo MemAvailable)

            # Above MemTotal is arithmetic: the variant cannot fit, ever. Above MemAvailable is a
            # guess, because page cache is reclaimable, so it must not block a legitimate deploy.
            if [ "$needed" -gt "$total" ]; then
                fail "${variant} needs $((needed / 1024 / 1024)) MiB resident, and this host has $((total / 1024 / 1024)) MiB in total"
            elif [ "$needed" -gt "$available" ]; then
                warn "${variant} needs $((needed / 1024 / 1024)) MiB resident and $((available / 1024 / 1024)) MiB is available; the kernel has to reclaim first"
            fi
        fi
    fi

    # The service listens above 1024 and the load balancer reaches it on 80, so the redirect is as
    # necessary as the binary. A host that lost it serves perfectly and is unreachable, which is
    # exactly the failure this whole check exists to find before a server is drained.
    case $(systemctl is-enabled unipept-api-ports 2>/dev/null) in
        enabled) ;;
        *) fail "unipept-api-ports is not enabled, so port 80 will not reach this service after a reboot" ;;
    esac
    case $(systemctl is-active unipept-api-ports 2>/dev/null) in
        active) ;;
        *) fail "unipept-api-ports is not active; run it as root to restore the port 80 redirect" ;;
    esac

    # And that it works, rather than only that systemd thinks it ran.
    #
    # Only meaningful while the service is answering on its own port: if it is not, nothing can be
    # concluded about the redirect, and a deploy is exactly what someone runs to fix a service that
    # is down. Refusing here would block the recovery.
    if [ -n "$port" ] && [ "$(http_code "http://127.0.0.1:${port}/health")" = "200" ]; then
        if [ "$(http_code "http://127.0.0.1:80/health")" != "200" ]; then
            fail "the service answers on ${port} but port 80 does not reach it; check unipept-api-ports"
        fi
    fi

    [ -w "${ROOT}/bin" ] || fail "${ROOT}/bin is not writable"

    # Room for a second copy beside the one running, since both exist during a swap. Measured from
    # the binary at hand rather than a guessed constant, with a little room to spare.
    local free_kb binary_kb=0
    free_kb=$(df -Pk "${ROOT}/bin" | awk 'NR == 2 { print $4 }')
    if [ -n "$from" ] && [ -f "$from" ]; then
        binary_kb=$(( $(stat -c %s "$from") / 1024 ))
    elif [ -f "$BINARY" ]; then
        binary_kb=$(( $(stat -c %s "$BINARY") / 1024 ))
    fi
    if [ "${free_kb:-0}" -lt $((binary_kb + 20480)) ]; then
        fail "${ROOT}/bin has $(( ${free_kb:-0} / 1024 )) MiB free, and a swap needs about $(( (binary_kb + 20480) / 1024 )) MiB"
    fi

    # With --from, the delivered binary itself: the checksum, and that it runs here. A build for the
    # wrong architecture matches its checksum and still cannot execute, and finding that out during
    # the rollout means a server already drained.
    if [ -n "$from" ]; then
        if [ ! -f "$from" ]; then
            fail "no binary at ${from}"
        elif ! sha256_matches "$from" "$(dirname "$from")/SHA256SUMS"; then
            fail "${from} does not match its checksum"
        else
            # Beside the binary rather than in /tmp: /tmp is mounted noexec on a hardened host, and
            # the probe would then fail for every architecture, reporting a good build as unrunnable.
            local probe="${ROOT}/bin/.probe.$$"
            install -m 0755 "$from" "$probe"
            if ! "$probe" --version >/dev/null 2>&1; then
                fail "${from} does not run on this host; wrong architecture or a missing library"
            fi
            rm -f "$probe"
        fi
    fi

    printf 'variant=%s\n' "${variant:-unknown}"
    printf 'port=%s\n' "${port:-unknown}"
    printf 'index_version=%s\n' "$index_version"
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

    [ -n "$timeout" ] || timeout=$(ready_timeout)

    # Before anything else: `$((SECONDS + timeout))` on a non-numeric value is fatal under `set -u`,
    # and it is only reached after the binary has been swapped, where nothing would roll it back.
    case $timeout in
        '' | *[!0-9]*) die "--timeout takes seconds, not '${timeout}'" ;;
    esac

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

    [ -n "$timeout" ] || timeout=$(ready_timeout)

    case $timeout in
        '' | *[!0-9]*) die "--timeout takes seconds, not '${timeout}'" ;;
    esac

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

# What a binary calls itself, or `unknown` for one too old to answer --version, which is every
# release before #257. `|| true` throughout: `status` has to describe a broken host, not join it.
reported_version() {
    local binary=$1 reported=''

    [ -x "$binary" ] || { printf -- '-\n'; return 0; }
    reported=$("$binary" --version 2>/dev/null | awk '{ print $NF }') || true
    printf '%s\n' "${reported:-unknown}"
}

# key=value lines, so the rollout can read them.
do_status() {
    prepare_user_manager

    printf 'version=%s\n' "$(reported_version "$BINARY")"
    printf 'previous=%s\n' "$(reported_version "$PREVIOUS")"
    printf 'variant=%s\n' "$(env_value VARIANT "$ENV_FILE" || true)"
    printf 'port=%s\n' "$(env_value PORT "$ENV_FILE" || true)"

    printf 'active=%s\n' "$(systemctl --user is-active "$SERVICE" || true)"
}

[ $# -gt 0 ] || usage
command=$1
shift

# Before anything is parsed, so even an early failure clears up after itself.
trap 'on_signal INT' INT
trap 'on_signal TERM' TERM
trap 'on_signal HUP' HUP
trap clean_staging EXIT

case $command in
    check) do_check "$@" ;;
    deploy) do_deploy "$@" ;;
    rollback) do_rollback "$@" ;;
    status) do_status "$@" ;;
    *) usage ;;
esac

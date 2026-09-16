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
  --timeout       seconds to wait for /health. Defaults to 900.
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

# Waits for the service to answer its own health route, on this host rather than through the load
# balancer.
wait_until_healthy() {
    local timeout=$1 port
    port=$(env_value PORT "$ENV_FILE") || die "cannot read ${ENV_FILE}"
    [ -n "$port" ] || die "PORT is not set in ${ENV_FILE}"

    log "waiting for /health on port ${port}, up to ${timeout}s"
    wait_for_http "http://127.0.0.1:${port}/health" "$timeout"
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
    mv "$STAGED" "$BINARY"
}

download_asset() {
    local tag=$1 variant=$2 directory=$3 asset
    asset=$(asset_name "$tag" "$variant")

    log "downloading ${asset}"
    curl -fsSL --retry 3 -o "${directory}/${asset}" "$(release_url "$tag" "$asset")"
    curl -fsSL --retry 3 -o "${directory}/SHA256SUMS" "$(release_url "$tag" SHA256SUMS)"

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
        log "the binary was already swapped, rolling back"
        do_rollback || log "could not roll back; ${PREVIOUS} is intact"
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
            local probe
            probe=$(mktemp)
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
    printf 'problems=%s\n' "$problems"
    printf 'warnings=%s\n' "$warnings"

    [ "$problems" -eq 0 ] || return 1
}

do_deploy() {
    local tag='' variant='' from='' timeout=$DEFAULT_READY_TIMEOUT no_rollback=false

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

    install_binary "$staged"
    swapped=true

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

    log "the service is not serving the new binary, rolling back"
    do_rollback --timeout "$timeout"
    clean_staging
    die "deploy failed and was rolled back"
}

do_rollback() {
    local timeout=$DEFAULT_READY_TIMEOUT

    while [ $# -gt 0 ]; do
        case $1 in
            --timeout) [ $# -ge 2 ] || usage; timeout=$2; shift 2 ;;
            *) usage ;;
        esac
    done

    prepare_user_manager
    [ -f "$PREVIOUS" ] || die "no previous binary at $PREVIOUS"

    log "putting back $("$PREVIOUS" --version)"

    # The binary being replaced is kept for diagnosis rather than overwritten: it is the one that
    # just failed, and it is the only copy on the host.
    [ -f "$BINARY" ] && keep_copy "$BINARY" "$REJECTED"

    # A second name again, so .previous survives a rollback that itself fails. With a move, a
    # rollback whose health check then failed left neither a working binary nor anything to retry.
    keep_copy "$PREVIOUS" "${BINARY}.rollback"
    mv "${BINARY}.rollback" "$BINARY"

    if ! restart_service || ! wait_until_healthy "$timeout"; then
        die "the previous binary is not healthy either; ${PREVIOUS} is intact and ${REJECTED} is what failed"
    fi

    # Only now, with the previous binary proven to serve, is the spare copy redundant.
    rm -f "$PREVIOUS"

    log "rolled back"
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

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

readonly SERVICE=unipept-api
readonly SERVICE_USER=unipept
readonly ROOT=/opt/unipept-api
readonly BINARY="${ROOT}/bin/unipept-api"
readonly PREVIOUS="${ROOT}/bin/unipept-api.previous"
readonly ENV_FILE="${ROOT}/etc/unipept-api.env"

# The preloaded and hybrid builds read the index into memory before they answer.
readonly DEFAULT_READY_TIMEOUT=900

usage() {
    cat >&2 <<'EOF'
usage:
  deploy.sh deploy [--version <tag>] [--variant <name>] [--from <path>] [--timeout <seconds>]
  deploy.sh rollback [--timeout <seconds>]
  deploy.sh status

  --version   release tag to download, for example v2.6.0. Required without --from.
  --variant   storage backend build. Defaults to VARIANT in the environment file.
  --from      install this binary instead of downloading. SHA256SUMS must sit beside it.
  --timeout   seconds to wait for /health. Defaults to 900.

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
    port=$(env_value PORT "$ENV_FILE")

    log "waiting for /health on port ${port}, up to ${timeout}s"
    wait_for_http "http://127.0.0.1:${port}/health" "$timeout"
}

install_binary() {
    local staged=$1 reported

    # `install` sets the mode on the copy, so the staged file needs neither the executable bit nor
    # this user as its owner — the rollout delivers it, and a person may have downloaded it.
    install -m 0755 "$staged" "${BINARY}.new"

    # On the copy, which is the file that will run. A binary for the wrong architecture matches its
    # checksum and still cannot execute, and this is where that is caught.
    reported=$("${BINARY}.new" --version) || die "the new binary does not run on this host"
    log "installing ${reported}"

    # Moving the running binary keeps it on its inode, so the running process is unharmed. The
    # rename into place is within one directory, so it is atomic.
    [ -f "$BINARY" ] && mv "$BINARY" "$PREVIOUS"
    mv "${BINARY}.new" "$BINARY"
}

download_asset() {
    local tag=$1 variant=$2 directory=$3 asset
    asset=$(asset_name "$tag" "$variant")

    log "downloading ${asset}"
    curl -fsSL --retry 3 -o "${directory}/${asset}" "$(release_url "$tag" "$asset")"
    curl -fsSL --retry 3 -o "${directory}/SHA256SUMS" "$(release_url "$tag" SHA256SUMS)"

    printf '%s\n' "${directory}/${asset}"
}

restart_service() {
    log "restarting ${SERVICE}"
    systemctl --user restart "$SERVICE"
}

do_deploy() {
    local tag='' variant='' from='' timeout=$DEFAULT_READY_TIMEOUT

    while [ $# -gt 0 ]; do
        # Two arguments or usage: `shift 2` with one left would fail, and under `set -e` that exits
        # before the check below, so a mistyped flag would say nothing at all.
        case $1 in
            --version) [ $# -ge 2 ] || usage; tag=$2; shift 2 ;;
            --variant) [ $# -ge 2 ] || usage; variant=$2; shift 2 ;;
            --from) [ $# -ge 2 ] || usage; from=$2; shift 2 ;;
            --timeout) [ $# -ge 2 ] || usage; timeout=$2; shift 2 ;;
            *) usage ;;
        esac
    done

    require_cmd curl sha256sum systemctl
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
        directory=$(mktemp -d)
        # shellcheck disable=SC2064  # $directory is wanted now, not at trap time.
        trap "rm -rf '$directory'" EXIT

        staged=$(download_asset "$tag" "$variant" "$directory")
        verify_sha256 "$staged" "${directory}/SHA256SUMS"
    fi

    install_binary "$staged"

    # From here the new binary is in place, so every failure has to put the old one back. Without
    # this, `set -e` would abort on a failed restart and leave the service down on a binary nobody
    # chose, with the one that worked sitting in .previous.
    if restart_service && wait_until_healthy "$timeout"; then
        log "deployed"
        return 0
    fi

    log "the service is not serving the new binary, rolling back"
    do_rollback --timeout "$timeout"
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
    mv "$PREVIOUS" "$BINARY"

    restart_service
    wait_until_healthy "$timeout" || die "the previous binary is not healthy either"

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

case $command in
    deploy) do_deploy "$@" ;;
    rollback) do_rollback "$@" ;;
    status) do_status "$@" ;;
    *) usage ;;
esac

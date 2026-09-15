#!/usr/bin/env bash
#
# The HAProxy runtime API, as the rollout needs it. Run on the load balancer.
#
# Every HAProxy detail lives here, so rollout.sh holds the sequence and nothing else. Fields are
# located by name in the `show stat` header rather than by column number, because that layout is
# not a stable interface.
#
# The socket is srw------- root:haproxy. Either run as root, or give the socket `mode 660` in
# haproxy.cfg and put the operator in the haproxy group.

set -euo pipefail

# shellcheck source-path=SCRIPTDIR source=../lib.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/lib.sh"

: "${HAPROXY_SOCKET:=/run/haproxy/haproxy.sock}"

usage() {
    cat >&2 <<'EOF'
usage: haproxy.sh <command> <backend>/<server> [arguments]

  drain <b>/<s>               stop sending new connections, let the open ones finish
  maint <b>/<s>               take out of rotation and stop health checking it
  ready <b>/<s>               put back in rotation
  state <b>/<s>               print the status field, for example UP, UP 1/100 or MAINT
  sessions <b>/<s>            print the current session count
  wait-empty <b>/<s> <secs>   wait until the session count reaches 0
  wait-up <b>/<s> <secs>      wait until the status field reads UP
  up-count <b>                print how many servers in the backend are UP

The socket path comes from HAPROXY_SOCKET, default /run/haproxy/haproxy.sock.
EOF
    exit 2
}

# Sends one runtime command and returns what HAProxy answered.
#
# socat rather than nc: talking to a unix socket is what it is for, and it is what the load
# balancer has.
runtime() {
    require_cmd socat
    [ -S "$HAPROXY_SOCKET" ] || die "no HAProxy socket at $HAPROXY_SOCKET"

    printf '%s\n' "$1" | socat "$HAPROXY_SOCKET" stdio 2>/dev/null ||
        die "cannot talk to $HAPROXY_SOCKET. Run as root, or join the haproxy group."
}

# Splits backend/server, refusing anything else: a typo here would address a server that does
# not exist, and `set server` on an unknown name is not an error HAProxy reports usefully.
split_target() {
    case ${1:-} in
        */*) backend=${1%%/*}; server=${1##*/} ;;
        *) die "expected <backend>/<server>, got '${1:-}'" ;;
    esac
    if [ -z "$backend" ] || [ -z "$server" ]; then
        die "expected <backend>/<server>, got '$1'"
    fi
}

# Reads one named field for one server out of `show stat`.
#
# `status` is not always one word: HAProxy appends the check counter while a server is in
# transition, so a healthy server reads `UP` or `UP 1/100`. Callers compare the first word.
stat_field() {
    local backend=$1 server=$2 field=$3 value

    value=$(runtime "show stat" | awk -F, -v px="$backend" -v sv="$server" -v want="$field" '
        # The header names every column, and the first is written "# pxname".
        /^#/ {
            for (i = 1; i <= NF; i++) {
                name = $i
                sub(/^# */, "", name)
                if (name == want) column = i
            }
            if (!column) exit 2
            next
        }
        $1 == px && $2 == sv { print $column; found = 1; exit }
        END { if (!found) exit 1 }
    ') || die "$backend/$server is not in the HAProxy configuration"

    printf '%s\n' "$value"
}

set_state() {
    local target=$1 state=$2
    split_target "$target"

    local answer
    answer=$(runtime "set server ${backend}/${server} state ${state}")

    # A refused command answers with a message; an accepted one answers with nothing.
    [ -z "${answer//[[:space:]]/}" ] || die "HAProxy refused the change: ${answer}"

    log "${backend}/${server} set to ${state}"
}

# Waits for the open sessions to finish, so a restart does not cut them off.
#
# The API's own request timeout is 150 seconds, so a server that is draining answers or gives up
# within that. `timeout server` on this load balancer is 1800 seconds, far above it, so this waits
# on the application rather than on HAProxy.
wait_empty() {
    local target=$1 timeout=$2
    split_target "$target"

    local deadline=$((SECONDS + timeout)) sessions
    while [ "$SECONDS" -lt "$deadline" ]; do
        sessions=$(stat_field "$backend" "$server" scur)
        if [ "${sessions:-0}" -eq 0 ]; then
            log "${backend}/${server} has no open sessions"
            return 0
        fi
        log "${backend}/${server} still has ${sessions} open, waiting"
        sleep 2
    done

    die "${backend}/${server} still had sessions open after ${timeout}s"
}

# Waits for HAProxy to agree the server is back.
#
# The rollout has already confirmed the server answers /health directly, so this waits only for
# HAProxy to see it: `rise` defaults to 2 checks, and the check interval on this backend is the
# default 2 seconds.
wait_up() {
    local target=$1 timeout=$2
    split_target "$target"

    local deadline=$((SECONDS + timeout)) state
    while [ "$SECONDS" -lt "$deadline" ]; do
        state=$(stat_field "$backend" "$server" status)
        # `UP` and `UP 1/100` both mean HAProxy is routing to it, which is what this waits for.
        case $state in
            UP*)
                log "${backend}/${server} is UP"
                return 0
                ;;
        esac
        sleep 2
    done

    die "${backend}/${server} did not come UP within ${timeout}s"
}

# How much of the backend is actually serving, so the rollout can refuse to empty it.
#
# Counts a backup server too: with one primary drained and a backup configured, the backup is what
# answers, so it is real capacity.
up_count() {
    local backend=$1
    runtime "show stat" | awk -F, -v px="$backend" '
        /^#/ {
            for (i = 1; i <= NF; i++) {
                name = $i
                sub(/^# */, "", name)
                if (name == "svname") svcol = i
                if (name == "status") statuscol = i
            }
            if (!svcol || !statuscol) exit 2
            next
        }
        $1 == px && $svcol != "BACKEND" && $svcol != "FRONTEND" && $statuscol ~ /^UP/ { total++ }
        END { print total + 0 }
    '
}

[ $# -ge 2 ] || usage
command=$1
shift

case $command in
    drain) set_state "$1" drain ;;
    maint) set_state "$1" maint ;;
    ready) set_state "$1" ready ;;
    state) split_target "$1"; stat_field "$backend" "$server" status ;;
    sessions) split_target "$1"; stat_field "$backend" "$server" scur ;;
    wait-empty) [ $# -eq 2 ] || usage; wait_empty "$1" "$2" ;;
    wait-up) [ $# -eq 2 ] || usage; wait_up "$1" "$2" ;;
    up-count) up_count "$1" ;;
    *) usage ;;
esac

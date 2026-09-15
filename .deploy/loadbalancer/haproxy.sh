#!/usr/bin/env bash
#
# The HAProxy runtime API, as the rollout needs it. Run on the load balancer.
#
# Every HAProxy detail lives here, so rollout.sh holds the sequence and nothing else. That includes
# knowing that one server sits in several backends: a target is `<backend[,backend...]>/<server>`,
# and the commands that change or wait on state take all of them at once. A rollout therefore never
# loops over backends, and a wait spends one deadline rather than one per backend.
#
# Fields are located by name in the `show stat` header, because that column layout is not a stable
# interface.
#
# The socket is srw------- root:haproxy. Either run as root, or give the socket `mode 660` in
# haproxy.cfg and put the operator in the haproxy group.

set -euo pipefail

# shellcheck source-path=SCRIPTDIR source=../lib.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/lib.sh"

: "${HAPROXY_SOCKET:=/run/haproxy/haproxy.sock}"

usage() {
    cat >&2 <<'EOF'
usage: haproxy.sh <command> <backend[,backend...]>/<server> [arguments]

  drain <target>               stop sending new connections, let the open ones finish
  maint <target>               take out of rotation and stop health checking it
  ready <target>               put back in rotation
  states <target>              print "backend=status" for each backend
  state <target>               print the status field of one backend, for example UP or UP 1/100
  sessions <target>            print the current session count of one backend
  wait-empty <target> <secs>   wait until every backend reports no open sessions
  wait-up <target> <secs>      wait until every backend reports UP
  up-count <backend>           print how many servers in one backend are UP
  least-up <backend[,...]>     print the smallest UP count across the backends

Every command but state, sessions, up-count and least-up takes several backends at once. The
socket path comes from HAPROXY_SOCKET.
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

# Splits `<backends>/<server>` into a backend list and a server name, one per line.
#
# A typo here would address a server that does not exist, and `set server` on an unknown name is
# not an error HAProxy reports usefully, so the shape is checked before anything is sent.
split_target() {
    local target=${1:-} backends server
    case $target in
        */*) backends=${target%%/*}; server=${target##*/} ;;
        *) die "expected <backend[,backend...]>/<server>, got '${target}'" ;;
    esac
    if [ -z "$backends" ] || [ -z "$server" ]; then
        die "expected <backend[,backend...]>/<server>, got '${target}'"
    fi
    printf '%s\n%s\n' "${backends//,/ }" "$server"
}

# One `show stat` covering every backend named, as "backend sessions status" per line.
#
# status comes last on purpose: it is not one word — HAProxy appends the check counter in a
# transitional state, so it reads "UP 1/100" — and anything after it in the line would be read as
# part of it. Callers take $3 onwards as the status and $2 as the session count.
#
# One dump answers for all the backends, so a poll costs a single socat call however many a server
# sits in. A backend that holds no such server is an error, not a silent omission.
server_rows() {
    local backends=$1 server=$2 rows expected found

    rows=$(runtime "show stat" | awk -F, -v wanted="$backends" -v sv="$server" '
        BEGIN { split(wanted, list, " "); for (i in list) want[list[i]] = 1 }
        # The header names every column, and the first is written "# pxname".
        /^#/ {
            for (i = 1; i <= NF; i++) {
                name = $i
                sub(/^# */, "", name)
                if (name == "status") status = i
                if (name == "scur") scur = i
            }
            if (!status || !scur) exit 2
            next
        }
        ($1 in want) && $2 == sv { print $1, $scur, $status }
    ')

    expected=$(printf '%s' "$backends" | wc -w)
    found=$(printf '%s' "$rows" | grep -c . || true)
    [ "$found" -eq "$expected" ] || die "${server} is not in every one of: ${backends// /, }"

    printf '%s\n' "$rows"
}

# Applies one state to the server in every backend named.
set_state() {
    local target=$1 state=$2 backends server backend answer
    { read -r backends; read -r server; } < <(split_target "$target")

    # Proves every backend really holds this server before any of them is changed.
    server_rows "$backends" "$server" >/dev/null

    for backend in $backends; do
        answer=$(runtime "set server ${backend}/${server} state ${state}")
        # A refused command answers with a message; an accepted one answers with nothing.
        [ -z "${answer//[[:space:]]/}" ] || die "HAProxy refused the change: ${answer}"
        log "${backend}/${server} set to ${state}"
    done
}

# Waits for the open sessions to finish in every backend, so a restart does not cut them off.
#
# One deadline for all of them, because they drain at the same time. The API's own request timeout
# is 150 seconds, so a draining server answers or gives up within that; `timeout server` on this
# load balancer is 1800 seconds, far above it, so this waits on the application, not on HAProxy.
wait_empty() {
    local target=$1 timeout=$2 backends server deadline open
    { read -r backends; read -r server; } < <(split_target "$target")

    deadline=$((SECONDS + timeout))
    while [ "$SECONDS" -lt "$deadline" ]; do
        open=$(server_rows "$backends" "$server" | awk '$2 != 0 { printf "%s=%s ", $1, $2 }')
        if [ -z "$open" ]; then
            log "${server} has no open sessions in ${backends// /, }"
            return 0
        fi
        log "${server} still has ${open}open, waiting"
        sleep 2
    done

    die "${server} still had sessions open after ${timeout}s"
}

# Waits for HAProxy to agree the server is back in every backend.
#
# The rollout has already confirmed the server answers /health directly, so this waits only for
# HAProxy to see it: `rise` defaults to 2 checks, and the check interval here is the default 2
# seconds. `UP` and `UP 1/100` both mean it is being routed to, which is what this waits for.
wait_up() {
    local target=$1 timeout=$2 backends server deadline pending
    { read -r backends; read -r server; } < <(split_target "$target")

    deadline=$((SECONDS + timeout))
    while [ "$SECONDS" -lt "$deadline" ]; do
        pending=$(server_rows "$backends" "$server" | awk '$3 !~ /^UP/ { printf "%s=%s ", $1, $3 }')
        if [ -z "$pending" ]; then
            log "${server} is UP in ${backends// /, }"
            return 0
        fi
        sleep 2
    done

    die "${server} did not come UP in every backend within ${timeout}s"
}

# How much of a backend is actually serving, so the rollout can refuse to empty one.
#
# Counts a backup server too: with the primaries drained and a backup configured, the backup is
# what answers, so it is real capacity.
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

# The thinnest of the backends, which is the one a drain empties first.
least_up() {
    local backend count least=''
    for backend in ${1//,/ }; do
        count=$(up_count "$backend")
        if [ -z "$least" ] || [ "$count" -lt "$least" ]; then least=$count; fi
    done
    printf '%s\n' "${least:-0}"
}

# "backend=status" per backend, for a caller that wants to report rather than wait.
states() {
    local backends server
    { read -r backends; read -r server; } < <(split_target "$1")
    server_rows "$backends" "$server" | awk '{ printf "%s=%s ", $1, $3 } END { printf "\n" }'
}

# One value of one backend, for an operator reading it rather than a rollout waiting on it.
#
# `status` keeps its counter: "UP 1/100" is what HAProxy says, and truncating it to UP would hide
# that a server is still being checked back in.
single_value() {
    local target=$1 field=$2 backends server
    { read -r backends; read -r server; } < <(split_target "$target")
    case $backends in
        *' '*) die "this command takes one backend, got '${backends// /, }'" ;;
    esac

    case $field in
        sessions) server_rows "$backends" "$server" | awk '{ print $2 }' ;;
        status) server_rows "$backends" "$server" | awk '{ $1 = ""; $2 = ""; sub(/^  */, ""); print }' ;;
    esac
}

[ $# -ge 2 ] || usage
command=$1
shift

case $command in
    drain) set_state "$1" drain ;;
    maint) set_state "$1" maint ;;
    ready) set_state "$1" ready ;;
    states) states "$1" ;;
    state) single_value "$1" status ;;
    sessions) single_value "$1" sessions ;;
    wait-empty) [ $# -eq 2 ] || usage; wait_empty "$1" "$2" ;;
    wait-up) [ $# -eq 2 ] || usage; wait_up "$1" "$2" ;;
    up-count) up_count "$1" ;;
    least-up) least_up "$1" ;;
    *) usage ;;
esac

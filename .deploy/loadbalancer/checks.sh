# shellcheck shell=bash
#
# What has to be true of the load balancer and the fleet before a rollout drains a server, one
# function per check. A check prints nothing when all is well; otherwise it prints each thing that is
# wrong as `check: …` and returns 1. None of them exits or changes anything. check_server_ready also
# passes on the server's own report, on standard output, for a caller that reads it.
#
# rollout.sh's preflight and install.sh's audit each run the ones they need, so a problem both look
# for is reported by both with the same line.
#
# Also read_inventory, which is not a check but reads what several of them are given.
#
# Uses log from core.sh and http_code from remote.sh. From the script that sources it: HAPROXY, the
# path of haproxy.sh, with HAPROXY_SOCKET set for it; REMOTE_DEPLOY; on_host HOST COMMAND, which runs
# one command on a server the way that script reaches it; and, for check_fleet_index, which only
# rollout.sh runs, its distinct_values. Beside haproxy.sh, in the checkout and on the load balancer.

# The inventory, one server per line: name host port haproxy_backends haproxy_server
#
# haproxy_backends is comma-separated, because a server can sit in more than one backend: routing
# the database endpoints separately puts every server in two. A drain has to cover all of them, or
# the server keeps taking the traffic of the one that was missed.
#
# Comments and blank lines are ignored, and so is a line too short to name its server, which
# check_inventory_entries reports. The variant is not here: the server owns that, in its environment
# file, so adding a server is one edit rather than two. Given a name, only that server's line.
read_inventory() {
    local file=$1 only=${2:-} name host port backends server
    while read -r name host port backends server _; do
        case ${name:-} in '' | \#*) continue ;; esac
        [ -n "$server" ] || continue
        [ -z "$only" ] || [ "$only" = "$name" ] || continue
        printf '%s %s %s %s %s\n' "$name" "$host" "$port" "$backends" "$server"
    done < "$file"
}

# The inventory is hand-edited, and three of its mistakes are silent: a short line, a repeated name,
# which makes one line unreachable by name, and a repeated haproxy_server, which drains one host
# while updating another. The whole file, whichever servers a run selects.
check_inventory_entries() {
    local file=$1 status=0 name host port backends server seen_names=' ' seen_servers=' '

    [ -r "$file" ] || { log "check: cannot read the inventory, ${file}"; return 1; }

    while read -r name host port backends server _; do
        case ${name:-} in '' | \#*) continue ;; esac
        if [ -z "$server" ]; then
            log "check: the inventory line for '${name}' has too few fields"
            status=1
            continue
        fi
        case $seen_names in *" ${name} "*) log "check: the inventory names '${name}' twice"; status=1 ;; esac
        case $seen_servers in *" ${server} "*) log "check: the inventory uses HAProxy server '${server}' twice"; status=1 ;; esac
        seen_names="${seen_names}${name} "
        seen_servers="${seen_servers}${server} "
        case $port in
            *[!0-9]*) log "check: ${name} has port '${port}', which is not a number"; status=1 ;;
        esac
    done < "$file"
    return "$status"
}

# The admin socket is how a rollout drains and restores a server, and the haproxy group is how the
# operator reaches it. HAProxy has to answer on it too: a socket left behind by one that stopped is
# still a socket.
check_haproxy_socket() {
    local socket=$1 config=$2 mode

    [ -S "$socket" ] || { log "check: no HAProxy admin socket at ${socket}"; return 1; }
    HAPROXY_SOCKET=$socket "$HAPROXY" servers >/dev/null \
        || { log "check: HAProxy does not answer on ${socket}"; return 1; }
    mode=$(stat -c %a "$socket")
    case $mode in
        66* | 77*) ;;
        *)
            log "check: the admin socket is mode ${mode}, so only root can use it. In ${config}, change"
            log "check:     stats socket ${socket} mode ${mode} level admin"
            log "check: to  stats socket ${socket} mode 660 level admin"
            log "check: and reload HAProxy. Do that now rather than during a rollout: a reload returns a"
            log "check: draining server to rotation."
            return 1
            ;;
    esac
}

# Every backend each inventory line names holds its server in what HAProxy is actually running,
# which is not always what haproxy.cfg says: an edit that was never reloaded is invisible to the file
# and obvious here. LINES is the inventory as "name host port backends server" per line.
check_haproxy_backends() {
    local lines=$1 running name backends server backend missing=''

    running=$("$HAPROXY" servers) || { log "check: HAProxy does not answer on ${HAPROXY_SOCKET}"; return 1; }

    while read -r name _ _ backends server _; do
        [ -n "$name" ] || continue
        for backend in ${backends//,/ }; do
            case $'\n'"${running}"$'\n' in
                *$'\n'"${backend}/${server}"$'\n'*) ;;
                *) missing="${missing}${backend}/${server} " ;;
            esac
        done
    done <<<"$lines"

    [ -z "$missing" ] || { log "check: HAProxy is not running these, which the inventory expects: ${missing% }"; return 1; }
}

# The route each backend checks, which decides what takes a server out of it: all_handlers needs the
# service up, db_handlers OpenSearch too. Only the file says, never the socket. Read, never written.
check_haproxy_health_uris() {
    local config=$1 status=0 pair backend expected actual

    [ -r "$config" ] || { log "check: cannot read ${config}"; return 1; }
    for pair in all_handlers:/health db_handlers:/health/database; do
        backend=${pair%%:*}
        expected=${pair#*:}
        actual=$(awk -v b="backend ${backend}" '
            $0 ~ "^"b"$" { inside = 1; next }
            /^(backend|frontend|listen|defaults|global)/ { inside = 0 }
            inside && /http-check send/ { for (i = 1; i <= NF; i++) if ($i == "uri") print $(i + 1) }
        ' "$config" | head -1)

        if [ -z "$actual" ]; then
            log "check: ${backend} has no http-check send uri; it should check ${expected}"
            status=1
        elif [ "$actual" != "$expected" ]; then
            log "check: ${backend} checks ${actual}; it should check ${expected}"
            status=1
        fi
    done
    return "$status"
}

# UP in every backend it sits in, possibly with a check counter after it. A server already out of
# rotation is one a rollout would put back without having been asked to.
check_server_up() {
    local name=$1 target=$2 states

    states=$("$HAPROXY" states "$target") \
        || { log "check: HAProxy cannot say how it sees ${name}, ${target}"; return 1; }
    printf '%s' "$states" | grep -qE '^([[:alnum:]_.-]+=UP[^=]*)+$' \
        || { log "check: ${name} is ${states}"; return 1; }
}

# Both routes the backends check, asked over the network as HAProxy asks them.
check_server_health() {
    local name=$1 host=$2 port=$3 status=0 route

    for route in /health /health/database; do
        [ "$(http_code "http://${host}:${port}${route}")" = 200 ] \
            || { log "check: ${name} does not answer ${route}"; status=1; }
    done
    return "$status"
}

# Reached at all. Apart from check_server_ready, so a host that is not reached says so, rather than
# that it is not ready.
check_server_reachable() {
    local name=$1 host=$2
    on_host "$host" true 2>/dev/null || { log "check: cannot reach ${name} at ${host} over ssh"; return 1; }
}

# deploy.sh is installed there and its `check` passes, with any arguments given passed on to it. Its
# report goes to standard output; a failing one is shown under the line that says so.
check_server_ready() {
    local name=$1 host=$2 report status=0
    shift 2

    report=$(on_host "$host" "${REMOTE_DEPLOY} check $*" 2>&1) || status=$?
    case $status in
        0) printf '%s\n' "$report" ;;
        # What the remote shell answers for a command it cannot find, and for one it cannot run.
        127) log "check: ${name} has no ${REMOTE_DEPLOY}; run the server install there first"; return 1 ;;
        126) log "check: ${name} cannot run ${REMOTE_DEPLOY}; run the server install there again"; return 1 ;;
        *)
            log "check: ${name} is not ready:"
            printf '%s\n' "$report" | sed 's/^/    /' >&2
            return 1
            ;;
    esac
}

# One fleet, one index. Two servers on different UniProt versions answer the same request differently
# depending on which one the load balancer picked, and every health check still passes. VERSIONS is
# "name=version " per server.
check_fleet_index() {
    local versions=$1
    [ "$(distinct_values "$versions")" -le 1 ] \
        || { log "check: the fleet does not agree on an index: ${versions}; --allow-index-mismatch accepts that"; return 1; }
}

# Another server stays UP in every backend this one sits in, so draining it is not an outage. A
# backup counts: with the primaries out, it is what answers. Asked before the run and again before
# each drain, since capacity can move in between.
check_backend_capacity() {
    local backends=$1 server=$2 status=0 backend up state

    for backend in ${backends//,/ }; do
        if ! up=$("$HAPROXY" up-count "$backend") || ! state=$("$HAPROXY" state "${backend}/${server}"); then
            log "check: HAProxy cannot say which servers are UP in ${backend}"
            return 1
        fi
        case $state in UP*) up=$((up - 1)) ;; esac
        [ "$up" -ge 1 ] \
            || { log "check: draining ${server} leaves ${backend} with no server UP; --allow-downtime accepts that"; status=1; }
    done
    return "$status"
}

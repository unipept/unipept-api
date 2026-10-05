# shellcheck shell=bash
#
# How a server is reached and asked whether it is up: the bounds on a connection to it, and polling
# its health. Needs nothing else. Sourced through .deploy/lib.sh.

# Seconds to wait for a restarted server to answer /health. Generous because the preloaded and
# hybrid builds read the index into memory before they answer, and the index is hundreds of
# gigabytes. Both the server and the load balancer start from this.
# shellcheck disable=SC2034  # read by the scripts that source this file.
readonly DEFAULT_READY_TIMEOUT=900

# What a connection to a server may not do: block for ever.
#
# ConnectTimeout alone bounds only the handshake. A server that answers and then stops holds the
# connection open, and the caller waits on it indefinitely with that server out of the pool. The
# keepalives are what end it.
#
# Here rather than in each caller, because the audit exists to reach servers "the way a rollout
# reaches them": written out twice, the two drift the first time a timeout is tuned, and the audit
# then passes on bounds no rollout ever uses. The flags that belong to the invocation rather than to
# the connection — ssh's -n, scp's -q — stay with their caller.
# shellcheck disable=SC2034  # read by the scripts that source this file.
readonly SSH_CONNECTION_BOUNDS=(-o BatchMode=yes -o ConnectTimeout=10 -o ServerAliveInterval=15 -o ServerAliveCountMax=4)

# The status a URL answers with, or 000 when it does not answer at all.
http_code() {
    curl -s -o /dev/null -w '%{http_code}' --max-time 5 "$1"
}

# Polls until the URL answers 200. Returns 1 when the timeout runs out.
wait_for_http() {
    local url=$1 timeout=$2
    local deadline=$((SECONDS + timeout))

    while [ "$SECONDS" -lt "$deadline" ]; do
        if [ "$(http_code "$url")" = "200" ]; then
            return 0
        fi
        sleep 2
    done

    return 1
}

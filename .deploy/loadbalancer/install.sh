#!/usr/bin/env bash
#
# Prepares the load balancer to run rollouts. Run once, as root.
#
# It installs the scripts, puts this host's configuration somewhere that is not a git checkout, and
# then audits what a rollout depends on: the admin socket, the backends HAProxy is actually running,
# and whether every server in the inventory can be reached.
#
# It never edits haproxy.cfg. That file holds the TLS certificates, the rate limiting and the ACLs,
# and a script that rewrites it is a script that eventually takes the public API down at the wrong
# moment. Where something is missing, the fragment is written out and the operator applies it.
#
# Flow:
#   1. Check that every command it uses is installed, that this runs as root, and that the operator
#      account exists.
#   2. Install rollout.sh, loadbalancer/haproxy.sh and lib.sh in /opt/unipept-rollout, in the shape
#      of the checkout, because rollout.sh resolves haproxy.sh relative to itself.
#   3. Write /etc/unipept-rollout/rollout.conf and servers.conf from the examples, or keep the ones
#      already there.
#   4. Put the operator in the haproxy group, which is what reaches the admin socket.
#   5. Read the socket path out of haproxy.cfg, and check that its mode lets that group use it.
#   6. Ask the socket what HAProxy is running. Report every server the inventory expects in a
#      backend that is not there, and write the configuration to add to /tmp.
#   7. Read haproxy.cfg for the health route each of the two backends checks.
#   8. Reach every server in the inventory the way a rollout reaches it: ssh as the operator,
#      deploy.sh installed, and `deploy.sh check` passing there.
#   9. Count what steps 4 to 8 reported. Nothing: say the host is ready. Otherwise exit non-zero.

set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly HERE
readonly SOURCE="${HERE}/.."

# shellcheck source-path=SCRIPTDIR source=../lib.sh
source "${SOURCE}/lib.sh"

# What `die` raises when it is called from inside a subshell.
trap 'exit 1' USR1

readonly ROOT=/opt/unipept-rollout
readonly CONFIG=/etc/unipept-rollout
readonly OPERATOR=${OPERATOR:-unipept}
readonly HAPROXY_CONFIG=/etc/haproxy/haproxy.cfg
readonly FRAGMENT=/tmp/unipept-haproxy-fragment.cfg

require_cmd chown curl getent install sha256sum socat ssh scp flock logger usermod
[ "$(id -u)" -eq 0 ] || die "run this as root. Rollouts themselves run as ${OPERATOR}."
id "$OPERATOR" >/dev/null 2>&1 || die "there is no ${OPERATOR} account on this host"

# The configuration to add when a backend is missing. Written out, never applied: haproxy.cfg holds
# the TLS certificates, the rate limiting and the ACLs, and the operator is the one who edits it.
write_fragment() {
    cat > "$FRAGMENT" <<'FRAGMENT'
# Add to haproxy.cfg, then reload. The frontend ACLs route the database-backed endpoints to their
# own backend, so a server whose OpenSearch is down stops receiving those requests while it keeps
# serving everything the index alone can answer.
#
# In the `handlers` frontend, before `use_backend all_handlers`:

    acl needs_database path_beg /api/v1/pept2prot /api/v2/pept2prot
    acl needs_database path_beg /api/v1/protinfo /api/v2/protinfo
    acl needs_database path_beg /private_api/proteins
    use_backend db_handlers if valid_path needs_database

# And as a new backend, alongside all_handlers. The server lines match all_handlers, including the
# ports, since the redirect on each server is what turns 80 into the port the service binds.

backend db_handlers
    balance leastconn
    mode http
    option httpchk
    http-check send meth GET uri /health/database

    email-alert mailers my_mailers
    email-alert from haproxy@unipeptapi.ugent.be
    email-alert to unipept@ugent.be
    email-alert level notice

    server patty patty.ugent.be:80 check fall 100 maxconn 100
    server selma selma.ugent.be:80 check fall 100 maxconn 100
    server rick  rick.ugent.be:80  check inter 10s fall 10 backup maxconn 100
FRAGMENT
    chmod 0644 "$FRAGMENT"
}

problems=0
note() { log "$*"; problems=$((problems + 1)); }

# The same shape as the checkout, because rollout.sh resolves haproxy.sh as loadbalancer/haproxy.sh
# relative to itself. Flattening it here left the installed rollout unable to find it at all.
install -d -m 0755 "$ROOT" "${ROOT}/loadbalancer" "$CONFIG"
install -m 0755 "${SOURCE}/rollout.sh" "${ROOT}/rollout.sh"
install -m 0755 "${HERE}/haproxy.sh" "${ROOT}/loadbalancer/haproxy.sh"
install -m 0644 "${SOURCE}/lib.sh" "${ROOT}/lib.sh"
log "installed the scripts in ${ROOT}"

# This host's own settings, kept out of the checkout: the inventory names the fleet and the
# configuration names where failures are emailed, neither of which belongs in a repository.
#
# The owner differs because the two files are read differently. rollout.conf is sourced — by
# rollout.sh, and by the audit below, which runs as root — so a line in it is a command root runs,
# and the operator edits it with sudoedit. servers.conf is data, read field by field, and stays the
# operator's to change.
install_config() {
    local example=$1 target=$2 owner=$3

    if [ -f "$target" ]; then
        # Its contents are this host's, but the owner is ours to correct on a file written before
        # rollout.conf became root's.
        chown "${owner}:${owner}" "$target"
        log "keeping ${target}"
        return 0
    fi
    install -m 0644 -o "$owner" -g "$owner" "$example" "$target"
    log "wrote ${target} from the example. Edit it before rolling out."
}

install_config "${SOURCE}/rollout.conf.example" "${CONFIG}/rollout.conf" root
install_config "${SOURCE}/servers.example.conf" "${CONFIG}/servers.conf" "$OPERATOR"

# The socket is the one thing a rollout cannot do without, and the only privilege it needs.
if getent group haproxy >/dev/null 2>&1; then
    usermod -a -G haproxy "$OPERATOR"
    log "${OPERATOR} is in the haproxy group"
else
    note "there is no haproxy group on this host; ${OPERATOR} cannot reach the admin socket"
fi

# `|| true` because pipefail turns a missing haproxy.cfg into a fatal exit here, which would skip
# the note below that exists to report exactly that.
socket=$(sed -n 's/.*stats socket \([^ ]*\).*/\1/p' "$HAPROXY_CONFIG" 2>/dev/null | head -1 || true)
socket=${socket:-/run/haproxy/haproxy.sock}
[ -r "$HAPROXY_CONFIG" ] || note "no ${HAPROXY_CONFIG} to read; assuming ${socket}"

if [ ! -S "$socket" ]; then
    note "no HAProxy admin socket at ${socket}"
else
    mode=$(stat -c %a "$socket")
    case $mode in
        66* | 77*) log "the admin socket is mode ${mode}, which the haproxy group can use" ;;
        *)
            note "the admin socket is mode ${mode}, so only root can use it. In ${HAPROXY_CONFIG}, change"
            note "    stats socket ${socket} mode ${mode} level admin"
            note "to  stats socket ${socket} mode 660 level admin"
            note "and reload HAProxy. Do that now rather than during a rollout: a reload returns a"
            note "draining server to rotation."
            ;;
    esac
fi

# What HAProxy is actually running, which is not always what the file says: an edit that was never
# reloaded is invisible to grep and obvious here.
if [ -S "$socket" ] && backends=$(printf 'show stat\n' | socat "$socket" stdio 2>/dev/null); then
    inventory=${CONFIG}/servers.conf
    missing=''

    while read -r name _ _ names server; do
        case ${name:-} in '' | \#*) continue ;; esac
        [ -n "$server" ] || continue
        for backend in ${names//,/ }; do
            if ! printf '%s\n' "$backends" | awk -F, -v b="$backend" -v s="$server" \
                '$1 == b && $2 == s { found = 1 } END { exit !found }'; then
                missing="${missing}${backend}/${server} "
            fi
        done
    done < "$inventory"

    if [ -n "$missing" ]; then
        note "HAProxy is not running these, which the inventory expects: ${missing}"
        write_fragment
        note "a configuration to add is in ${FRAGMENT}"
    else
        log "every server in the inventory is in every backend it names"
    fi

    # The check URI cannot be read from the socket, only from the file, so this is the one place the
    # configuration is read. Read, never written.
    for pair in "all_handlers:/health" "db_handlers:/health/database"; do
        backend=${pair%%:*}
        expected=${pair##*:}
        actual=$(awk -v b="backend ${backend}" '
            $0 ~ "^"b"$" { inside = 1; next }
            /^(backend|frontend|listen|defaults|global)/ { inside = 0 }
            inside && /http-check send/ { for (i = 1; i <= NF; i++) if ($i == "uri") print $(i + 1) }
        ' "$HAPROXY_CONFIG" 2>/dev/null | head -1)

        if [ -z "$actual" ]; then
            note "${backend} has no http-check send uri; it should check ${expected}"
        elif [ "$actual" != "$expected" ]; then
            note "${backend} checks ${actual}; it should check ${expected}"
        else
            log "${backend} checks ${expected}"
        fi
    done
fi

# Every server the inventory names, reached the way a rollout reaches it, on the options a rollout
# uses — the same array rollout.sh builds SSH_OPTIONS from, so tuning a timeout there tunes it here.
readonly AUDIT_SSH=(-n "${SSH_CONNECTION_BOUNDS[@]}")

# Read the way rollout.sh reads it, which is `source`. Matching the lines with sed instead meant
# anything shell understands and a line-matcher does not became part of the value: the example file
# comments half its settings, so `SSH_USER=unipept  # deploy account` is the natural thing for an
# operator to write, and the audit then tried to reach `unipept  # deploy account@patty` and called
# every server unreachable — while the rollout it is auditing read the same file and worked.
#
# In a subshell, so a setting here cannot land in this script's own variables. It is still root
# running what the file says, which is why install_config keeps rollout.conf root-owned.
conf_value() {
    (
        # The file names only what this host decides; the rest is unset, and reading one must not
        # end the audit.
        set +u
        # shellcheck source=/dev/null  # written on this host, not in this repository.
        source "${CONFIG}/rollout.conf" >/dev/null 2>&1 || exit 0
        printf '%s' "${!1}"
    )
}

# Every server the inventory names, reached the way a rollout reaches it.
ssh_user=$(conf_value SSH_USER)
remote=$(conf_value REMOTE_DEPLOY)
remote=${remote:-/opt/unipept-api/lib/deploy.sh}

while read -r name host _ _ _; do
    case ${name:-} in '' | \#*) continue ;; esac
    target=${ssh_user:+${ssh_user}@}${host}

    if ! sudo -u "$OPERATOR" ssh "${AUDIT_SSH[@]}" "$target" true 2>/dev/null; then
        note "${OPERATOR} cannot ssh to ${target}; install a key there"
    elif ! sudo -u "$OPERATOR" ssh "${AUDIT_SSH[@]}" "$target" "test -x ${remote}" 2>/dev/null; then
        note "${target} has no ${remote}; run the server install there first"
    elif ! sudo -u "$OPERATOR" ssh "${AUDIT_SSH[@]}" "$target" "${remote} check" >/dev/null 2>&1; then
        note "${target} is not ready; run '${remote} check' there to see why"
    else
        log "${name} is reachable and ready"
    fi
done < "${CONFIG}/servers.conf"

printf '\n' >&2
if [ "$problems" -eq 0 ]; then
    log "this load balancer is ready: ${ROOT}/rollout.sh --version <tag> --dry-run"
else
    log "${problems} thing(s) above to settle before a rollout will work"
    exit 1
fi

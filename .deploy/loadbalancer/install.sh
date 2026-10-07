#!/usr/bin/env bash
#
# Prepares the load balancer to run rollouts, and updates its scripts. Run as root, once and after
# each release.
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
#   2. Refuse a rollout.conf anyone but root can write: this runs as root and sources it.
#   3. Write /etc/unipept-rollout/rollout.conf and servers.conf from the examples, or keep the ones
#      already there.
#   4. Take the rollout lock, so no rollout runs from the files while they are replaced. Install
#      rollout.sh, loadbalancer/haproxy.sh, lib.sh and the parts it loads from lib/ in
#      /opt/unipept-rollout, in the shape of the checkout, because rollout.sh resolves haproxy.sh
#      and lib.sh relative to itself.
#   5. Put the operator in the haproxy group, which is what reaches the admin socket.
#   6. Read the socket path out of haproxy.cfg, and check that its mode lets that group use it.
#   7. Ask the socket what HAProxy is running. Report every server the inventory expects in a
#      backend that is not there, and write the configuration to add to /tmp.
#   8. Read haproxy.cfg for the health route each of the two backends checks.
#   9. Reach every server in the inventory the way a rollout reaches it: ssh as the operator,
#      deploy.sh installed, and `deploy.sh check` passing there.
#  10. Count what steps 5 to 9 reported. Nothing: say the host is ready. Otherwise exit non-zero.

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly HERE
readonly SOURCE="${HERE}/.."

[ -r "${SOURCE}/lib.sh" ] || { echo "Error: there is no ${SOURCE}/lib.sh to load." 1>&2; exit 2; }
# shellcheck source=../lib.sh
source "${SOURCE}/lib.sh"

readonly ROOT=/opt/unipept-rollout
readonly CONFIG=/etc/unipept-rollout
readonly OPERATOR=${OPERATOR:-unipept}
readonly HAPROXY_CONFIG=/etc/haproxy/haproxy.cfg
readonly FRAGMENT=/tmp/unipept-haproxy-fragment.cfg

require chown curl getent install sha256sum socat ssh scp flock logger usermod
[ "$(id -u)" -eq 0 ] || die "run this as root. Rollouts themselves run as ${OPERATOR}."
id "$OPERATOR" >/dev/null 2>&1 || die "there is no ${OPERATOR} account on this host"

# This runs as root, and sources rollout.conf for the audit. One that another user could write would
# hand that user root, whoever owns it now, so it is refused rather than taken back and read.
if [ -e "${CONFIG}/rollout.conf" ]; then
    case "$(stat -c '%U %A' "${CONFIG}/rollout.conf")" in
        "root -rw-r--r--" | "root -rw-------" | "root -r--r--r--" | "root -r--------") ;;
        *) die "${CONFIG}/rollout.conf can be written by someone other than root, and this runs as root and reads it. Make it root's, mode 0644, after checking what is in it." ;;
    esac
fi

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
    # alert, not notice: a rollout's own drain, maintenance and restore are logged at notice, so
    # notice mails eighteen times for a fleet of three. A failed check is alert, an empty backend
    # emerg. rollout.sh mails what a rollout did.
    email-alert level alert

    server patty patty.ugent.be:80 check fall 100 maxconn 100
    server selma selma.ugent.be:80 check fall 100 maxconn 100
    server rick  rick.ugent.be:80  check inter 10s fall 10 backup maxconn 100
FRAGMENT
    chmod 0644 "$FRAGMENT"
}

problems=0
note() { log "$*"; problems=$((problems + 1)); }

# One setting of rollout.conf, read with `source` as rollout.sh reads it, so a value means the same
# to the audit as to a rollout: `SSH_USER=unipept  # deploy account` is the user unipept to both.
#
# In a subshell, so a setting here cannot land in this script's own variables. It is still root
# running what the file says, which is why a rollout.conf others can write is refused above.
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

install -d -m 0755 "$ROOT" "$CONFIG"

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
        # Its contents are this host's. A rollout.conf root does not own was refused above, so this
        # only ever hands servers.conf back to the operator.
        chown "${owner}:${owner}" "$target"
        log "keeping ${target}"
        return 0
    fi
    install -m 0644 -o "$owner" -g "$owner" "$example" "$target"
    log "wrote ${target} from the example. Edit it before rolling out."
}

install_config "${SOURCE}/rollout.conf.example" "${CONFIG}/rollout.conf" root
install_config "${SOURCE}/servers.example.conf" "${CONFIG}/servers.conf" "$OPERATOR"

# A rollout running while this replaces its files could load one release's lib.sh into another's
# rollout.sh. So this takes the rollout's own lock for as long as it runs, and refuses while a
# rollout holds it.
take_rollout_lock || die "$(rollout_lock_refused $?). Install once it has finished."

# The same shape as the checkout, because rollout.sh resolves haproxy.sh as loadbalancer/haproxy.sh
# relative to itself, and haproxy.sh lib.sh one level up. A release, made whole in releases/ and put
# in place at once by switch_release: each file a script opens is one whole release's, and a rollout,
# which would pair two, is refused the lock this holds; an install stopped part way leaves the one
# before, and nothing the checkout no longer has lingers. Named after when it was made, and by which
# run, so a second install makes its own.
release_id="$(date -u +%Y%m%dT%H%M%SZ).$$"
release="${ROOT}/releases/${release_id}"
install -d -m 0755 -o root -g root "${ROOT}/releases"
install -d -m 0755 "${release}/lib" "${release}/loadbalancer"
install -m 0644 "${SOURCE}/lib/"*.sh "${release}/lib/"
install -m 0644 "${SOURCE}/lib.sh" "${release}/lib.sh"
install -m 0755 "${SOURCE}/rollout.sh" "${release}/rollout.sh"
install -m 0755 "${HERE}/haproxy.sh" "${release}/loadbalancer/haproxy.sh"
switch_release "$ROOT" "$release_id" lib lib.sh loadbalancer rollout.sh
log "installed the scripts in ${ROOT}"

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

# Every server the inventory names, reached the way a rollout reaches it.
ssh_user=$(conf_value SSH_USER)
remote=$(conf_value REMOTE_DEPLOY)
remote=${remote:-$DEFAULT_REMOTE_DEPLOY}

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

#!/usr/bin/env bash
#
# Prepares the load balancer to run rollouts, and updates its scripts. Run as root, once and after
# each release.
#
# It installs the scripts, puts this host's configuration somewhere that is not a git checkout, and
# then audits what a rollout depends on: the inventory, the admin socket, the backends HAProxy is
# actually running, and whether every server in the inventory can be reached. The audit runs the
# checks in checks.sh, the ones a rollout's preflight runs too.
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
#      rollout.sh, loadbalancer/haproxy.sh and checks.sh, lib.sh and the parts it loads from lib/
#      in /opt/unipept-rollout, in the shape of the checkout, because rollout.sh resolves the
#      loadbalancer/ scripts and lib.sh relative to itself.
#   5. Put the operator in the haproxy group, which is what reaches the admin socket.
#   6. Check the inventory.
#   7. Read the socket path out of haproxy.cfg, and check that its mode lets that group use it.
#   8. Ask the socket what HAProxy is running. Report every server the inventory expects in a
#      backend that is not there, and write the configuration to add to /tmp.
#   9. Check the health route each of the two backends checks, in haproxy.cfg.
#  10. Reach every server in the inventory the way a rollout reaches it: ssh as the operator,
#      deploy.sh installed, and `deploy.sh check` passing there.
#  11. Count what steps 5 to 10 reported. Nothing: say the host is ready. Otherwise exit non-zero.

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly HERE
readonly SOURCE="${HERE}/.."

[ -r "${SOURCE}/lib.sh" ] || { echo "Error: there is no ${SOURCE}/lib.sh to load." 1>&2; exit 2; }
# shellcheck source=../lib.sh
source "${SOURCE}/lib.sh"
# shellcheck source=checks.sh
source "${HERE}/checks.sh"

readonly ROOT=/opt/unipept-rollout
readonly CONFIG=/etc/unipept-rollout
readonly OPERATOR=${OPERATOR:-unipept}
readonly HAPROXY_CONFIG=/etc/haproxy/haproxy.cfg
readonly FRAGMENT=/tmp/unipept-haproxy-fragment.cfg
readonly INVENTORY=${CONFIG}/servers.conf
readonly HAPROXY=${HERE}/haproxy.sh

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
install_config "${SOURCE}/servers.example.conf" "$INVENTORY" "$OPERATOR"

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
install -m 0644 "${HERE}/checks.sh" "${release}/loadbalancer/checks.sh"
switch_release "$ROOT" "$release_id" lib lib.sh loadbalancer rollout.sh
log "installed the scripts in ${ROOT}"

# The socket is the one thing a rollout cannot do without, and the only privilege it needs.
if getent group haproxy >/dev/null 2>&1; then
    usermod -a -G haproxy "$OPERATOR"
    log "${OPERATOR} is in the haproxy group"
else
    log "there is no haproxy group on this host; ${OPERATOR} cannot reach the admin socket"
    problems=$((problems + 1))
fi

check_inventory_entries "$INVENTORY" || problems=$((problems + 1))

# The inventory as rollout.sh reads it, one "name host port backends server" per server.
inventory=$(awk '$1 !~ /^#/ && NF >= 5 { print $1, $2, $3, $4, $5 }' "$INVENTORY")

# `|| true` because pipefail turns a missing haproxy.cfg into a fatal exit here, which
# check_haproxy_health_uris below exists to report.
socket=$(sed -n 's/.*stats socket \([^ ]*\).*/\1/p' "$HAPROXY_CONFIG" 2>/dev/null | head -1 || true)
socket=${socket:-/run/haproxy/haproxy.sock}
[ -r "$HAPROXY_CONFIG" ] || log "no ${HAPROXY_CONFIG} to read; assuming ${socket}"
export HAPROXY_SOCKET=$socket

if ! check_haproxy_socket "$socket" "$HAPROXY_CONFIG"; then
    problems=$((problems + 1))
elif check_haproxy_backends "$inventory"; then
    log "every server in the inventory is in every backend it names"
else
    problems=$((problems + 1))
    write_fragment
    log "a configuration to add is in ${FRAGMENT}"
fi

check_haproxy_health_uris "$HAPROXY_CONFIG" || problems=$((problems + 1))

# Every server the inventory names, reached the way a rollout reaches it: as the operator, on the
# options a rollout uses — the same bounds rollout.sh builds SSH_OPTIONS from, so tuning a timeout
# there tunes it here — and as the user rollout.conf names.
readonly AUDIT_SSH=(-n "${SSH_CONNECTION_BOUNDS[@]}")
ssh_user=$(conf_value SSH_USER)
REMOTE_DEPLOY=$(conf_value REMOTE_DEPLOY)
REMOTE_DEPLOY=${REMOTE_DEPLOY:-$DEFAULT_REMOTE_DEPLOY}

on_host() {
    sudo -u "$OPERATOR" ssh "${AUDIT_SSH[@]}" "${ssh_user:+${ssh_user}@}$1" "$2"
}

while read -r name host _; do
    [ -n "$name" ] || continue
    if check_server_reachable "$name" "$host" && check_server_ready "$name" "$host" >/dev/null; then
        log "${name} is reachable and ready"
    else
        problems=$((problems + 1))
    fi
done <<<"$inventory"

printf '\n' >&2
if [ "$problems" -eq 0 ]; then
    log "this load balancer is ready: ${ROOT}/rollout.sh --version <tag> --dry-run"
else
    log "${problems} thing(s) above to settle before a rollout will work"
    exit 1
fi

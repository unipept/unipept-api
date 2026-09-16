#!/usr/bin/env bash
#
# Prepares a server to run the API. Run once per host, as root.
#
# This is the only step that needs root. Afterwards the service user owns everything a deploy
# touches and restarts its own unit, so no deploy uses sudo.
#
# It installs no binary: deploy.sh does that, here and on every release after it.

set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly HERE

# shellcheck source-path=SCRIPTDIR source=../lib.sh
source "${HERE}/../lib.sh"

# What `die` raises when it is called from inside a subshell.
trap 'exit 1' USR1

readonly SERVICE=unipept-api
readonly USER=unipept
readonly ROOT=/opt/unipept-api
readonly ENV_FILE="${ROOT}/etc/unipept-api.env"

require_cmd getent install iptables loginctl setpriv systemctl useradd usermod
[ "$(id -u)" -eq 0 ] || die "run this as root. It is the only step that needs it."

# A home directory, because a user unit lives in it. A real shell, because the rollout runs
# `ssh unipept@host .../deploy.sh`, and sshd execs a remote command through the login shell:
# /usr/sbin/nologin answers "This account is currently not available" and runs nothing.
if id "$USER" >/dev/null 2>&1; then
    log "the ${USER} user is already there"
    # An account created before this script may still carry nologin, which would fail every deploy.
    case $(getent passwd "$USER" | cut -d: -f7) in
        *nologin | *false) usermod --shell /bin/bash "$USER"; log "gave ${USER} a login shell, for ssh" ;;
    esac
else
    useradd --create-home --shell /bin/bash "$USER"
    log "created the ${USER} user"
fi

home=$(getent passwd "$USER" | cut -d: -f6)
if [ -z "$home" ] || [ ! -d "$home" ]; then
    die "${USER} has no home directory, and a user unit needs one"
fi
unit_directory="${home}/.config/systemd/user"

# Owned by the service user, so a deploy replaces the binary without privilege.
install -d -m 0755 -o "$USER" -g "$USER" "$ROOT" "${ROOT}/bin" "${ROOT}/etc" "${ROOT}/lib"

# Never overwritten: it holds this host's index path, its port and its storage backend.
if [ -f "$ENV_FILE" ]; then
    # Its contents are this host's, but the mode is ours to correct on an existing file.
    chmod 0600 "$ENV_FILE"
    log "keeping the environment file already at ${ENV_FILE}"
else
    # 0600: DATABASE_ADDRESS can carry credentials, and nothing but the service reads this.
    install -m 0600 -o "$USER" -g "$USER" "${HERE}/unipept-api.env.example" "$ENV_FILE"
    log "wrote ${ENV_FILE} from the example. Edit it before starting the service."
fi

# The rollout runs these over SSH as the service user, so they sit at a fixed path it owns.
# Re-running install.sh is how they are updated.
install -m 0755 -o "$USER" -g "$USER" "${HERE}/deploy.sh" "${ROOT}/lib/deploy.sh"
install -m 0644 -o "$USER" -g "$USER" "${HERE}/../lib.sh" "${ROOT}/lib/lib.sh"
log "installed ${ROOT}/lib/deploy.sh"

# The service cannot bind port 80 itself, so a netfilter rule sends 80 to the port it does bind.
# Root-owned, because only root can change netfilter and nothing about a deploy should be able to.
install -m 0755 "${HERE}/unipept-api-ports.sh" "${ROOT}/lib/unipept-api-ports.sh"
install -m 0644 "${HERE}/unipept-api-ports.service" /etc/systemd/system/unipept-api-ports.service
log "installed the port redirect"

install -d -m 0755 -o "$USER" -g "$USER" "$unit_directory"
install -m 0644 -o "$USER" -g "$USER" "${HERE}/unipept-api.service" "${unit_directory}/${SERVICE}.service"
log "installed ${unit_directory}/${SERVICE}.service"

systemctl daemon-reload
# Enabled so it returns after a reboot, and started now so the port works before the first deploy.
systemctl enable --now unipept-api-ports
log "port 80 reaches $(env_value PORT "$ENV_FILE")"

# Without lingering, the user manager stops when the last session ends, and starts no unit at boot.
loginctl enable-linger "$USER"
log "enabled lingering for ${USER}"

# As the service user, against its own manager. The runtime directory exists once lingering is on.
runtime="/run/user/$(id -u "$USER")"
for _ in $(seq 20); do [ -d "$runtime" ] && break; sleep 0.5; done
[ -d "$runtime" ] || die "${runtime} did not appear; check systemd-logind"

as_user() {
    setpriv --reuid "$USER" --regid "$USER" --init-groups \
        env XDG_RUNTIME_DIR="$runtime" "$@"
}

as_user systemctl --user daemon-reload
as_user systemctl --user enable "$SERVICE"
log "enabled ${SERVICE}, not started: it has no binary until deploy.sh runs"

cat >&2 <<EOF

Still to do on this host:
  1. Edit ${ENV_FILE}: INDEX_LOCATION, DATABASE_ADDRESS, PORT and VARIANT.
  2. Make the index directory readable by ${USER}, and keep it out of /home.
  3. Give ${USER} an authorized_keys for the load balancer, if this host is rolled out to.
  4. As ${USER}: ${ROOT}/lib/deploy.sh deploy --version <tag>

HAProxy keeps its server lines on port 80: the redirect installed here sends 80 to PORT inside this
host, so nothing on the network changes. Changing PORT later means re-running this script.
EOF

# What is still wrong, now, rather than at the first deploy.
#
# Everything above is installed with values nobody has edited yet, so this is expected to report
# problems on a first run. Saying what they are turns "install, deploy, read a failure, edit, deploy
# again" into "install, read this list, edit, re-run".
printf '\n' >&2
log "checking this host against ${ENV_FILE}"

# One call: the key=value lines are not useful here and go to /dev/null, the problems go to the
# terminal on stderr, and the exit status decides what to say about them.
if as_user "${ROOT}/lib/deploy.sh" check >/dev/null; then
    log "this host is ready; the deploy above is the only step left"
else
    printf '\n' >&2
    log "the lines above are what to fix before deploying. Then re-run this script, or:"
    log "  sudo -u ${USER} ${ROOT}/lib/deploy.sh check"
fi

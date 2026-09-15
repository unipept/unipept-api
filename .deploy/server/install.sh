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

readonly SERVICE=unipept-api
readonly USER=unipept
readonly ROOT=/opt/unipept-api
readonly ENV_FILE="${ROOT}/etc/unipept-api.env"

require_cmd getent install loginctl setpriv systemctl useradd usermod
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

install -d -m 0755 -o "$USER" -g "$USER" "$unit_directory"
install -m 0644 -o "$USER" -g "$USER" "${HERE}/unipept-api.service" "${unit_directory}/${SERVICE}.service"
log "installed ${unit_directory}/${SERVICE}.service"

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
  5. Point HAProxy's server lines at PORT.
EOF

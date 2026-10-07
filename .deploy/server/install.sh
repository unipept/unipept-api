#!/usr/bin/env bash
#
# Prepares a server to run the API. Run once per host, as root.
#
# This is the only step that needs root. Afterwards the service user owns everything a deploy
# touches and restarts its own unit, so no deploy uses sudo.
#
# It installs no binary: deploy.sh does that, here and on every release after it.
#
# Flow:
#   1. Check that every command it uses is installed, and that this runs as root. Take the API lock,
#      so no deploy, rollback, start or stop runs while this changes the host.
#   2. Create the unipept user, or give an account that already exists a login shell: the rollout
#      runs the deploy over ssh, and sshd needs a shell to exec a remote command.
#   3. Create /opt/unipept-api, root's, and its bin and etc directories, owned by that user.
#   4. Write etc/unipept-api.env from the example, or keep the file already there.
#   5. Install deploy.sh, the checks it makes and lib.sh in deploy/, laid out as the checkout lays
#      them out: deploy/server/deploy.sh is the path the rollout calls over ssh. Staged whole and
#      swapped in by a rename, so nothing the checkout no longer has lingers.
#   6. Install the port redirect script, in root/, and its system unit, both owned by root.
#   7. Install the service unit in the service user's ~/.config/systemd/user.
#   8. Enable and restart unipept-api-ports, so port 80 reaches the port the service binds.
#   9. Enable lingering for the service user, and wait for its runtime directory to appear.
#  10. Enable the user unit as that user. It is not started: there is no binary until a deploy.
#      Let go of the API lock.
#  11. Print what is left to do by hand on this host.
#  12. Run `deploy.sh check`, and report what is still wrong now rather than at the first deploy.

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly HERE

# shellcheck source=../lib.sh
source "${HERE}/../lib.sh" || exit 2

readonly SERVICE=unipept-api
readonly USER=unipept
readonly ROOT=/opt/unipept-api
readonly ENV_FILE="${ROOT}/etc/unipept-api.env"

require flock getent install iptables loginctl setpriv systemctl useradd usermod
[ "$(id -u)" -eq 0 ] || die "run this as root. It is the only step that needs it."

# A deploy, rollback, start or stop running while this changes the host could load one release's
# lib.sh into another's deploy.sh, or restart the service on a unit being replaced. So this holds the
# API lock until the unit is enabled, and refuses, before it changes anything, while anything else
# holds it. The service user can take the lock file this leaves: it is made 0644.
take_api_lock || die "$(api_lock_refused $?). Install once it has finished."

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

# The root of the install is root's, so the service user cannot rename what root runs out of it and
# put its own in its place. bin/ and etc/ are the service user's, so a deploy replaces the binary,
# and the service reads its settings, without privilege.
install -d -m 0755 -o root -g root "$ROOT"
install -d -m 0755 -o "$USER" -g "$USER" "${ROOT}/bin" "${ROOT}/etc"

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

# The rollout runs deploy.sh over ssh as the service user, so it sits at a fixed path. Laid out as
# the checkout lays it out, deploy/ for .deploy/, so it finds lib.sh one level up in both. Root's,
# since only this script changes it; the service user runs it and changes nothing in it. Re-running
# install.sh is how it is updated.
#
# Staged whole, then swapped in by a rename: nothing the checkout no longer has lingers, and a
# deploy started meanwhile finds one release or the other, and is refused the lock this holds.
staging="${ROOT}/.staging"
rm -rf "${staging:?}"
install -d -m 0755 "${staging}/deploy/lib" "${staging}/deploy/server"
install -m 0644 "${HERE}/../lib.sh" "${staging}/deploy/"
install -m 0644 "${HERE}/../lib/"*.sh "${staging}/deploy/lib/"
install -m 0644 "${HERE}/checks.sh" "${staging}/deploy/server/"
install -m 0755 "${HERE}/deploy.sh" "${staging}/deploy/server/"
# The old one is moved aside before the new one takes its name, and removed once it has.
rm -rf "${ROOT:?}/deploy.old"
[ ! -e "${ROOT}/deploy" ] || mv "${ROOT}/deploy" "${ROOT}/deploy.old"
mv "${staging}/deploy" "${ROOT}/deploy"
rm -rf "${ROOT:?}/deploy.old"
rmdir "$staging"
log "installed ${ROOT}/deploy/server/deploy.sh"

# The service cannot bind port 80 itself, so a netfilter rule sends 80 to the port it does bind.
# Root-owned, in a directory root owns, because root runs it: only root can change netfilter, and
# nothing about a deploy should be able to.
install -d -m 0755 -o root -g root "${ROOT}/root"
install -m 0755 "${HERE}/unipept-api-ports.sh" "${ROOT}/root/unipept-api-ports.sh"
install -m 0644 "${HERE}/unipept-api-ports.service" /etc/systemd/system/unipept-api-ports.service
log "installed the port redirect"

install -d -m 0755 -o "$USER" -g "$USER" "$unit_directory"
install -m 0644 -o "$USER" -g "$USER" "${HERE}/unipept-api.service" "${unit_directory}/${SERVICE}.service"
log "installed ${unit_directory}/${SERVICE}.service"

systemctl daemon-reload
# enable for the next boot, restart for now. `enable --now` would be a no-op on a re-run: the unit
# is Type=oneshot with RemainAfterExit, so systemd already considers it active and starts nothing —
# leaving the old rules in place after a PORT change or an update to the script.
systemctl enable unipept-api-ports
systemctl restart unipept-api-ports
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

# Held until here, so no deploy restarts the service on a unit or a port redirect being replaced. Let
# go before what is left, which changes nothing, so the check at the end does not hold it.
exec 7<&-

cat >&2 <<EOF

Still to do on this host:
  1. Edit ${ENV_FILE}: INDEX_LOCATION, DATABASE_ADDRESS, PORT and VARIANT.
  2. Make the index directory readable by ${USER}, and keep it out of /home.
  3. Give ${USER} an authorized_keys for the load balancer, if this host is rolled out to.
  4. As ${USER}: ${ROOT}/deploy/server/deploy.sh deploy --version <tag>

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
if as_user "${ROOT}/deploy/server/deploy.sh" check >/dev/null; then
    log "this host is ready; the deploy above is the only step left"
else
    printf '\n' >&2
    log "the lines above are what to fix before deploying. Then re-run this script, or:"
    log "  sudo -u ${USER} ${ROOT}/deploy/server/deploy.sh check"
fi

# shellcheck shell=bash
#
# Shared by the deploy scripts. Sourced, never run. Sourcing it sets the shell options and the traps
# every script runs with: see lib/core.sh.
#
# Each part in its own file under lib/, and each says in its header what it uses of the others. They
# define functions and settings, and run nothing that needs another part while being sourced, so
# their order does not matter: what every script runs on, how settings are read, the locks, a
# release of the API, and how a server is reached.

# The directory of this file, which is the repository's .deploy in a checkout and the directory
# install.sh puts it in on a host. Not HERE, which belongs to the script that sources it.
DEPLOY_DIR="${BASH_SOURCE[0]%/*}"

# shellcheck source=lib/core.sh
source "${DEPLOY_DIR}/lib/core.sh"
# shellcheck source=lib/config.sh
source "${DEPLOY_DIR}/lib/config.sh"
# shellcheck source=lib/locks.sh
source "${DEPLOY_DIR}/lib/locks.sh"
# shellcheck source=lib/release.sh
source "${DEPLOY_DIR}/lib/release.sh"
# shellcheck source=lib/remote.sh
source "${DEPLOY_DIR}/lib/remote.sh"

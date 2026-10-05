# shellcheck shell=bash
#
# Shared by the deploy scripts. Sourced, never run.
#
# Each part in its own file under lib/, and each says in its header what it uses of the others. They
# define functions and settings and run nothing while being sourced, so their order does not matter:
# what every script runs on, how settings are read, the locks, a release of the API, and how a
# server is reached. The same layout as unipept-database's .deploy/lib.sh.

# The directory of this file, which is the repository's .deploy in a checkout and the directory
# install.sh puts it in on a host. Not HERE, which belongs to the script that sources it.
LIB_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib"

# shellcheck source-path=SCRIPTDIR source=lib/core.sh
source "${LIB_DIR}/core.sh"
# shellcheck source-path=SCRIPTDIR source=lib/config.sh
source "${LIB_DIR}/config.sh"
# shellcheck source-path=SCRIPTDIR source=lib/locks.sh
source "${LIB_DIR}/locks.sh"
# shellcheck source-path=SCRIPTDIR source=lib/release.sh
source "${LIB_DIR}/release.sh"
# shellcheck source-path=SCRIPTDIR source=lib/remote.sh
source "${LIB_DIR}/remote.sh"

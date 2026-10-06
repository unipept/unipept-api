# shellcheck shell=bash
#
# What the suites share: for now only the assertions they make. Sourced, never run.

# check, check_true, check_absent, section and summary.
# shellcheck source=assertions.sh
source "${BASH_SOURCE[0]%/*}/assertions.sh"

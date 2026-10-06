# shellcheck shell=bash
#
# What the suites share: for now only the assertions they make. Sourced, never run.

TESTS_DIR="$(cd "${BASH_SOURCE[0]%/*}" && pwd)"

# check, check_true, check_absent, section and summary.
# shellcheck source=assertions.sh
source "${TESTS_DIR}/assertions.sh"

# shellcheck shell=bash
#
# What the suites share. Sourced, never run.

TESTS_DIR="$(cd "${BASH_SOURCE[0]%/*}" && pwd)"

# check, check_true, check_absent, section and summary.
# shellcheck source=assertions.sh
source "${TESTS_DIR}/assertions.sh"

# One check, through the one_check the suite defines, which runs it as its script would and leaves
# what it printed in /tmp/one-check.log: passing on a good input without a word, and failing on a bad
# one, saying what is wrong.
both_ways() {
  local name=$1 good=$2 bad=$3 says=$4
  one_check "$good"; check "${name} passes" "$?" "0"
  check "and prints nothing" "$(grep -c 'check:' /tmp/one-check.log)" "0"
  one_check "$bad"; check "${name} fails" "$?" "1"
  check "and says so" "$(grep -c -- "$says" /tmp/one-check.log)" "1"
}

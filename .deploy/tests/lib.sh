# shellcheck shell=bash
#
# The assertions the suites share. Sourced, never run.

pass=0
fail=0

# Compares what happened with what should have happened, and keeps going either way: a suite reports
# everything that is wrong in one run rather than stopping at the first.
check() {
    local what=$1 got=$2 want=$3

    if [ "$got" = "$want" ]; then
        printf '  PASS %s\n' "$what"
        pass=$((pass + 1))
    else
        printf '  FAIL %s: expected [%s] got [%s]\n' "$what" "$want" "$got"
        fail=$((fail + 1))
    fi
}

# For a condition that is true or not, rather than a value to compare.
check_true() {
    local what=$1
    shift
    if "$@"; then check "$what" yes yes; else check "$what" no yes; fi
}

section() {
    printf '== %s ==\n' "$*"
}

# The last thing a suite runs. Non-zero if anything failed, so the runner can stop.
summary() {
    printf '\npassed=%s failed=%s\n' "$pass" "$fail"
    [ "$fail" -eq 0 ]
}

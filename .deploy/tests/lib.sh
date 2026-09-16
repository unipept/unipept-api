# shellcheck shell=bash
#
# The assertions the suites share. Sourced, never run.

pass=0
fail=0

# The case being run. A failure prints it, because the assertion name alone is read out of a log
# where the heading above it is hundreds of lines away, or lost in a fold.
current_section=''
# Every failure, collected so the summary can repeat them.
failures=''


# Compares what happened with what should have happened, and keeps going either way: a suite reports
# everything that is wrong in one run rather than stopping at the first.
check() {
    local what=$1 got=$2 want=$3

    if [ "$got" = "$want" ]; then
        printf '  PASS %s\n' "$what"
        pass=$((pass + 1))
    else
        # The section is on the failure only. On a pass it is the line above, and repeating it on
        # every one of three hundred would bury the failures it exists to find.
        printf '  FAIL [%s] %s: expected [%s] got [%s]\n' "${current_section:-no section}" "$what" "$want" "$got"
        failures="${failures}  [${current_section:-no section}] ${what}: expected [${want}] got [${got}]"$'\n'
        fail=$((fail + 1))
    fi
}

# For a condition that is true or not, rather than a value to compare.
check_true() {
    local what=$1
    shift
    if "$@"; then check "$what" yes yes; else check "$what" no yes; fi
}

# Names the case that follows, and is what a failure inside it reports. Every case goes through
# here: a heading printed with `echo` records nothing, so its failures would name no case.
section() {
    current_section="$*"
    printf '== %s ==\n' "$*"
}

# The last thing a suite runs. Non-zero if anything failed, so the runner can stop.
#
# The failures are repeated here because the run is long: by the time it ends, the first one is far
# enough up the log that the summary is the only place both are visible at once.
summary() {
    if [ "$fail" -gt 0 ]; then
        printf '\nwhat failed:\n%s' "$failures"
    fi
    printf '\npassed=%s failed=%s\n' "$pass" "$fail"
    [ "$fail" -eq 0 ]
}

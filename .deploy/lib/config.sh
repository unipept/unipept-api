# shellcheck shell=bash
#
# How a script reads its settings: the `key=value` lines of an environment file, or of what
# `deploy.sh status` prints. Needs nothing else. Sourced through .deploy/lib.sh.

# Reads one key out of `key=value` lines: a systemd environment file when given a path, otherwise
# standard input, which is the shape `deploy.sh status` prints for the rollout to read.
#
# Returns non-zero for a file it cannot read rather than calling `die`. `status` and `check` both
# describe a host that is broken, and a helper that stopped the run would leave them unable to say
# what is wrong with it.
env_value() {
    local key=$1 file=${2:-}

    if [ -n "$file" ]; then
        [ -f "$file" ] || return 1
        sed -n "s/^${key}=//p" "$file" | tail -1
    else
        sed -n "s/^${key}=//p" | tail -1
    fi
}

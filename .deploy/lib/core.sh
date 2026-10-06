# shellcheck shell=bash
#
# What every deploy script runs on: logging, stopping, and the commands it needs. Needs nothing
# else. Sourced through .deploy/lib.sh, never run.

# The script's own PID, captured before any subshell can shadow it, so `die` can stop the run from
# inside one. Each script arms the trap that answers it.
readonly MAIN_PID=$$

# Prints a line on standard error, stamped with the time of day in UTC.
log() {
    printf '%s  %s\n' "$(date -u '+%H:%M:%S')" "$*" >&2
}

# Stops the run, from anywhere.
#
# `exit` alone is not enough: inside $( ), < ( ) or a pipeline it ends only that subshell, and the
# script carries on with a message printed and nothing else changed. Reading an inventory, fetching a
# release and checking a file are all done that way, so this has to work there or a failure is
# announced and then ignored.
#
# $$ stays the script's own PID inside a subshell while BASHPID is the subshell's, which is how one
# tells the two apart. USR1 rather than TERM, so that a caller can keep telling a real interrupt from
# a failure; the script traps it and exits 1, running whatever cleanup it has registered.
die() {
    log "error: $*"
    [ "$$" = "$BASHPID" ] || kill -USR1 "$MAIN_PID" 2>/dev/null
    exit 1
}

# Stops the run, naming the first one missing, unless every command given is installed.
require_cmd() {
    local cmd
    for cmd in "$@"; do
        command -v "$cmd" >/dev/null || die "$cmd is not installed"
    done
}

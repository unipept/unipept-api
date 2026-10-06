# shellcheck shell=bash
#
# What every deploy script runs on: the shell options, logging, stopping, the commands a script
# needs, and the trap that reports a command that failed. Needs nothing else. Sourced through
# .deploy/lib.sh, never run. Byte for byte the same as its copy in the other repository that
# deploys Unipept, so a change here is made there in the same release.
#
# What a script's exit status means, for every script that sources this:
#   0    done, or yes
#   1    a check found a problem, or no
#   2    an error, or a usage error
#   3    what was asked about is not there
#   130  interrupted
# A variable used before it is set is the one exception: the shell stops on it itself, with its own
# message and status, before any trap here runs.

# Stops at the first command that fails where nothing expected it to, at a variable that was never
# set, and at a failure anywhere in a pipeline, and runs the ERR trap below in functions and
# subshells too.
set -Eeuo pipefail

# The script's own process, captured before any subshell can shadow it. A die inside a command
# substitution only ends that subshell, and the caller then reports the same failure a second time
# through the ERR trap, so die signals the script itself. USR1 rather than TERM, so a real
# interrupt still reads as one.
readonly MAIN_PID=$$
trap 'exit 2' USR1

# Prints a line on standard error, stamped with the date and time in UTC. Standard error, so a
# function whose output is captured can still say what it is doing.
log() { printf '%s  %s\n' "$(date -u +'%F %TZ')" "$*" >&2; }

# Stops the script, from anywhere in it, saying why.
die() {
    echo "Error: $*" >&2
    [ "$$" = "$BASHPID" ] || kill -USR1 "$MAIN_PID" 2> /dev/null
    exit 2
}

# Stops unless every command given is installed. A command written as command:package names what
# to install, where that is not the command itself: flock:util-linux.
require() {
    local wanted command

    for wanted in "$@"; do
        command=${wanted%%:*}
        command -v "$command" > /dev/null && continue
        case $wanted in
            *:*) die "${command} is not installed. It comes with ${wanted#*:}." ;;
            *) die "${command} is not installed." ;;
        esac
    done
}

# Stops a flag from swallowing the next flag, or nothing at all, as its value.
need_value() {
    local flag=$1 value=${2:-}

    { [ -n "$value" ] && [[ "$value" != --* ]]; } || die "${flag} requires a value."
}

# Stops on an option the script does not take.
unknown_option() {
    die "unknown option '$1'. Run with --help for the options."
}

# The ERR trap: a command that failed where nothing expected it to. Names the script, the command,
# and the file and line it is on, which is a part of lib.sh where it failed in one, or the script
# itself for code given to `bash -c` or `bash -s`, which has no file. In a subshell, such as a
# command substitution or a process substitution, it says nothing and passes the status on: -E runs
# it there even where the shell that started the subshell expects the failure, as in
# `x=$(...) || return 1`, and a subshell cannot tell. Where that shell acts on the status, as an
# assignment from $(...) does, its own trap reports the failure once, at the line that started the
# subshell. Where it does not, as in `echo "$(...)"`, nothing is reported, as set -e alone would not
# stop there either.
on_error() {
    local status=$? line=${BASH_LINENO[0]} file=${BASH_SOURCE[1]-$0} command=$BASH_COMMAND

    [ "$$" = "$BASHPID" ] || exit "$status"
    echo "Error: ${0##*/} stopped: '${command}' failed with exit status ${status} at line ${line} of ${file}." >&2
    exit 2
}
trap on_error ERR

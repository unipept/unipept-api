# shellcheck shell=bash
#
# The locks that keep the deploy scripts on one host from working on the same thing at once: the
# rollout's own, which rollout.sh holds for a run and loadbalancer/install.sh while it replaces the
# scripts a run loads; and the API's, which deploy.sh holds while it changes the service and
# server/install.sh while it replaces deploy.sh. Uses die, from core.sh. Sourced through .deploy/lib.sh.

# Where the locks are: in /run/lock, which every account can make a file in and nothing ages out.
# Fixed, so every script that takes one names the same file. The API lock is taken by more than this
# repository's scripts, which find it in deploy.sh status.
# shellcheck disable=SC2034  # read by the scripts that source this file.
readonly ROLLOUT_LOCK=/run/lock/unipept-rollout.lock
readonly API_LOCK=/run/lock/unipept-api.lock

# Opens a lock file for flock on file descriptor 7 or 9, made first where no one has made it. Fails
# with 2 where the file cannot be made, 3 where it cannot be read.
#
# Opened for reading, which is all flock needs, so any account can take a lock another one made: in
# a sticky directory such as /run/lock, opening another account's file to write is refused, even to
# root.
#
# Made 0644 whoever makes it: root's umask may be 077, and the service user or the operator could
# then not read a lock an install left behind.
open_lock() {
    local fd=$1 lock=$2

    # Made where missing, and there regardless where another account made it first: its open to
    # create is then refused, in a sticky directory, though the file is there to take.
    if [ ! -e "$lock" ]; then
        (umask 022 && : > "$lock") 2> /dev/null || [ -e "$lock" ] || return 2
    fi
    case $fd in
        7) { exec 7< "$lock"; } 2> /dev/null || return 3 ;;
        9) { exec 9< "$lock"; } 2> /dev/null || return 3 ;;
        *) die "open_lock takes descriptor 7 or 9, not ${fd}" ;;
    esac
}

# Takes the rollout lock on file descriptor 9, held until the script exits; flock lets go when the
# process dies, however it dies, so nothing stale is left to clear by hand. Fails rather than waits:
# 1 where another holds it, 2 or 3 as open_lock.
take_rollout_lock() {
    open_lock 9 "$ROLLOUT_LOCK" || return
    flock -n 9
}

# What a script that could not take the rollout lock says, by why.
rollout_lock_refused() {
    case $1 in
        1) echo "another rollout holds ${ROLLOUT_LOCK}; wait for it, or run 'rollout.sh status'" ;;
        2) echo "cannot create ${ROLLOUT_LOCK}; /run/lock has to let every account make a file in it" ;;
        *) echo "cannot read ${ROLLOUT_LOCK}, which belongs to $(stat -c %U "$ROLLOUT_LOCK" 2> /dev/null || echo someone)" ;;
    esac
}

# Takes the API lock on file descriptor 7, held until the script exits, and fails as
# take_rollout_lock does.
#
# A caller that holds it hands it down by leaving descriptor 7 open on it, and this then takes that
# descriptor rather than open the file again: flock ties a lock to the open file, not to the process,
# so a second open would conflict with the caller's own lock. On the descriptor handed down, flock
# succeeds where the caller holds the lock, and takes it where nobody does.
take_api_lock() {
    [ /dev/fd/7 -ef "$API_LOCK" ] || open_lock 7 "$API_LOCK" || return
    flock -n 7
}

# What a script that could not take the API lock says, by why.
api_lock_refused() {
    case $1 in
        1) echo "a deploy, rollback, start or stop, or a change to the index this host serves, holds ${API_LOCK}; wait for it to finish" ;;
        2) echo "cannot create ${API_LOCK}; /run/lock has to let every account make a file in it" ;;
        *) echo "cannot read ${API_LOCK}, which belongs to $(stat -c %U "$API_LOCK" 2> /dev/null || echo someone)" ;;
    esac
}

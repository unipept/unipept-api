# shellcheck shell=bash
#
# The locks that keep the deploy scripts on one host from working on the same thing at once: for now
# the rollout's own, which rollout.sh holds for a run and loadbalancer/install.sh while it replaces
# the scripts a run loads. The API lock, which deploy.sh takes for a deploy, a rollback, a start and
# a stop, comes here with the deploy.sh status contract. unipept-database has a part of the same name,
# for its own locks. Needs nothing else. Sourced through .deploy/lib.sh.

# Where the rollout lock is, unless rollout.conf sets LOCK_FILE.
# shellcheck disable=SC2034  # read by the scripts that source this file.
readonly DEFAULT_ROLLOUT_LOCK=/tmp/unipept-rollout.lock

# Takes the rollout lock on file descriptor 9, held until the script exits; flock lets go when the
# process dies, however it dies, so nothing stale is left to clear by hand. Fails rather than waits:
# 1 where another holds it, 2 where the file cannot be made, 3 where it cannot be read.
#
# Opened for reading, which is all flock needs and is what makes the lock usable by both the
# operator and root. Opened for writing, a file root created is refused to the operator — and bash
# reports that itself and carries on with the descriptor unopened, so `flock` then failed on a bad
# descriptor and this said another rollout was holding a lock that nobody held.
take_rollout_lock() {
    local lock=$1

    if [ ! -e "$lock" ]; then
        { : > "$lock"; } 2> /dev/null || return 2
    fi
    { exec 9< "$lock"; } 2> /dev/null || return 3
    flock -n 9
}

# What a script that could not take the rollout lock says, by why.
rollout_lock_refused() {
    local status=$1 lock=$2

    case $status in
        1) echo "another rollout holds ${lock}; wait for it, or run 'rollout.sh status'" ;;
        2) echo "cannot create ${lock}; set LOCK_FILE in rollout.conf to a path this account can write" ;;
        *) echo "cannot read ${lock}, which belongs to $(stat -c %U "$lock" 2> /dev/null || echo someone)" ;;
    esac
}

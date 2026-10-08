# shellcheck shell=bash
#
# What a rollout in progress is doing, kept in RUN_STATE beside the rollout lock, and the three
# commands that act on a fleet outside a rollout: `status`, `abort` and `ready`.
#
# Uses log and die from core.sh, env_value from config.sh and ROLLOUT_LOCK from locks.sh;
# check_inventory_entries and read_inventory from checks.sh. From rollout.sh, which sources it:
# RUN_STATE, VERSION, STARTED_AT, RUN_BY, INVENTORY, ONLY, HERE, HAPROXY, on_server and return_to_pool. Beside
# haproxy.sh, in the checkout and on the load balancer.

# Makes the state file writable by this run before anything depends on it.
#
# Unlike the lock, this one is written, and the operator and root take it in turns: the operator
# cannot rewrite root's file at the default mode, and root cannot rewrite the operator's in sticky
# /run/lock at any mode. `note_phase` tolerates a failed write so a rollout is never lost to one,
# which is exactly why it has to be settled here instead — a silent failure there leaves `status`
# and `abort` reading a phase that has moved on.
prepare_run_state() {
    # Another account's file is replaced where this one may remove it, which root may: in sticky
    # /run/lock, root's write to the operator's file is refused even where its mode allows it, and
    # the failure would be silent. This run holds the lock, so no other run is using the file.
    if [ -e "$RUN_STATE" ] && [ ! -O "$RUN_STATE" ]; then
        rm -f "$RUN_STATE" 2>/dev/null || true
    fi
    if [ -e "$RUN_STATE" ] && [ ! -w "$RUN_STATE" ]; then
        # Naming the owner because they are the only one who can clear it: /run/lock is sticky, so this
        # account cannot remove a file it does not own however writable the directory looks.
        die "${RUN_STATE} belongs to $(stat -c %U "$RUN_STATE" 2>/dev/null || echo someone), who has to remove it"
    fi
    # 0666 on creation, because the next run is as likely to be the other account. The load balancer
    # carries operator logins only, and /run/lock is sticky, so nobody else can replace it.
    if [ ! -e "$RUN_STATE" ]; then
        (umask 0 && : > "$RUN_STATE") 2>/dev/null ||
            die "cannot create ${RUN_STATE}; /run/lock has to let every account make a file in it"
    fi
}

# Says what this run is doing, for `status` to read and `abort` to signal.
#
# Rewritten whole each time rather than appended to, so reading it never has to decide which of two
# phases is the current one.
note_phase() {
    local phase=$1 server=${2:-}

    {
        printf 'pid=%s\n' "$$"
        printf 'version=%s\n' "$VERSION"
        printf 'phase=%s\n' "$phase"
        printf 'server=%s\n' "$server"
        printf 'started=%s\n' "$STARTED_AT"
        printf 'by=%s\n' "$RUN_BY"
    } > "$RUN_STATE" 2>/dev/null || true
}

# Whether a rollout is running, decided by the lock rather than by the state file.
#
# A run killed uncatchably leaves its state file behind, and nothing in the file can say so. The
# lock cannot outlive the process that held it, so taking it is the test: if it can be taken, the
# file is leftovers.
a_run_is_in_progress() {
    # No lock file, no run — and said without creating one. `flock` would make it, and a file this
    # leaves behind as root is one the operator's next rollout has to work around.
    [ -e "$ROLLOUT_LOCK" ] || return 1

    # Opened for reading on a descriptor of its own for this one command, rather than by
    # `flock <file>`, which opens it to create it: in sticky /run/lock that is refused on a file
    # another account made, even to root. Not with `exec`, which would redirect this shell for good:
    # `exec 8< file 2>/dev/null` sends stderr to /dev/null permanently.
    #
    # Taking the lock and letting go is the whole test. A failure for any other reason reads as a
    # run in progress, which is the answer that refuses to act.
    ! { flock -n 8; } 2>/dev/null 8< "$ROLLOUT_LOCK"
}

# Reads the fleet without changing any of it. What to reach for after a run stopped part way, or
# when "which server is still out of the pool" needs an answer.
do_status() {
    local name host port backends server report inventory

    # Before the fleet, because it changes what the fleet below means: a server out of the pool is
    # expected while a rollout is working on it, and needs attention once nothing is.
    if a_run_is_in_progress; then
        if [ -f "$RUN_STATE" ]; then
            log "a rollout of $(env_value version "$RUN_STATE") is $(env_value phase "$RUN_STATE")$(
                s=$(env_value server "$RUN_STATE"); [ -n "$s" ] && printf ' %s' "$s")"
            log "started $(env_value started "$RUN_STATE") by $(env_value by "$RUN_STATE"), pid $(env_value pid "$RUN_STATE")"
            log "to stop it: ${HERE}/rollout.sh abort"
        else
            log "${ROLLOUT_LOCK} is held, but no state file says by what: a rollout starting, or loadbalancer/install.sh replacing these scripts"
        fi
    else
        # Leftovers from a run that was killed uncatchably. Said rather than deleted: it names what
        # was going on when the machine stopped, and the fleet below is what it left.
        [ -f "$RUN_STATE" ] && log "no rollout is running; ${RUN_STATE} is from one that did not finish"
        log "no rollout is running"
    fi
    printf '\n' >&2

    # Read whatever the inventory's problems, which are said rather than refused: this is what is
    # run to find out what happened.
    # shellcheck disable=SC2153  # rollout.sh's INVENTORY, not this function's inventory.
    check_inventory_entries "$INVENTORY" || true
    inventory=$(read_inventory "$INVENTORY" "$ONLY")

    printf '%-10s %-22s %-28s %-10s %-10s %s\n' SERVER ADDRESS HAPROXY VERSION VARIANT INDEX
    while read -r name host port backends server; do
        [ -n "$name" ] || continue
        report=$(on_server "$host" status 2>/dev/null || true)
        printf '%-10s %-22s %-28s %-10s %-10s %s\n' \
            "$name" "${host}:${port}" \
            "$("$HAPROXY" states "${backends}/${server}" 2>/dev/null || echo unreachable)" \
            "$(printf '%s\n' "$report" | env_value version)" \
            "$(printf '%s\n' "$report" | env_value variant)" \
            "$(index=$(printf '%s\n' "$report" | env_value index_version); printf '%s\n' "${index:--}")"
    done <<<"$inventory"
}

# Stops a rollout that is running, from anywhere.
#
# The run restores the server it drained through its own handlers, so this only has to reach them.
# Ctrl-C cannot, from another terminal: the signal has to go to that process, and the state file
# says which one it is.
do_abort() {
    local pid

    a_run_is_in_progress || die "no rollout is running; ${ROLLOUT_LOCK} is free"
    [ -f "$RUN_STATE" ] || die "a rollout holds ${ROLLOUT_LOCK} but wrote no ${RUN_STATE}; find it with 'ps'"

    pid=$(env_value pid "$RUN_STATE")
    case ${pid:-} in
        '' | *[!0-9]*) die "${RUN_STATE} names no pid to stop" ;;
    esac
    kill -0 "$pid" 2>/dev/null || die "pid ${pid} is not running, but the lock is held; find it with 'ps'"

    log "stopping the rollout of $(env_value version "$RUN_STATE") started by $(env_value by "$RUN_STATE")"
    kill -TERM "$pid" 2>/dev/null || die "could not signal ${pid}"

    # Its children too, and this is the part that makes the signal land. A shell runs a trap when
    # the command it is waiting on returns, and that command is an ssh running a deploy, which can
    # be an hour on a host that reads its index. Ending the connection is what Ctrl-C does by
    # signalling the whole foreground group: deploy.sh takes the HUP it is written for and rolls the
    # server back, ssh returns, and the run's own handler puts the server in the pool.
    local child
    for child in $(ps -o pid= --ppid "$pid" 2>/dev/null); do
        kill -TERM "$child" 2>/dev/null || true
    done

    # Until the lock is free, because that is when the run's own cleanup has finished. A server it
    # had drained is put back by its handlers, not by this.
    local waited=0
    while a_run_is_in_progress; do
        sleep 1
        waited=$((waited + 1))
        if [ "$waited" -ge 120 ]; then
            die "the rollout has not stopped after ${waited}s; check 'rollout.sh status'"
        fi
    done
    log "the rollout stopped. What it left is in 'rollout.sh status'"
}

# Returns named servers to the pool, or every server that can be.
#
# What the mail after a failure asks for. It goes through `return_to_pool`, the one way back that
# holds a server to answering both routes: put back without it, a server can take database traffic
# it cannot serve.
do_ready() {
    local wanted=("$@") inventory name host port backends server chosen=0 restored=0 refused=0

    # A line the inventory cannot be read from is said, and the servers it can be are put back.
    check_inventory_entries "$INVENTORY" || true
    inventory=$(read_inventory "$INVENTORY" "$ONLY")
    while read -r name host port backends server; do
        [ -n "$name" ] || continue

        if [ "${#wanted[@]}" -gt 0 ]; then
            case " ${wanted[*]} " in *" ${name} "*) ;; *) continue ;; esac
        fi
        chosen=$((chosen + 1))

        # Already serving traffic, so there is nothing to put back.
        case $("$HAPROXY" states "${backends}/${server}") in
            *MAINT* | *DRAIN*) ;;
            *) log "${name} is already in the pool"; continue ;;
        esac

        if return_to_pool "$name" "$host" "$port" "${backends}/${server}"; then
            restored=$((restored + 1))
        else
            refused=$((refused + 1))
        fi
    done <<<"$inventory"

    if [ "${#wanted[@]}" -gt 0 ] && [ "$chosen" -ne "${#wanted[@]}" ]; then
        die "the inventory does not name every one of: ${wanted[*]}"
    fi

    log "${restored} server(s) returned to the pool, ${refused} still out"
    # One left out is this command's "no", exit 1, rather than a failed test the error trap reports.
    [ "$refused" -eq 0 ] || exit 1
}

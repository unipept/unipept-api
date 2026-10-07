# shellcheck shell=bash
#
# What a rollout leaves behind for people: one journal line per server, and mail. Mailed from here
# rather than left to HAProxy's email-alert, which fires on every state change in every backend.
#
# Uses log from core.sh. From rollout.sh, which sources it: VERSION, COMMAND, HERE, REMOTE_DEPLOY,
# RUN_BY, RUN_FROM, NOTIFY_TO, NOTIFY_FROM and NOTIFY_SMTP, STATUS, and the lists this file's note_*
# functions keep. Beside haproxy.sh, in the checkout and on the load balancer.

# Sends one message to the team, through the MTA this host already runs for HAProxy's email-alert.
#
# curl rather than mail or sendmail: neither is installed on a stock Ubuntu 24.04, and curl is
# already required here. The hostname goes in the URL path so that EHLO does not announce a filename.
#
# Never fatal. A rollout that has just failed must not also fail at telling somebody.
notify() {
    local subject=$1 body=$2 message

    if [ -z "${NOTIFY_TO:-}" ]; then
        log "no NOTIFY_TO set, so nobody was emailed: ${subject}"
        return 0
    fi

    message=$(mktemp)
    {
        printf 'From: %s\n' "${NOTIFY_FROM:-unipept-rollout@$(hostname -f 2>/dev/null || hostname)}"
        printf 'To: %s\n' "$NOTIFY_TO"
        printf 'Subject: %s\n\n' "$subject"
        printf '%s\n' "$body"
    } > "$message"

    if curl -s --max-time 20 \
        --url "smtp://${NOTIFY_SMTP:-127.0.0.1:25}/$(hostname -f 2>/dev/null || hostname)" \
        --mail-from "${NOTIFY_FROM:-unipept-rollout@$(hostname -f 2>/dev/null || hostname)}" \
        --mail-rcpt "$NOTIFY_TO" --upload-file "$message"; then
        log "emailed ${NOTIFY_TO}: ${subject}"
    else
        log "could not email ${NOTIFY_TO}; the message was: ${subject}"
    fi
    rm -f "$message"
    return 0
}

# A server that took the release and went back into the pool.
#
# Mailed from here rather than left to HAProxy's email-alert, which fires on every state change in
# every backend: eighteen messages for a fleet of three, none of them about the release.
note_updated() {
    local name=$1

    UPDATED="${UPDATED}${name} "
    notify "[unipept-rollout] ${name} is on ${VERSION}" \
"${name} took ${VERSION#v} and is back in every backend it belongs to.

  $(printf '%s' "${STATUS[$name]:-unknown}" | tr '\n' ' ')

Run by ${RUN_BY} on $(hostname -f 2>/dev/null || hostname)."
}

# An update that failed but left the fleet serving. Worth an email, not an alarm.
#
# Two lists: one to read, one to iterate. Splitting "patty (serving 2.5.3)" on whitespace was logging
# a journal line that claimed the server was called "2.5.3)".
note_failed_update() {
    FAILED_UPDATE="${FAILED_UPDATE}${1} (serving ${2:-unknown}) "
    FAILED_NAMES="${FAILED_NAMES}${1} "
}

# A server nobody can route to. This is the case that has to reach a person.
note_down() {
    NEEDS_ATTENTION="${NEEDS_ATTENTION}${1} [${2}] "
    DOWN_NAMES="${DOWN_NAMES}${1} "
    # Left out of the pool, but said so. `finish` reports only what nothing else did.
    # shellcheck disable=SC2034  # rollout.sh's, which finish reads.
    CURRENT_TARGET=''
}

# The mail that closes a run, given its exit status: a server that needs a person first, then an
# update that was rolled back, then a rollout that went through. Nothing for a run that changed
# nothing.
notify_outcome() {
    local status=$1

    # Two texts for the one state, because a subcommand reaches it too. `ready` takes no --version
    # and attempts no fleet, so the rollout wording mailed "A rollout of  left a server" and sent
    # the operator looking for servers after it that were never part of the run. Keyed the way
    # record_run is keyed: a run that set out to change something has a VERSION, and nothing else
    # does.
    if [ -n "$NEEDS_ATTENTION" ] && [ -n "$VERSION" ]; then
        notify "[unipept-rollout] a server needs attention on $(hostname -s)" \
"A rollout of ${VERSION} left a server that cannot be routed to.

  ${NEEDS_ATTENTION}

The servers after it were not attempted, so the rest of the fleet is untouched.

To see the fleet:      ${HERE}/rollout.sh status
To return a server:    ${HERE}/loadbalancer/haproxy.sh ready <backends>/<server>
On the server itself:  ${REMOTE_DEPLOY} status

Run by ${RUN_BY} on $(hostname -f 2>/dev/null || hostname)."
    elif [ -n "$NEEDS_ATTENTION" ]; then
        notify "[unipept-rollout] a server needs attention on $(hostname -s)" \
"'rollout.sh ${COMMAND}' could not return a server to the pool.

  ${NEEDS_ATTENTION}

No rollout was running, so nothing else on the fleet was touched.

To see the fleet:      ${HERE}/rollout.sh status
To return a server:    ${HERE}/loadbalancer/haproxy.sh ready <backends>/<server>
On the server itself:  ${REMOTE_DEPLOY} status

Run by ${RUN_BY} on $(hostname -f 2>/dev/null || hostname)."
    elif [ -n "$FAILED_UPDATE" ]; then
        notify "[unipept-rollout] ${VERSION} was rolled back on $(hostname -s)" \
"A rollout of ${VERSION} stopped and the fleet is serving its previous version.

  ${FAILED_UPDATE}

Every server is in the pool. The servers after the failure were not attempted, so the fleet is
consistent only if this was the first one. Check with:

  ${HERE}/rollout.sh status

Run by ${RUN_BY} on $(hostname -f 2>/dev/null || hostname)."
    elif [ -n "$UPDATED" ] && [ "$status" -eq 0 ]; then
        # Keyed on a server having been updated, not on VERSION: --dry-run sets that too.
        notify "[unipept-rollout] ${VERSION} deployed on $(hostname -s)" \
"Every server this run set out to update is serving ${VERSION#v} and is back in rotation.

  ${UPDATED}

Run by ${RUN_BY} on $(hostname -f 2>/dev/null || hostname)."
    fi
}

# The lines of a server's `deploy.sh status` that the closing summary and the journal keep: what it
# runs and serves. Not INDEX_LOCATION, whose spaces would split a journal line, nor the lock's path or
# the format, which say nothing about the run.
recorded() {
    grep -E '^(version|previous|variant|port|active|index_version|opensearch_index)=' || true
}

# One journal line per server, so "who deployed what, when" has an answer that outlives a terminal.
#
# Only for a run that set out to change something. `status` and the recovery commands take no
# --version, so recording them wrote `version= ... exit=0` and read back as a rollout of nothing.
record_run() {
    local status=$1 name

    [ -n "$VERSION" ] || return 0

    for name in "${!STATUS[@]}"; do
        logger -t unipept-rollout -- \
            "version=${VERSION} server=${name} by=${RUN_BY} from=${RUN_FROM:-local} outcome=deployed $(printf '%s' "${STATUS[$name]}" | tr '\n' ' ')"
    done
    for name in $FAILED_NAMES; do
        logger -t unipept-rollout -- "version=${VERSION} server=${name} by=${RUN_BY} from=${RUN_FROM:-local} outcome=rolled-back"
    done
    for name in $DOWN_NAMES; do
        logger -t unipept-rollout -- "version=${VERSION} server=${name} by=${RUN_BY} from=${RUN_FROM:-local} outcome=needs-attention"
    done
    logger -t unipept-rollout -- "version=${VERSION} by=${RUN_BY} from=${RUN_FROM:-local} exit=${status}"
}

# shellcheck shell=bash
#
# Shared by the deploy scripts. Sourced, never run.

# The script's own PID, captured before any subshell can shadow it, so `die` can stop the run from
# inside one. Each script arms the trap that answers it.
readonly MAIN_PID=$$

readonly REPOSITORY=unipept/unipept-api

# Seconds to wait for a restarted server to answer /health. Generous because the preloaded and
# hybrid builds read the index into memory before they answer, and the index is hundreds of
# gigabytes. Both the server and the load balancer start from this.
# shellcheck disable=SC2034  # read by the scripts that source this file.
readonly DEFAULT_READY_TIMEOUT=900

# Seconds to wait for a draining server to finish. Above the API's own 150-second request timeout,
# so a server answers or gives up before this runs out.
# shellcheck disable=SC2034  # read by the scripts that source this file.
readonly DEFAULT_DRAIN_TIMEOUT=240

# The asset names release.yml publishes. Both the server and the load balancer ask for them, so the
# format lives here rather than being spelled out on each side.
asset_name() {
    local version=${1#v} variant=$2
    printf 'unipept-api-%s-x86_64-linux-gnu-%s\n' "$version" "$variant"
}

release_url() {
    local tag=$1 file=$2
    printf 'https://github.com/%s/releases/download/%s/%s\n' "$REPOSITORY" "$tag" "$file"
}

# Every path `start` in api/src/lib.rs opens, relative to INDEX_LOCATION. The service cannot come up
# without all of them, so a deploy that does not check them first trades a clear message for a
# timeout.
# shellcheck disable=SC2034  # read by the scripts that source this file.
readonly INDEX_FILES="
.version
sa.bin
proteins.bin
mapping.bin
kmer_table.bin
datastore/sampledata.json
datastore/ec_numbers.tsv
datastore/go_terms.tsv
datastore/interpro_entries.tsv
datastore/proteomes.tsv
datastore/lineages.tsv
datastore/taxons.tsv
"

# The index files each storage backend reads into memory, by variant. The choice is compiled in, so
# a host given the wrong build cannot correct it with a restart.
#
# mmap maps everything; preloaded holds all of it; hybrid maps only the suffix array, which is by far
# the largest part, and holds the rest.
files_resident_for() {
    case $1 in
        preloaded) printf 'sa.bin proteins.bin mapping.bin kmer_table.bin\n' ;;
        hybrid) printf 'proteins.bin mapping.bin kmer_table.bin\n' ;;
        mmap) printf '\n' ;;
        *) die "unknown variant '$1'; expected mmap, preloaded or hybrid" ;;
    esac
}

# A field from /proc/meminfo, in bytes. It reports kB.
meminfo() {
    local field=$1 value
    value=$(awk -v f="${field}:" '$1 == f { print $2 }' /proc/meminfo)
    [ -n "$value" ] || die "no ${field} in /proc/meminfo"
    printf '%s\n' $((value * 1024))
}

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

require_cmd() {
    local cmd
    for cmd in "$@"; do
        command -v "$cmd" >/dev/null || die "$cmd is not installed"
    done
}

# Whether one file matches the SHA256SUMS that lists it. Returns non-zero rather than exiting, so a
# caller that is collecting problems can carry on and report them all.
sha256_matches() {
    local file=$1 sums=$2
    local name expected actual

    name=$(basename "$file")
    [ -f "$sums" ] || { log "no checksums at $sums"; return 1; }

    expected=$(awk -v name="$name" '$2 == name || $2 == "*" name { print $1 }' "$sums")
    [ -n "$expected" ] || { log "$name is not listed in $sums"; return 1; }

    actual=$(sha256sum "$file" | cut -d' ' -f1)
    [ "$expected" = "$actual" ] || { log "$name does not match its checksum"; return 1; }

    return 0
}

# As `sha256_matches`, for a caller that has nothing to do but stop.
verify_sha256() {
    sha256_matches "$1" "$2" || die "refusing $(basename "$1")"
    log "$(basename "$1") matches its checksum"
}

# The status a URL answers with, or 000 when it does not answer at all.
http_code() {
    curl -s -o /dev/null -w '%{http_code}' --max-time 5 "$1"
}

# Polls until the URL answers 200. Returns 1 when the timeout runs out.
wait_for_http() {
    local url=$1 timeout=$2
    local deadline=$((SECONDS + timeout))

    while [ "$SECONDS" -lt "$deadline" ]; do
        if [ "$(http_code "$url")" = "200" ]; then
            return 0
        fi
        sleep 2
    done

    return 1
}

# Sends one message to the team, through the MTA this host already runs for HAProxy's email-alert.
#
# curl rather than mail or sendmail: neither is installed on a stock Ubuntu 22.04, and curl is
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

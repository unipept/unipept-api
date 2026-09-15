# shellcheck shell=bash
#
# Shared by the deploy scripts. Sourced, never run.

log() {
    printf '%s  %s\n' "$(date -u '+%H:%M:%S')" "$*" >&2
}

die() {
    log "error: $*"
    exit 1
}

require_cmd() {
    local cmd
    for cmd in "$@"; do
        command -v "$cmd" >/dev/null || die "$cmd is not installed"
    done
}

# Checks one file against the SHA256SUMS that lists it.
verify_sha256() {
    local file=$1 sums=$2
    local name expected actual

    name=$(basename "$file")
    [ -f "$sums" ] || die "no checksums at $sums"

    expected=$(awk -v name="$name" '$2 == name || $2 == "*" name { print $1 }' "$sums")
    [ -n "$expected" ] || die "$name is not listed in $sums"

    actual=$(sha256sum "$file" | cut -d' ' -f1)
    [ "$expected" = "$actual" ] || die "$name does not match its checksum"

    log "$name matches its checksum"
}

# Polls until the URL answers 200. Returns 1 when the timeout runs out.
wait_for_http() {
    local url=$1 timeout=$2
    local deadline=$((SECONDS + timeout))

    while [ "$SECONDS" -lt "$deadline" ]; do
        if [ "$(curl -s -o /dev/null -w '%{http_code}' --max-time 5 "$url")" = "200" ]; then
            return 0
        fi
        sleep 2
    done

    return 1
}

# Reads one key out of a systemd environment file.
env_value() {
    local key=$1 file=$2

    [ -f "$file" ] || die "no environment file at $file"
    sed -n "s/^${key}=//p" "$file" | tail -1
}

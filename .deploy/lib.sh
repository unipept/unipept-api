# shellcheck shell=bash
#
# Shared by the deploy scripts. Sourced, never run.

readonly REPOSITORY=unipept/unipept-api

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

# Reads one key out of `key=value` lines: a systemd environment file when given a path, otherwise
# standard input, which is the shape `deploy.sh status` prints for the rollout to read.
env_value() {
    local key=$1 file=${2:-}

    if [ -n "$file" ]; then
        [ -f "$file" ] || die "no environment file at $file"
        sed -n "s/^${key}=//p" "$file" | tail -1
    else
        sed -n "s/^${key}=//p" | tail -1
    fi
}

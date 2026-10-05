# shellcheck shell=bash
#
# A release of the API: where it is published, what its files are called, how it is downloaded, and
# whether a file matches its checksum. Uses log and die from core.sh. Sourced through .deploy/lib.sh.

readonly REPOSITORY=unipept/unipept-api

# What a release download may not do: wait forever.
#
# `--retry` acts on a failure that finished, so it does nothing for a transfer that connects and then
# goes quiet, which is what a half-open connection through a NAT looks like. A flat `--max-time` is
# the wrong bound the other way: the binary is large and a slow link is not a failure.
#
# So the bound is on throughput. Below 1 KiB/s for 30 seconds is a stall, curl reports it as the
# timeout it is, and `--retry` treats a timeout as transient and tries again. Three tries and it
# gives up, which is the behaviour every caller here already handles.
# shellcheck disable=SC2034  # read by the scripts that source this file.
readonly CURL_DOWNLOAD=(-fsSL --retry 3 --connect-timeout 20 --speed-limit 1024 --speed-time 30)

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

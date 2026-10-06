# shellcheck shell=bash
#
# What has to be true of a server before it is asked to run, install or start the API, one function
# per check. A check prints nothing when all is well; otherwise it prints each thing that is wrong
# as `check: …` and returns 1. A check either finds problems or warns, never both, so a caller can
# count the two apart. None of them exits or changes anything.
#
# Uses log and die from core.sh, env_value from config.sh, sha256_matches from release.sh and
# http_code from remote.sh, and SERVICE_USER, ROOT, BINARY and OPENSEARCH_TIMEOUT from deploy.sh,
# which sources it. Beside deploy.sh, in the checkout and on a server.

# Every path `start` in api/src/lib.rs needs, relative to INDEX_LOCATION. The service cannot come up
# without all of them, so a deploy that does not check them first trades a clear message for a
# timeout. The database build writes these files and checks the same list, so a change here needs
# the same change there.
readonly INDEX_FILES="
.version
sa.bin
proteins.bin
mapping.bin
datastore/sampledata.json
datastore/ec_numbers.tsv
datastore/go_terms.tsv
datastore/interpro_entries.tsv
datastore/proteomes.tsv
datastore/lineages.tsv
datastore/taxons.tsv
"

# Index files the service opens when they exist and runs without. Searches without them are slower.
readonly OPTIONAL_INDEX_FILES="
kmer_table.bin
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

# The bytes a variant holds resident for an index, or 0 for one that holds nothing, for the two
# memory checks.
resident_bytes() {
    local variant=$1 index=$2 needed=0 relative size

    for relative in $(files_resident_for "$variant"); do
        size=$(stat -c %s "${index}/${relative}" 2>/dev/null || echo 0)
        needed=$((needed + size))
    done
    printf '%s\n' "$needed"
}

# What a .version file holds, with the whitespace around it removed and none inside: the binary reads
# it the same way.
version_in() {
    local content
    content=$(<"$1")
    content="${content#"${content%%[![:space:]]*}"}"
    printf '%s\n' "${content%"${content##*[![:space:]]}"}"
}

# The OpenSearch index a version is served from: uniprot_entries-2026-03 for 2026.03. Fails for a
# version that makes no valid index name. The same rule as `index_name` in the binary's database
# crate, which refuses such a version at startup; change both together.
index_name() {
    case $1 in
        '' | [!0-9]* | *[!a-z0-9.-]*) return 1 ;;
    esac
    printf 'uniprot_entries-%s\n' "${1//./-}"
}

# The --uniprot-version to load a version with, for a release version, which is all load.sh takes.
load_hint() {
    [[ $1 =~ ^[0-9]{4}\.[0-9]{2}$ ]] && printf ' --uniprot-version %s' "${1//./-}"
    return 0
}

# The status OpenSearch answers a search of an index with, as /health/database searches it, so a
# closed index answers too: 000 when nothing answers. For check_opensearch_answers and
# check_opensearch_index, which judge the one answer.
search_status() {
    local status
    # curl exits non-zero when nothing listens, and prints 000, or nothing at all.
    status=$(http_code "${1%/}/${2}/_search?size=0&terminate_after=1" "$OPENSEARCH_TIMEOUT") || true
    printf '%s\n' "${status:-000}"
}

# Whether a variant is one the service is built in.
valid_variant() {
    case $1 in mmap | preloaded | hybrid) ;; *) return 1 ;; esac
}

# Every command an update reaches for, not only the three a deploy used to name. A missing `install`
# or `ln` would otherwise surface with the binary half replaced.
check_commands() {
    local cmd status=0

    for cmd in curl sha256sum install mktemp systemctl awk sed ln mv df; do
        command -v "$cmd" > /dev/null || { log "check: ${cmd} is not installed"; status=1; }
    done
    return "$status"
}

check_user() {
    [ "$(id -un)" = "$SERVICE_USER" ] || { log "check: running as $(id -un), not ${SERVICE_USER}"; return 1; }
}

# systemctl --user talks to the user manager through this, and a non-interactive ssh command does
# not always have it set. Lingering is what keeps it in place.
check_lingering() {
    local runtime="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}"
    [ -d "$runtime" ] || { log "check: no ${runtime}; enable lingering for ${SERVICE_USER}"; return 1; }
}

# The environment file, and the values in it the service cannot run without. INDEX_LOCATION may be
# missing when an index directory is given instead.
check_env_file() {
    local file=$1 index_given=$2 status=0 value

    [ -r "$file" ] || { log "check: cannot read ${file}"; return 1; }

    [ -n "$(env_value INDEX_LOCATION "$file")" ] || [ -n "$index_given" ] \
        || { log "check: INDEX_LOCATION is not set in ${file}"; status=1; }
    [ -n "$(env_value DATABASE_ADDRESS "$file")" ] || { log "check: DATABASE_ADDRESS is not set"; status=1; }

    value=$(env_value PORT "$file")
    case $value in
        '') log "check: PORT is not set"; status=1 ;;
        *[!0-9]*) log "check: PORT is '${value}', which is not a number"; status=1 ;;
    esac

    value=$(env_value VARIANT "$file")
    if [ -z "$value" ]; then
        log "check: VARIANT is not set; expected mmap, preloaded or hybrid"; status=1
    elif ! valid_variant "$value"; then
        log "check: VARIANT is '${value}'; expected mmap, preloaded or hybrid"; status=1
    fi

    # Optional, so only a value that is set and wrong is a problem. Checked because `ready_timeout`
    # falls back rather than refusing, and a host that silently kept the shared 900 is the failure
    # this whole setting exists to avoid.
    value=$(env_value READY_TIMEOUT "$file")
    case $value in
        *[!0-9]*) log "check: READY_TIMEOUT is '${value}', which is not a number of seconds"; status=1 ;;
    esac
    return "$status"
}

# The index, which is the setting most often wrong and the slowest to find out about: without these
# the service simply never answers and the deploy waits out its whole timeout.
check_index_dir() {
    [ -d "$1" ] && [ -r "$1" ] || { log "check: ${1} is not a readable directory"; return 1; }
}

# ProtectHome=yes hides /home from the unit, so an index there is readable now and gone the moment
# systemd starts the service.
check_index_not_home() {
    case $1 in
        /home/*) log "check: INDEX_LOCATION is under /home, which ProtectHome=yes hides from the service"; return 1 ;;
    esac
}

check_index_files() {
    local index=$1 relative status=0

    for relative in $INDEX_FILES; do
        [ -r "${index}/${relative}" ] || { log "check: ${index}/${relative} is missing or unreadable"; status=1; }
    done
    return "$status"
}

# A warning: the service runs without it.
check_index_optional_files() {
    local index=$1 relative status=0

    for relative in $OPTIONAL_INDEX_FILES; do
        [ -r "${index}/${relative}" ] || {
            log "check: ${index}/${relative} is missing or unreadable; the service runs without it, but searches are slower"
            status=1
        }
    done
    return "$status"
}

# The version an index's .version holds names an index the service can serve.
check_index_version() {
    local index=$1 version=$2
    index_name "$version" > /dev/null \
        || { log "check: ${index}/.version holds '${version}', which names no OpenSearch index; the service will not start"; return 1; }
}

# Given a variant and the bytes it holds resident. The backend is compiled in, so a host that cannot
# hold its variant cannot be corrected by a restart, only by deploying a different build. Above
# MemTotal is arithmetic: the variant cannot fit, ever.
check_memory_fits() {
    local variant=$1 needed=$2 total
    total=$(meminfo MemTotal)
    [ "$needed" -le "$total" ] \
        || { log "check: ${variant} needs $((needed / 1024 / 1024)) MiB resident, and this host has $((total / 1024 / 1024)) MiB in total"; return 1; }
}

# A warning: page cache is reclaimable, so more than is available now must not block a deploy. Only
# asked of a variant that fits at all, which check_memory_fits reports otherwise.
check_memory_free() {
    local variant=$1 needed=$2 available
    available=$(meminfo MemAvailable)
    [ "$needed" -le "$available" ] \
        || { log "check: ${variant} needs $((needed / 1024 / 1024)) MiB resident and $((available / 1024 / 1024)) MiB is available; the kernel has to reclaim first"; return 1; }
}

# A warning, given the status a search got: nothing answered. A binary can still be deployed during
# an outage; `start` refuses it.
check_opensearch_answers() {
    local database=$1 index=$2 status=$3
    [ "$status" != 000 ] \
        || { log "check: OpenSearch at ${database} does not answer, so whether ${index} is there is unknown"; return 1; }
}

# Given the status a search of the version's index got: the proteins are there, in an index that
# answers. 000, nothing answering, is check_opensearch_answers's to report.
check_opensearch_index() {
    local index=$1 version=$2 status=$3
    case $status in
        200 | 000) ;;
        404) log "check: ${index} is not in OpenSearch; load its proteins with unipept-database's load.sh$(load_hint "$version")"; return 1 ;;
        *) log "check: ${index} does not answer a search (HTTP ${status}); is it closed?"; return 1 ;;
    esac
}

# The service listens above 1024 and the load balancer reaches it on 80, so the redirect is as
# necessary as the binary. A host that lost it serves perfectly and is unreachable.
#
# That it works is only tested while the service answers on its own port: if it does not, nothing
# can be concluded about the redirect, and a deploy is exactly what someone runs to fix a service
# that is down.
check_ports_redirect() {
    local port=$1 status=0

    [ "$(systemctl is-enabled unipept-api-ports 2> /dev/null)" = enabled ] \
        || { log "check: unipept-api-ports is not enabled, so port 80 will not reach this service after a reboot"; status=1; }
    [ "$(systemctl is-active unipept-api-ports 2> /dev/null)" = active ] \
        || { log "check: unipept-api-ports is not active; run it as root to restore the port 80 redirect"; status=1; }
    if [ -n "$port" ] && [ "$(http_code "http://127.0.0.1:${port}/health")" = 200 ] \
        && [ "$(http_code "http://127.0.0.1:80/health")" != 200 ]; then
        log "check: the service answers on ${port} but port 80 does not reach it; check unipept-api-ports"
        status=1
    fi
    return "$status"
}

check_bin_writable() {
    [ -w "${ROOT}/bin" ] || { log "check: ${ROOT}/bin is not writable"; return 1; }
}

# Room for a second copy beside the one running, since both exist during a swap. Measured from the
# binary at hand, the one given or the one installed, with a little room to spare.
check_bin_room() {
    local from=$1 free_kb binary_kb=0
    free_kb=$(df -Pk "${ROOT}/bin" | awk 'NR == 2 { print $4 }')
    if [ -n "$from" ] && [ -f "$from" ]; then
        binary_kb=$(($(stat -c %s "$from") / 1024))
    elif [ -f "$BINARY" ]; then
        binary_kb=$(($(stat -c %s "$BINARY") / 1024))
    fi
    [ "${free_kb:-0}" -ge $((binary_kb + 20480)) ] \
        || { log "check: ${ROOT}/bin has $((${free_kb:-0} / 1024)) MiB free, and a swap needs about $(((binary_kb + 20480) / 1024)) MiB"; return 1; }
}

# A delivered binary: its checksum, and that it runs here. A build for the wrong architecture
# matches its checksum and still cannot execute, and finding that out during a rollout means a
# server already drained.
check_binary() {
    local from=$1 probe runs

    [ -f "$from" ] || { log "check: no binary at ${from}"; return 1; }
    sha256_matches "$from" "$(dirname "$from")/SHA256SUMS" || { log "check: ${from} does not match its checksum"; return 1; }

    # Beside the binary rather than in /tmp: /tmp is mounted noexec on a hardened host, and the probe
    # would then fail for every architecture, reporting a good build as unrunnable.
    probe="${ROOT}/bin/.probe.$$"
    install -m 0755 "$from" "$probe" \
        || { rm -f "$probe"; log "check: cannot place a copy of ${from} in ${ROOT}/bin to try it"; return 1; }
    runs=true
    "$probe" --version > /dev/null 2>&1 || runs=false
    rm -f "$probe"
    [ "$runs" = true ] || { log "check: ${from} does not run on this host; wrong architecture or a missing library"; return 1; }
}

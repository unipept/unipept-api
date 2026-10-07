#!/usr/bin/env bash
# install.sh + deploy.sh against a real systemd user manager. No sudo anywhere after install.
# shellcheck disable=SC2181,SC2012  # SC2181: these cases check the exit status of a command whose
# output went to a log file, which is then grepped, so `if ! cmd` cannot do both. SC2012: `ls | wc -l`
# over a known-simple path is clearer here than `find`.
set -uo pipefail

R=/deploy
# The container path; shellcheck is pointed at the checkout instead.
# shellcheck source=../lib.sh
source /deploy/tests/lib.sh

section "install.sh (the one root step)"
$R/server/install.sh >/tmp/install.log 2>&1; check "exit 0" "$?" "0"
check "linger on"      "$(loginctl show-user unipept -p Linger --value)" "yes"
check "bin dir owned"  "$(stat -c %U /opt/unipept-api/bin)" "unipept"
check "unit installed" "$([ -f /home/unipept/.config/systemd/user/unipept-api.service ] && echo yes)" "yes"
check "deploy.sh there" "$(stat -c '%U %a' /opt/unipept-api/lib/deploy.sh)" "unipept 755"
check "and the checks it makes" "$(stat -c '%U %a' /opt/unipept-api/lib/checks.sh)" "unipept 644"
check "and every part of lib.sh beside it" "$(ls /opt/unipept-api/lib/lib)" "$(ls /deploy/lib)"
check "all of them the service user's" \
    "$(stat -c '%U' /opt/unipept-api/lib/lib /opt/unipept-api/lib/lib.sh /opt/unipept-api/lib/lib/*.sh | sort -u)" "unipept"

# A fake index holding every file start() opens, since `check` now requires them and `deploy` runs
# `check` first. Created before the first deploy, not half way down.
mkdir -p /srv/index/datastore
: > /srv/index/sa.bin; : > /srv/index/proteins.bin; : > /srv/index/mapping.bin; : > /srv/index/kmer_table.bin
echo "2026.09-test" > /srv/index/.version
for f in sampledata.json ec_numbers.tsv go_terms.tsv interpro_entries.tsv proteomes.tsv lineages.tsv taxons.tsv; do : > "/srv/index/datastore/$f"; done
chmod -R a+rX /srv/index

# Point the env file at a test port, the fake index, and a variant.
sed -i 's#^PORT=.*#PORT=8099#; s#^VARIANT=.*#VARIANT=hybrid#; s#^INDEX_LOCATION=.*#INDEX_LOCATION=/srv/index#' /opt/unipept-api/etc/unipept-api.env
# install.sh applied the redirect against the example PORT; the suite just changed it.
systemctl restart unipept-api-ports >/dev/null 2>&1

# A fake OpenSearch at the example's DATABASE_ADDRESS. An index named in `indices` answers a search
# with the status in `status`, 200 unless a case says otherwise; any other index is 404, as a real
# cluster answers for one never loaded.
mkdir -p /srv/opensearch
echo uniprot_entries-2026-09-test > /srv/opensearch/indices
echo 200 > /srv/opensearch/status
cat > /usr/local/bin/fake-opensearch <<'EOF'
#!/usr/bin/env bash
read -r _ target _
while IFS= read -r line; do [ -z "${line%$'\r'}" ] && break; done
index=${target#/}; index=${index%%/*}; index=${index%%\?*}
status=404
grep -qx "$index" /srv/opensearch/indices && status=$(cat /srv/opensearch/status)
printf 'HTTP/1.1 %s X\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}' "$status"
EOF
chmod 755 /usr/local/bin/fake-opensearch
start_fake_opensearch() {
  socat TCP-LISTEN:9200,reuseaddr,fork SYSTEM:/usr/local/bin/fake-opensearch &
  FAKE_OPENSEARCH=$!
  local _
  for _ in $(seq 50); do
    [ "$(curl -s -o /dev/null -w '%{http_code}' http://localhost:9200/)" != 000 ] && return 0
    sleep 0.1
  done
}
start_fake_opensearch

UID_N=$(id -u unipept)
as_user() { setpriv --reuid unipept --regid unipept --init-groups env XDG_RUNTIME_DIR=/run/user/"$UID_N" HOME=/home/unipept bash -c "$1"; }

# A fake binary that serves /health, so a real user unit really runs and really answers.
stage() { # version healthy -> dir
  local d; d=$(mktemp -d); chmod 755 "$d"
  cat > "$d/unipept-api-$1-x86_64-linux-gnu-hybrid" <<EOF
#!/usr/bin/env bash
if [ "\${1:-}" = "--version" ]; then echo "unipept-api $1"; exit 0; fi
[ "$2" = yes ] || { sleep 600; exit 1; }
PORT=\$(sed -n 's/^PORT=//p' /opt/unipept-api/etc/unipept-api.env | tail -1)
# socat forks per connection, so two probes in a row cannot land in a gap where nothing is
# listening. A single-connection \`nc -l\` loop has exactly that gap between instances, and
# \`check\` probes the service twice — on its own port and then on 80 — so the gap made it
# intermittently report that port 80 does not reach the service.
printf 'HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok' > /tmp/response.http
exec socat TCP-LISTEN:\$PORT,reuseaddr,fork SYSTEM:'cat /tmp/response.http'
EOF
  sed -i "s/unipept-api \$1/unipept-api $1/" "$d/unipept-api-$1-x86_64-linux-gnu-hybrid"
  chmod 755 "$d/unipept-api-$1-x86_64-linux-gnu-hybrid"
  ( cd "$d" && sha256sum unipept-api-* > SHA256SUMS )
  echo "$d"
}

section "deploy as the unipept user, no sudo"
d1=$(stage 2.6.0 yes)
as_user "/opt/unipept-api/lib/deploy.sh deploy --from $d1/unipept-api-2.6.0-x86_64-linux-gnu-hybrid --timeout 30" >/tmp/d1.log 2>&1
check "exit 0" "$?" "0"
check "unit active"    "$(as_user 'systemctl --user is-active unipept-api')" "active"
check "health answers" "$(curl -s -o /dev/null -w '%{http_code}' http://127.0.0.1:8099/health)" "200"
check "version live"   "$(/opt/unipept-api/bin/unipept-api --version)" "unipept-api 2.6.0"

section "the journal has the unit's output"
check "journal readable as root" "$([ -n "$(journalctl -n 20 _SYSTEMD_USER_UNIT=unipept-api.service 2>/dev/null)" ] && echo yes)" "yes"

section "status, read the way the rollout reads it"
as_user "/opt/unipept-api/lib/deploy.sh status" > /tmp/st.txt 2>/dev/null
check "version line" "$(sed -n 's/^version=//p' /tmp/st.txt)" "2.6.0"
check "variant line" "$(sed -n 's/^variant=//p' /tmp/st.txt)" "hybrid"
check "active line"  "$(sed -n 's/^active=//p' /tmp/st.txt)" "active"
check "the format first" "$(head -1 /tmp/st.txt)" "status_format=1"
check "the index it serves" "$(sed -n 's/^index_location=//p' /tmp/st.txt)" "/srv/index"
check "that index's version" "$(sed -n 's/^index_version=//p' /tmp/st.txt)" "2026.09-test"
check "and its OpenSearch index" "$(sed -n 's/^opensearch_index=//p' /tmp/st.txt)" "uniprot_entries-2026-09-test"
check "the API lock" "$(sed -n 's/^api_lock=//p' /tmp/st.txt)" "/run/lock/unipept-api.lock"
check "the lock install.sh made, which any account can open" "$(stat -c '%a' /run/lock/unipept-api.lock)" "644"

section "upgrade keeps the old binary"
d2=$(stage 2.7.0 yes)
as_user "/opt/unipept-api/lib/deploy.sh deploy --from $d2/unipept-api-2.7.0-x86_64-linux-gnu-hybrid --timeout 30" >/tmp/d2.log 2>&1
check "exit 0" "$?" "0"
check "new version" "$(/opt/unipept-api/bin/unipept-api --version)" "unipept-api 2.7.0"
check "previous kept" "$(/opt/unipept-api/bin/unipept-api.previous --version)" "unipept-api 2.6.0"

section "an unhealthy deploy rolls itself back"
d3=$(stage 2.9.0 no)
as_user "/opt/unipept-api/lib/deploy.sh deploy --from $d3/unipept-api-2.9.0-x86_64-linux-gnu-hybrid --timeout 12" >/tmp/d3.log 2>&1
check "exit non-zero" "$([ $? -ne 0 ] && echo yes)" "yes"
check "back on the previous" "$(/opt/unipept-api/bin/unipept-api --version)" "unipept-api 2.7.0"
check "serving again" "$(curl -s -o /dev/null -w '%{http_code}' http://127.0.0.1:8099/health)" "200"

section "a corrupted binary is refused before anything is installed"
d4=$(stage 3.0.0 yes)
echo tampered >> "$d4/unipept-api-3.0.0-x86_64-linux-gnu-hybrid"
as_user "/opt/unipept-api/lib/deploy.sh deploy --from $d4/unipept-api-3.0.0-x86_64-linux-gnu-hybrid --timeout 20" >/tmp/d4.log 2>&1
check "exit non-zero" "$([ $? -ne 0 ] && echo yes)" "yes"
check "says checksum"  "$([ "$(grep -c 'checksum' /tmp/d4.log)" -ge 1 ] && echo yes)" "yes"
check "binary untouched" "$(/opt/unipept-api/bin/unipept-api --version)" "unipept-api 2.7.0"

section "rollback with nothing to go back to"
rm -f /opt/unipept-api/bin/unipept-api.previous
as_user "/opt/unipept-api/lib/deploy.sh rollback --timeout 5" >/tmp/d5.log 2>&1
check "exit non-zero" "$([ $? -ne 0 ] && echo yes)" "yes"
check "says no previous" "$(grep -c 'no previous binary' /tmp/d5.log)" "1"

section "deploy with neither --from nor --version"
as_user "/opt/unipept-api/lib/deploy.sh deploy" >/tmp/d6.log 2>&1
check "exit 2" "$?" "2"

section "root is refused, to keep the user manager the right one"
/opt/unipept-api/lib/deploy.sh status >/tmp/root.log 2>&1
check "exit non-zero" "$([ $? -ne 0 ] && echo yes)" "yes"
check "says run as unipept" "$(grep -c 'run this as unipept' /tmp/root.log)" "1"

section "the service user can run a remote command over ssh"
check "login shell is not nologin" "$(getent passwd unipept | cut -d: -f7)" "/bin/bash"

section "the environment file is not world readable"
check "mode 0600" "$(stat -c %a /opt/unipept-api/etc/unipept-api.env)" "600"

section "a flag without its value says usage"
as_user "/opt/unipept-api/lib/deploy.sh deploy --version" >/tmp/f1.log 2>&1
check "exit 2"      "$?" "2"
check "printed usage" "$(grep -c 'usage:' /tmp/f1.log)" "1"

section "status answers for a binary too old for --version"
printf '#!/bin/sh\nexit 2\n' > /opt/unipept-api/bin/unipept-api.old && chmod 755 /opt/unipept-api/bin/unipept-api.old
cp /opt/unipept-api/bin/unipept-api /tmp/keep-real
cp /opt/unipept-api/bin/unipept-api.old /opt/unipept-api/bin/unipept-api
as_user "/opt/unipept-api/lib/deploy.sh status" >/tmp/f2.log 2>&1
check "exit 0"          "$?" "0"
check "says unknown"    "$(sed -n 's/^version=//p' /tmp/f2.log)" "unknown"
check "still reports variant" "$(sed -n 's/^variant=//p' /tmp/f2.log)" "hybrid"
cp /tmp/keep-real /opt/unipept-api/bin/unipept-api

section "status names no index version where the index has none"
sed -i 's#^INDEX_LOCATION=.*#INDEX_LOCATION=/srv/nowhere#' /opt/unipept-api/etc/unipept-api.env
as_user "/opt/unipept-api/lib/deploy.sh status" >/tmp/f3.log 2>&1
check "exit 0" "$?" "0"
check "the index as configured" "$(sed -n 's/^index_location=//p' /tmp/f3.log)" "/srv/nowhere"
check "no version" "$(sed -n 's/^index_version=//p' /tmp/f3.log)" "-"
check "no OpenSearch index" "$(sed -n 's/^opensearch_index=//p' /tmp/f3.log)" "-"
mkdir -p /srv/nowhere && printf '2026.09\nversion=9.9.9\n' > /srv/nowhere/.version
as_user "/opt/unipept-api/lib/deploy.sh status" >/tmp/f4.log 2>&1
check "a .version of two lines: no version" "$(sed -n 's/^index_version=//p' /tmp/f4.log)" "-"
check "and adds no line" "$(grep -c '^version=' /tmp/f4.log)" "1"
rm -rf /srv/nowhere
sed -i 's#^INDEX_LOCATION=.*#INDEX_LOCATION=/srv/index#' /opt/unipept-api/etc/unipept-api.env

section "a restart that fails rolls back rather than aborting"
cp /opt/unipept-api/bin/unipept-api /opt/unipept-api/bin/unipept-api.previous
# A unit that cannot start: the binary exits at once, so restart fails.
# shellcheck disable=SC2016  # $1 belongs to the generated script, not to this one.
printf '#!/bin/sh\nif [ "$1" = --version ]; then echo "unipept-api 9.9.9"; exit 0; fi\nexit 3\n' > /tmp/broken
chmod 755 /tmp/broken
d9=$(mktemp -d); chmod 755 "$d9"; cp /tmp/broken "$d9/unipept-api-9.9.9-x86_64-linux-gnu-hybrid"
( cd "$d9" && sha256sum unipept-api-* > SHA256SUMS )
as_user "/opt/unipept-api/lib/deploy.sh deploy --from $d9/unipept-api-9.9.9-x86_64-linux-gnu-hybrid --timeout 12" >/tmp/f3.log 2>&1
check "exit non-zero"  "$([ $? -ne 0 ] && echo yes)" "yes"
check "rolled back"    "$([ "$(grep -c 'rolled back' /tmp/f3.log)" -ge 1 ] && echo yes)" "yes"
check "serving again"  "$(curl -s -o /dev/null -w '%{http_code}' http://127.0.0.1:8099/health)" "200"

section "a restart that returns non-zero rolls back, rather than aborting"
# systemctl shadowed so `restart` fails: this is the path where the binary is already swapped and
# set -e would otherwise abort with the service down and the good binary in .previous.
mkdir -p /tmp/fakebin
cat > /tmp/fakebin/systemctl <<'EOF'
#!/usr/bin/env bash
for a in "$@"; do [ "$a" = restart ] && exit 1; done
exec /usr/bin/systemctl "$@"
EOF
chmod +x /tmp/fakebin/systemctl
d10=$(mktemp -d); chmod 755 "$d10"
cp /tmp/keep-real "$d10/unipept-api-8.8.8-x86_64-linux-gnu-hybrid"
( cd "$d10" && sha256sum unipept-api-* > SHA256SUMS )
setpriv --reuid unipept --regid unipept --init-groups \
  env XDG_RUNTIME_DIR=/run/user/"$UID_N" HOME=/home/unipept PATH=/tmp/fakebin:/usr/bin:/bin \
  /opt/unipept-api/lib/deploy.sh deploy --from "$d10/unipept-api-8.8.8-x86_64-linux-gnu-hybrid" --timeout 8 >/tmp/f4.log 2>&1
check "exit non-zero"   "$([ $? -ne 0 ] && echo yes)" "yes"
check "rollback ran"    "$([ "$(grep -c 'rolling back' /tmp/f4.log)" -ge 1 ] && echo yes)" "yes"
check "old binary back" "$(/opt/unipept-api/bin/unipept-api --version)" "unipept-api 2.7.0"

section "check passes on a good host and reports the index version"
as_user "/opt/unipept-api/lib/deploy.sh check" >/tmp/c0.log 2>&1
check "exit 0"            "$?" "0"
check "index_version"     "$(sed -n 's/^index_version=//p' /tmp/c0.log)" "2026.09-test"
check "no problems"       "$(sed -n 's/^problems=//p' /tmp/c0.log)" "0"

section "each check on its own, on the good host and on a bad one"
# The installed checks.sh, as deploy.sh sources it: with lib.sh, and deploy.sh's settings, which a
# case may change before the check. As the service user unless the case is about the user. Exits
# with the check's own status.
one_check() {
  as_user "source /opt/unipept-api/lib/lib.sh
SERVICE_USER=unipept ROOT=/opt/unipept-api BINARY=/opt/unipept-api/bin/unipept-api OPENSEARCH_TIMEOUT=5
source /opt/unipept-api/lib/checks.sh
$1 || exit 1" >/tmp/one-check.log 2>&1
}
# Passes on the host case above found ready, and fails on a host or an input made bad for it, saying
# what is wrong.
both_ways() {
  local name=$1 good=$2 bad=$3 says=$4
  one_check "$good"; check "${name} passes" "$?" "0"
  one_check "$bad"; check "${name} fails" "$?" "1"
  check "and says so" "$(grep -c -- "$says" /tmp/one-check.log)" "1"
}
mkdir -p /tmp/bad-index && chmod 755 /tmp/bad-index
printf 'PORT=eighty\nVARIANT=mmap\n' > /tmp/bad.env && chmod 644 /tmp/bad.env
total=$(awk '$1 == "MemTotal:" { print $2 * 1024 }' /proc/meminfo)
available=$(awk '$1 == "MemAvailable:" { print $2 * 1024 }' /proc/meminfo)
# Larger than the room left beside the binary, as a sparse file, which takes none of it. Also used
# through `check` below.
truncate -s $((($(df -Pk /opt/unipept-api/bin | awk 'NR == 2 { print $4 }') + 1048576) * 1024)) /tmp/huge-binary
mkdir -p /tmp/locked/bin && chmod 755 /tmp/locked /tmp/locked/bin

both_ways check_commands check_commands "PATH=/nonexistent check_commands" "curl is not installed"
both_ways check_lingering check_lingering "XDG_RUNTIME_DIR=/nonexistent check_lingering" "enable lingering"
both_ways check_env_file "check_env_file /opt/unipept-api/etc/unipept-api.env ''" "check_env_file /tmp/bad.env ''" "PORT is 'eighty'"
both_ways check_index_dir "check_index_dir /srv/index" "check_index_dir /srv/no-index" "is not a readable directory"
both_ways check_index_not_home "check_index_not_home /srv/index" "check_index_not_home /home/unipept/index" "under /home"
both_ways check_index_files "check_index_files /srv/index" "check_index_files /tmp/bad-index" "sa.bin is missing or unreadable"
both_ways check_index_optional_files "check_index_optional_files /srv/index" "check_index_optional_files /tmp/bad-index" "searches are slower"
both_ways check_index_version "check_index_version /srv/index 2026.09-test" "check_index_version /srv/index 'not a version!'" "names no OpenSearch index"
both_ways check_memory_fits "check_memory_fits preloaded 1048576" "check_memory_fits preloaded $((total + 1048576))" "MiB in total"
both_ways check_memory_free "check_memory_free preloaded 1048576" "check_memory_free preloaded $(((available + total) / 2))" "MiB is available"
both_ways check_opensearch_answers \
  "check_opensearch_answers http://localhost:9200 uniprot_entries-2026-09-test \$(search_status http://localhost:9200 uniprot_entries-2026-09-test)" \
  "check_opensearch_answers http://localhost:1 uniprot_entries-2026-09-test \$(search_status http://localhost:1 uniprot_entries-2026-09-test)" \
  "does not answer"
both_ways check_opensearch_index \
  "check_opensearch_index uniprot_entries-2026-09-test 2026.09-test \$(search_status http://localhost:9200 uniprot_entries-2026-09-test)" \
  "check_opensearch_index uniprot_entries-2026-10-never 2026.10-never \$(search_status http://localhost:9200 uniprot_entries-2026-10-never)" \
  "is not in OpenSearch"
both_ways check_bin_writable check_bin_writable "ROOT=/tmp/locked check_bin_writable" "bin is not writable"
both_ways check_bin_room "check_bin_room ''" "BINARY=/tmp/huge-binary check_bin_room ''" "MiB free"
both_ways check_binary "check_binary $d1/unipept-api-2.6.0-x86_64-linux-gnu-hybrid" "check_binary /tmp/no-binary" "no binary at"
# Failing, it is the redirect cases further down.
one_check "check_ports_redirect 8099"; check "check_ports_redirect passes" "$?" "0"
# Run as root, which is the case it is about.
bash -c "source /opt/unipept-api/lib/lib.sh; SERVICE_USER=unipept; source /opt/unipept-api/lib/checks.sh; check_user || exit 1" >/tmp/one-check.log 2>&1
check "check_user fails for another user" "$?" "1"
check "and says so" "$(grep -c 'running as root, not unipept' /tmp/one-check.log)" "1"
one_check check_user; check "check_user passes for the service user" "$?" "0"
rm -rf /tmp/bad-index /tmp/locked /tmp/bad.env


section "deploy.sh check runs every one of them"
# Each check above tested on its own, here through `check`, so one left out of it fails a case.
check_says() {
  local name=$1 says=$2
  check "${name}" "$(grep -c -- "$says" /tmp/c-all.log)" "1"
}
mkdir -p /tmp/no-sha256sum
for tool in /usr/bin/* /bin/*; do ln -sf "$tool" /tmp/no-sha256sum/ 2>/dev/null; done
rm -f /tmp/no-sha256sum/sha256sum
setpriv --reuid unipept --regid unipept --init-groups env XDG_RUNTIME_DIR=/run/user/"$UID_N" HOME=/home/unipept \
  PATH=/tmp/no-sha256sum /opt/unipept-api/lib/deploy.sh check >/tmp/c-all.log 2>&1
check_says "a command missing" "sha256sum is not installed"
/opt/unipept-api/lib/deploy.sh check >/tmp/c-all.log 2>&1
check_says "another user" "running as root, not unipept"
setpriv --reuid unipept --regid unipept --init-groups env XDG_RUNTIME_DIR=/nonexistent HOME=/home/unipept \
  /opt/unipept-api/lib/deploy.sh check >/tmp/c-all.log 2>&1
check_says "no lingering" "no /nonexistent; enable lingering"
as_user "/opt/unipept-api/lib/deploy.sh check --index /srv/no-index" >/tmp/c-all.log 2>&1
check_says "an index that is not there" "/srv/no-index is not a readable directory"
mkdir -p /tmp/tight-index && cp -r /srv/index/. /tmp/tight-index/ && chmod -R a+rX /tmp/tight-index
truncate -s $(((available + total) / 2)) /tmp/tight-index/proteins.bin
as_user "/opt/unipept-api/lib/deploy.sh check --index /tmp/tight-index" >/tmp/c-all.log 2>&1
check_says "a variant that needs more than is free" "MiB is available"
truncate -s $((total + 1024 * 1024)) /tmp/tight-index/proteins.bin
as_user "/opt/unipept-api/lib/deploy.sh check --index /tmp/tight-index" >/tmp/c-all.log 2>&1
check_says "one that needs more than the host has" "MiB in total"
check "is that problem alone, not a warning as well" "$(grep -c 'MiB is available' /tmp/c-all.log)" "0"
chmod 555 /opt/unipept-api/bin
as_user "/opt/unipept-api/lib/deploy.sh check" >/tmp/c-all.log 2>&1
check_says "a binary directory it cannot write" "bin is not writable"
chmod 755 /opt/unipept-api/bin
chmod a+r /tmp/huge-binary
as_user "/opt/unipept-api/lib/deploy.sh check --from /tmp/huge-binary" >/tmp/c-all.log 2>&1
check_says "a binary with no room beside it" "MiB free"
rm -rf /tmp/no-sha256sum /tmp/tight-index /tmp/huge-binary


section "a deploy.sh copied without its checks says what to run"
mkdir -p /tmp/lone && cp /opt/unipept-api/lib/deploy.sh /opt/unipept-api/lib/lib.sh /tmp/lone/ && cp -r /opt/unipept-api/lib/lib /tmp/lone/
chmod -R a+rX /tmp/lone
as_user "/tmp/lone/deploy.sh status" >/tmp/lone.log 2>&1
check "it stops" "$?" "2"
check "and names install.sh" "$(grep -c 'there is no checks.sh beside /tmp/lone/deploy.sh; run server/install.sh again' /tmp/lone.log)" "1"
rm -rf /tmp/lone


section "a missing k-mer table is a warning"
mv /srv/index/kmer_table.bin /srv/kmer_table.bin.away
as_user "/opt/unipept-api/lib/deploy.sh check" >/tmp/c1k.log 2>&1
check "exit 0"          "$?" "0"
check "no problems"     "$(sed -n 's/^problems=//p' /tmp/c1k.log)" "0"
check "one more warning" "$(sed -n 's/^warnings=//p' /tmp/c1k.log)" "$(( $(sed -n 's/^warnings=//p' /tmp/c0.log) + 1 ))"
check "names the file"  "$(grep -c 'kmer_table.bin is missing' /tmp/c1k.log)" "1"
check "index_version"   "$(sed -n 's/^index_version=//p' /tmp/c1k.log)" "2026.09-test"
mv /srv/kmer_table.bin.away /srv/index/kmer_table.bin


section "check finds the OpenSearch index of the version it serves"
check "names it" "$(sed -n 's/^opensearch_index=//p' /tmp/c0.log)" "uniprot_entries-2026-09-test"

echo uniprot_entries-2026-08-test > /srv/opensearch/indices
as_user "/opt/unipept-api/lib/deploy.sh check" >/tmp/o1.log 2>&1
check "a version never loaded: non-zero" "$([ $? -ne 0 ] && echo yes)" "yes"
check "says to load it" "$(grep -c "uniprot_entries-2026-09-test is not in OpenSearch; load its proteins with unipept-database's load.sh$" /tmp/o1.log)" "1"
echo uniprot_entries-2026-09-test > /srv/opensearch/indices

# A closed index is in the cluster and still refuses every search.
echo 400 > /srv/opensearch/status
as_user "/opt/unipept-api/lib/deploy.sh check" >/tmp/o2.log 2>&1
check "a closed index: non-zero" "$([ $? -ne 0 ] && echo yes)" "yes"
check "asks whether it is closed" "$(grep -c 'does not answer a search (HTTP 400); is it closed' /tmp/o2.log)" "1"
echo 200 > /srv/opensearch/status

# An outage is /health/database's to report; a binary has to be deployable during one.
kill "$FAKE_OPENSEARCH"; wait "$FAKE_OPENSEARCH" 2>/dev/null
as_user "/opt/unipept-api/lib/deploy.sh check" >/tmp/o3.log 2>&1
check "OpenSearch down: exit 0" "$?" "0"
check "one more warning" "$(sed -n 's/^warnings=//p' /tmp/o3.log)" "$(( $(sed -n 's/^warnings=//p' /tmp/c0.log) + 1 ))"
check "says it does not answer" "$(grep -c 'OpenSearch at http://localhost:9200 does not answer' /tmp/o3.log)" "1"
start_fake_opensearch

# The process refuses to start on a version that names no index, so check says so first. OpenSearch
# refuses an index name with a capital in it.
echo "2026.09-Test" > /srv/index/.version
as_user "/opt/unipept-api/lib/deploy.sh check" >/tmp/o4.log 2>&1
check "a malformed .version: non-zero" "$([ $? -ne 0 ] && echo yes)" "yes"
check "says it names no index" "$(grep -c 'which names no OpenSearch index' /tmp/o4.log)" "1"

# Whitespace inside is not removed, as the binary does not remove it, and an empty file is no version.
printf '2026. 09-test\n' > /srv/index/.version
as_user "/opt/unipept-api/lib/deploy.sh check" >/tmp/o5.log 2>&1
check "whitespace inside: non-zero" "$([ $? -ne 0 ] && echo yes)" "yes"
check "says it names no index" "$(grep -c "holds '2026. 09-test', which names no OpenSearch index" /tmp/o5.log)" "1"
: > /srv/index/.version
as_user "/opt/unipept-api/lib/deploy.sh check" >/tmp/o6.log 2>&1
check "an empty .version: non-zero" "$([ $? -ne 0 ] && echo yes)" "yes"
check "says it names no index" "$(grep -c "holds '', which names no OpenSearch index" /tmp/o6.log)" "1"

# Whitespace around the version is not part of it.
printf '  2026.09-test \n\n' > /srv/index/.version
as_user "/opt/unipept-api/lib/deploy.sh check" >/tmp/o7.log 2>&1
check "whitespace around: exit 0" "$?" "0"
check "it is trimmed" "$(sed -n 's/^index_version=//p' /tmp/o7.log)" "2026.09-test"
echo "2026.09-test" > /srv/index/.version

# A release version gets the command that loads it; anything else, which load.sh refuses, does not.
cp -a /srv/index /srv/index-release
echo "2026.03" > /srv/index-release/.version
as_user "/opt/unipept-api/lib/deploy.sh check --index /srv/index-release" >/tmp/o8.log 2>&1
check "a release: the command" "$(grep -c "load.sh --uniprot-version 2026-03$" /tmp/o8.log)" "1"
rm -rf /srv/index-release

section "check --index checks a directory before anything points at it"
cp -a /srv/index /srv/index-next
echo "2026.10-test" > /srv/index-next/.version
as_user "/opt/unipept-api/lib/deploy.sh check --index /srv/index-next" >/tmp/n1.log 2>&1
check "its proteins not loaded: non-zero" "$([ $? -ne 0 ] && echo yes)" "yes"
check "names that index" "$(grep -c 'uniprot_entries-2026-10-test is not in OpenSearch' /tmp/n1.log)" "1"
echo uniprot_entries-2026-10-test >> /srv/opensearch/indices
as_user "/opt/unipept-api/lib/deploy.sh check --index /srv/index-next" >/tmp/n2.log 2>&1
check "loaded: exit 0" "$?" "0"
check "reports its version" "$(sed -n 's/^index_version=//p' /tmp/n2.log)" "2026.10-test"
check "and its index" "$(sed -n 's/^opensearch_index=//p' /tmp/n2.log)" "uniprot_entries-2026-10-test"
check "INDEX_LOCATION is untouched" "$(sed -n 's/^INDEX_LOCATION=//p' /opt/unipept-api/etc/unipept-api.env)" "/srv/index"
rm /srv/index-next/mapping.bin
as_user "/opt/unipept-api/lib/deploy.sh check --index /srv/index-next" >/tmp/n3.log 2>&1
check "a missing file there: non-zero" "$([ $? -ne 0 ] && echo yes)" "yes"
check "names the file there" "$(grep -c '/srv/index-next/mapping.bin is missing' /tmp/n3.log)" "1"
cp -a /srv/index/mapping.bin /srv/index-next/mapping.bin

section "stop, then start on what the environment file names"
as_user "/opt/unipept-api/lib/deploy.sh start --timeout 30" >/tmp/r0.log 2>&1
check "start while running: non-zero" "$([ $? -ne 0 ] && echo yes)" "yes"
check "says to stop it first" "$(grep -c 'is running; stop it first' /tmp/r0.log)" "1"

# Not running, rather than inactive: the stand-in binary exits non-zero on SIGTERM, so systemd calls
# it failed where the real one, which shuts down gracefully, is inactive.
as_user "/opt/unipept-api/lib/deploy.sh stop" >/tmp/r1s.log 2>&1
check "stop: exit 0" "$?" "0"
check "stopped" "$(as_user 'systemctl --user is-active unipept-api' | grep -qx active || echo yes)" "yes"

sed -i 's#^INDEX_LOCATION=.*#INDEX_LOCATION=/srv/index-next#' /opt/unipept-api/etc/unipept-api.env
as_user "/opt/unipept-api/lib/deploy.sh start --timeout 30" >/tmp/r1.log 2>&1
check "start: exit 0" "$?" "0"
check "says started" "$(grep -c 'started$' /tmp/r1.log)" "1"
check "active" "$(as_user 'systemctl --user is-active unipept-api')" "active"
check "health answers" "$(curl -s -o /dev/null -w '%{http_code}' http://127.0.0.1:8099/health)" "200"
check "binary untouched" "$(/opt/unipept-api/bin/unipept-api --version)" "unipept-api 2.7.0"

# Refused before the start, so nothing half-started is left behind.
as_user "/opt/unipept-api/lib/deploy.sh stop" >/dev/null 2>&1
sed -i '/uniprot_entries-2026-09-test/d' /srv/opensearch/indices
sed -i 's#^INDEX_LOCATION=.*#INDEX_LOCATION=/srv/index#' /opt/unipept-api/etc/unipept-api.env
as_user "/opt/unipept-api/lib/deploy.sh start --timeout 30" >/tmp/r2.log 2>&1
check "its proteins not loaded: non-zero" "$([ $? -ne 0 ] && echo yes)" "yes"
check "says the host is not ready" "$(grep -c 'this host is not ready' /tmp/r2.log)" "1"
check "not started" "$(as_user 'systemctl --user is-active unipept-api' | grep -qx active || echo yes)" "yes"
echo uniprot_entries-2026-09-test >> /srv/opensearch/indices

# The proteins have to answer: a start without them would serve /health and nothing else.
kill "$FAKE_OPENSEARCH"; wait "$FAKE_OPENSEARCH" 2>/dev/null
as_user "/opt/unipept-api/lib/deploy.sh start --timeout 30" >/tmp/r5.log 2>&1
check "OpenSearch down: non-zero" "$([ $? -ne 0 ] && echo yes)" "yes"
check "says to start it first" "$(grep -c 'OpenSearch does not answer; start it first' /tmp/r5.log)" "1"
check "still not started" "$(as_user 'systemctl --user is-active unipept-api' | grep -qx active || echo yes)" "yes"
start_fake_opensearch

as_user "/opt/unipept-api/lib/deploy.sh start --timeout nonsense" >/tmp/r3.log 2>&1
check "a timeout that is not seconds: non-zero" "$([ $? -ne 0 ] && echo yes)" "yes"
check "says so" "$(grep -c "takes seconds, not 'nonsense'" /tmp/r3.log)" "1"

# A unit that tripped systemd's start limit refuses a plain start; a deliberate one clears it.
for _ in 1 2 3 4 5 6 7; do as_user 'systemctl --user restart unipept-api' >/dev/null 2>&1; done
as_user 'systemctl --user stop unipept-api' >/dev/null 2>&1
check "the limit tripped" "$(as_user 'systemctl --user start unipept-api' >/dev/null 2>&1 || echo refused)" "refused"
as_user "/opt/unipept-api/lib/deploy.sh start --timeout 30" >/tmp/r4.log 2>&1
check "start clears it: exit 0" "$?" "0"
check "back on the first" "$(as_user 'systemctl --user is-active unipept-api')" "active"
rm -rf /srv/index-next

section "the API lock: what changes the service, or installs, is refused while another holds it"
LOCK=/run/lock/unipept-api.lock
# Held by another process, on a descriptor opened for reading as the scripts open it. `exec`, so the
# kill below ends the process that holds it.
( flock -n 7 && exec sleep 30 ) 7<"$LOCK" &
holder=$!
for _ in $(seq 50); do ( flock -n 7 ) 7<"$LOCK" || break; sleep 0.1; done
running=$(/opt/unipept-api/bin/unipept-api --version)
d_locked=$(stage 6.6.6 yes)
# Where the deploy that holds the lock stages its binary: nothing refused or merely reading clears it.
printf 'staged\n' > /opt/unipept-api/bin/unipept-api.new; chown unipept: /opt/unipept-api/bin/unipept-api.new
for command in "deploy --from $d_locked/unipept-api-6.6.6-x86_64-linux-gnu-hybrid --timeout 30" \
    "rollback --timeout 30" stop "start --timeout 30"; do
  as_user "/opt/unipept-api/lib/deploy.sh $command" >/tmp/l1.log 2>&1
  check "${command%% *}: exit 2" "$?" "2"
  check "${command%% *}: says what holds it" "$(grep -c "holds ${LOCK}; wait for it to finish" /tmp/l1.log)" "1"
done
check "binary untouched" "$(/opt/unipept-api/bin/unipept-api --version)" "$running"
check "still active" "$(as_user 'systemctl --user is-active unipept-api')" "active"
as_user "/opt/unipept-api/lib/deploy.sh check" >/dev/null 2>&1
check "check answers meanwhile" "$?" "0"
as_user "/opt/unipept-api/lib/deploy.sh status" >/tmp/l2.log 2>&1
check "status answers meanwhile" "$(sed -n 's/^active=//p' /tmp/l2.log)" "active"
check "the holder's staged binary left alone" "$(cat /opt/unipept-api/bin/unipept-api.new 2>/dev/null)" "staged"
rm -f /opt/unipept-api/bin/unipept-api.new
touch -d '2000-01-01' /opt/unipept-api/lib/deploy.sh
$R/server/install.sh >/tmp/l7.log 2>&1
check "install.sh: exit 2" "$?" "2"
check "install.sh: says to wait" "$(grep -c "holds ${LOCK}; wait for it to finish. Install once it has finished." /tmp/l7.log)" "1"
check "install.sh: deploy.sh not replaced" "$(stat -c %Y /opt/unipept-api/lib/deploy.sh)" "$(date -d '2000-01-01' +%s)"
kill "$holder" 2>/dev/null; wait "$holder" 2>/dev/null
$R/server/install.sh >/tmp/l8.log 2>&1
check "install.sh once it is free: exit 0" "$?" "0"
check "and replaces deploy.sh" "$([ "$(stat -c %Y /opt/unipept-api/lib/deploy.sh)" != "$(date -d '2000-01-01' +%s)" ] && echo yes)" "yes"

section "the API lock: a caller that holds it hands it down on descriptor 7"
as_user "exec 7<${LOCK} && flock -n 7 && /opt/unipept-api/lib/deploy.sh stop && /opt/unipept-api/lib/deploy.sh start --timeout 30" >/tmp/l3.log 2>&1
check "stop and start under it: exit 0" "$?" "0"
check "started" "$(grep -c 'started$' /tmp/l3.log)" "1"
check "active" "$(as_user 'systemctl --user is-active unipept-api')" "active"
# The caller's lock still keeps out a deploy.sh it did not hand it to.
as_user "exec 7<${LOCK} && flock -n 7 && /opt/unipept-api/lib/deploy.sh stop 7<&-" >/tmp/l4.log 2>&1
check "one not handed it: exit 2" "$?" "2"
check "is refused" "$(grep -c "holds ${LOCK}" /tmp/l4.log)" "1"
check "still active" "$(as_user 'systemctl --user is-active unipept-api')" "active"

section "the API lock: one that cannot be read or made is said so"
mv "$LOCK" /tmp/api-lock.keep
: > "$LOCK"; chmod 600 "$LOCK"
as_user "/opt/unipept-api/lib/deploy.sh stop" >/tmp/l5.log 2>&1
check "unreadable: exit 2" "$?" "2"
check "names its owner" "$(grep -c "cannot read ${LOCK}, which belongs to root" /tmp/l5.log)" "1"
rm -f "$LOCK"
chmod 1755 /run/lock
as_user "/opt/unipept-api/lib/deploy.sh stop" >/tmp/l6.log 2>&1
check "cannot be made: exit 2" "$?" "2"
check "says why" "$(grep -c "cannot create ${LOCK}" /tmp/l6.log)" "1"
chmod 1777 /run/lock
mv /tmp/api-lock.keep "$LOCK"
check "still active" "$(as_user 'systemctl --user is-active unipept-api')" "active"

section "each failure on its own"
# A missing index file.
mv /srv/index/mapping.bin /srv/mapping.bin.away
as_user "/opt/unipept-api/lib/deploy.sh check" >/tmp/c1.log 2>&1
check "missing file: a check's no" "$?" "1"
check_absent "and not an error"  'stopped:' /tmp/c1.log
check "names the file"         "$(grep -c 'mapping.bin is missing' /tmp/c1.log)" "1"
mv /srv/mapping.bin.away /srv/index/mapping.bin

# An index under /home, which ProtectHome hides.
sed -i 's#^INDEX_LOCATION=.*#INDEX_LOCATION=/home/unipept/index#' /opt/unipept-api/etc/unipept-api.env
mkdir -p /home/unipept/index && chown unipept:unipept /home/unipept/index
as_user "/opt/unipept-api/lib/deploy.sh check" >/tmp/c2.log 2>&1
check "under /home: non-zero" "$([ $? -ne 0 ] && echo yes)" "yes"
check "says ProtectHome"       "$(grep -c 'ProtectHome' /tmp/c2.log)" "1"
sed -i 's#^INDEX_LOCATION=.*#INDEX_LOCATION=/srv/index#' /opt/unipept-api/etc/unipept-api.env

# An unknown VARIANT.
sed -i 's#^VARIANT=.*#VARIANT=nonsense#' /opt/unipept-api/etc/unipept-api.env
as_user "/opt/unipept-api/lib/deploy.sh check" >/tmp/c3.log 2>&1
check "bad variant: non-zero" "$([ $? -ne 0 ] && echo yes)" "yes"
check "names the expected set" "$([ "$(grep -c 'expected mmap, preloaded or hybrid' /tmp/c3.log)" -ge 1 ] && echo yes)" "yes"

# An unset VARIANT.
sed -i 's#^VARIANT=.*#VARIANT=#' /opt/unipept-api/etc/unipept-api.env
as_user "/opt/unipept-api/lib/deploy.sh check" >/tmp/c4.log 2>&1
check "unset variant: non-zero" "$([ $? -ne 0 ] && echo yes)" "yes"
sed -i 's#^VARIANT=.*#VARIANT=hybrid#' /opt/unipept-api/etc/unipept-api.env

# A non-numeric PORT.
sed -i 's#^PORT=.*#PORT=eighty#' /opt/unipept-api/etc/unipept-api.env
as_user "/opt/unipept-api/lib/deploy.sh check" >/tmp/c5.log 2>&1
check "bad port: non-zero" "$([ $? -ne 0 ] && echo yes)" "yes"
check "says not a number"  "$(grep -c 'not a number' /tmp/c5.log)" "1"
sed -i 's#^PORT=.*#PORT=8099#' /opt/unipept-api/etc/unipept-api.env

# A command shadowed out of PATH.
mkdir -p /tmp/emptybin
setpriv --reuid unipept --regid unipept --init-groups env XDG_RUNTIME_DIR=/run/user/"$UID_N" HOME=/home/unipept PATH=/tmp/emptybin /opt/unipept-api/lib/deploy.sh check >/tmp/c6.log 2>&1
check "no commands: non-zero" "$([ $? -ne 0 ] && echo yes)" "yes"

section "the memory arm judges each variant on what it holds resident"
# A 'sa.bin' larger than RAM: fatal for preloaded, irrelevant for mmap, ignored by hybrid.
total_kb=$(awk '$1 == "MemTotal:" { print $2 }' /proc/meminfo)
truncate -s "$(( (total_kb + 1048576) * 1024 ))" /srv/index/sa.bin
sed -i 's#^VARIANT=.*#VARIANT=preloaded#' /opt/unipept-api/etc/unipept-api.env
as_user "/opt/unipept-api/lib/deploy.sh check" >/tmp/c7.log 2>&1
check "preloaded: non-zero"  "$([ $? -ne 0 ] && echo yes)" "yes"
check "says in total"        "$(grep -c 'in total' /tmp/c7.log)" "1"
sed -i 's#^VARIANT=.*#VARIANT=mmap#' /opt/unipept-api/etc/unipept-api.env
as_user "/opt/unipept-api/lib/deploy.sh check" >/tmp/c8.log 2>&1
check "mmap: exit 0"         "$?" "0"
sed -i 's#^VARIANT=.*#VARIANT=hybrid#' /opt/unipept-api/etc/unipept-api.env
as_user "/opt/unipept-api/lib/deploy.sh check" >/tmp/c9.log 2>&1
check "hybrid ignores sa.bin: exit 0" "$?" "0"
truncate -s 0 /srv/index/sa.bin

section "check --from validates the delivered binary"
d20=$(stage 4.0.0 yes)
as_user "/opt/unipept-api/lib/deploy.sh check --from $d20/unipept-api-4.0.0-x86_64-linux-gnu-hybrid" >/tmp/c10.log 2>&1
check "good binary: exit 0" "$?" "0"
echo tampered >> "$d20/unipept-api-4.0.0-x86_64-linux-gnu-hybrid"
as_user "/opt/unipept-api/lib/deploy.sh check --from $d20/unipept-api-4.0.0-x86_64-linux-gnu-hybrid" >/tmp/c11.log 2>&1
check "tampered: non-zero"  "$([ $? -ne 0 ] && echo yes)" "yes"
check "says checksum"       "$([ "$(grep -c 'checksum' /tmp/c11.log)" -ge 1 ] && echo yes)" "yes"
# A binary that cannot execute here at all.
d21=$(mktemp -d); chmod 755 "$d21"
printf '\x7fELF garbage not a real binary' > "$d21/unipept-api-4.1.0-x86_64-linux-gnu-hybrid"
( cd "$d21" && sha256sum unipept-api-* > SHA256SUMS )
as_user "/opt/unipept-api/lib/deploy.sh check --from $d21/unipept-api-4.1.0-x86_64-linux-gnu-hybrid" >/tmp/c12.log 2>&1
check "unrunnable: non-zero" "$([ $? -ne 0 ] && echo yes)" "yes"
check "says does not run"    "$(grep -c 'does not run on this host' /tmp/c12.log)" "1"

section "deploy refuses when check fails, before swapping anything"
running_before=$(/opt/unipept-api/bin/unipept-api --version)
sed -i 's#^VARIANT=.*#VARIANT=nonsense#' /opt/unipept-api/etc/unipept-api.env
d22=$(stage 4.2.0 yes)
as_user "/opt/unipept-api/lib/deploy.sh deploy --from $d22/unipept-api-4.2.0-x86_64-linux-gnu-hybrid --timeout 20" >/tmp/c13.log 2>&1
check "exit non-zero"      "$([ $? -ne 0 ] && echo yes)" "yes"
check "says not ready"     "$(grep -c 'not ready' /tmp/c13.log)" "1"
check "binary untouched"   "$(/opt/unipept-api/bin/unipept-api --version)" "$running_before"
sed -i 's#^VARIANT=.*#VARIANT=hybrid#' /opt/unipept-api/etc/unipept-api.env

section "--no-rollback leaves the decision to the caller"
d23=$(stage 4.3.0 no)
as_user "/opt/unipept-api/lib/deploy.sh deploy --from $d23/unipept-api-4.3.0-x86_64-linux-gnu-hybrid --timeout 10 --no-rollback" >/tmp/c14.log 2>&1
check "exit non-zero"        "$([ $? -ne 0 ] && echo yes)" "yes"
check "new binary still on"  "$(/opt/unipept-api/bin/unipept-api --version)" "unipept-api 4.3.0"
check_absent "did not roll back" 'rolled back' /tmp/c14.log
check "says caller decides"  "$(grep -c 'for the caller to decide' /tmp/c14.log)" "1"
check "no orphan .new"       "$([ -e /opt/unipept-api/bin/unipept-api.new ] && echo present || echo absent)" "absent"
# Put the host back for the cases after this.
as_user "/opt/unipept-api/lib/deploy.sh rollback --timeout 20" >/dev/null 2>&1

section "a failed rollback keeps .previous and the rejected binary"
# The scenario is both binaries being bad. .previous cannot simply be overwritten with a broken one,
# because install_binary derives it from whatever is currently installed — so the *installed* binary
# is the one that has to be unhealthy. mv, not cp, so the running service keeps its own inode.
d24=$(stage 6.0.0 no)
mv "$d24/unipept-api-6.0.0-x86_64-linux-gnu-hybrid" /tmp/broken-installed
chown unipept:unipept /tmp/broken-installed
as_user "mv /tmp/broken-installed /opt/unipept-api/bin/unipept-api"

d25=$(stage 6.1.0 no)
as_user "/opt/unipept-api/lib/deploy.sh deploy --from $d25/unipept-api-6.1.0-x86_64-linux-gnu-hybrid --timeout 8" >/tmp/c15.log 2>&1
check "exit non-zero"        "$([ $? -ne 0 ] && echo yes)" "yes"
check ".previous kept"       "$([ -f /opt/unipept-api/bin/unipept-api.previous ] && echo yes)" "yes"
check ".failed kept"         "$([ -f /opt/unipept-api/bin/unipept-api.failed ] && echo yes)" "yes"
check "says previous intact" "$(grep -c 'is intact' /tmp/c15.log)" "1"
rm -f /opt/unipept-api/bin/unipept-api.failed /opt/unipept-api/bin/unipept-api.previous

section "the swap never leaves the binary absent"
# install_binary is keep_copy then an atomic rename, so no instant has nothing at the binary path.
# The probe stops between the two steps; `mv BINARY .previous; mv .new BINARY` fails this.
cp /opt/unipept-api/bin/unipept-api /tmp/swapsrc
chmod a+rx /tmp/swapsrc
absent=0
for _ in $(seq 1 15); do
  as_user "bash /swap-probe.sh" >/dev/null 2>&1 &
  probe=$!
  sleep 1
  [ -x /opt/unipept-api/bin/unipept-api ] || absent=$((absent+1))
  wait $probe 2>/dev/null
done
check "binary present at every check" "$absent" "0"
check "and still runs"                "$([ -n "$(/opt/unipept-api/bin/unipept-api --version)" ] && echo yes)" "yes"
rm -f /opt/unipept-api/bin/unipept-api.previous

section "an interrupt after the swap rolls back"
# A binary that never answers /health, so the deploy is still inside its health wait when the signal
# lands. TERM there has to put the old binary back rather than walk away.
as_user "/opt/unipept-api/lib/deploy.sh deploy --from $d1/unipept-api-2.6.0-x86_64-linux-gnu-hybrid --timeout 30" >/dev/null 2>&1
before_interrupt=$(/opt/unipept-api/bin/unipept-api --version)
d30=$(stage 7.0.0 no)
as_user "/opt/unipept-api/lib/deploy.sh deploy --from $d30/unipept-api-7.0.0-x86_64-linux-gnu-hybrid --timeout 120" >/tmp/i1.log 2>&1 &
runner=$!
sleep 6
# The deploy is the child of `setpriv`; signal the process group so deploy.sh itself gets it.
pkill -TERM -f 'deploy.sh deploy --from' >/dev/null 2>&1
wait $runner 2>/dev/null
check "caught the signal"    "$(grep -c 'caught TERM' /tmp/i1.log)" "1"
check "rolled back"          "$(grep -c 'already swapped, rolling back' /tmp/i1.log)" "1"
check "old binary restored"  "$(/opt/unipept-api/bin/unipept-api --version)" "$before_interrupt"
check "serving again"        "$(curl -s -o /dev/null -w '%{http_code}' http://127.0.0.1:8099/health)" "200"
check "no orphan .new"       "$([ -e /opt/unipept-api/bin/unipept-api.new ] && echo present || echo absent)" "absent"

section "HUP behaves like TERM, which is what a dying ssh sends"
d31=$(stage 7.1.0 no)
as_user "/opt/unipept-api/lib/deploy.sh deploy --from $d31/unipept-api-7.1.0-x86_64-linux-gnu-hybrid --timeout 120" >/tmp/i2.log 2>&1 &
runner=$!
sleep 6
pkill -HUP -f 'deploy.sh deploy --from' >/dev/null 2>&1
wait $runner 2>/dev/null
check "caught HUP"          "$(grep -c 'caught HUP' /tmp/i2.log)" "1"
check "rolled back"         "$(grep -c 'rolling back' /tmp/i2.log)" "1"
check "serving again"       "$(curl -s -o /dev/null -w '%{http_code}' http://127.0.0.1:8099/health)" "200"

section "an interrupt before the swap changes nothing"
running=$(/opt/unipept-api/bin/unipept-api --version)
as_user "/opt/unipept-api/lib/deploy.sh deploy --version v9.9.9 --timeout 30" >/tmp/i3.log 2>&1 &
runner=$!
sleep 1
pkill -TERM -f 'deploy.sh deploy --version' >/dev/null 2>&1
wait $runner 2>/dev/null
check "binary untouched"  "$(/opt/unipept-api/bin/unipept-api --version)" "$running"
check_absent "did not roll back" 'rolling back' /tmp/i3.log
check "no orphan .new"    "$([ -e /opt/unipept-api/bin/unipept-api.new ] && echo present || echo absent)" "absent"

section "the port 80 redirect"
check "unit enabled"    "$(systemctl is-enabled unipept-api-ports 2>/dev/null)" "enabled"
check "unit active"     "$(systemctl is-active unipept-api-ports 2>/dev/null)" "active"
check "80 reaches the service" "$(curl -s -o /dev/null -w '%{http_code}' --max-time 3 http://127.0.0.1:80/health)" "200"
check "the service's own port still answers" "$(curl -s -o /dev/null -w '%{http_code}' --max-time 3 http://127.0.0.1:8099/health)" "200"

section "check notices when the redirect is gone"
systemctl stop unipept-api-ports >/dev/null 2>&1
iptables -t nat -F UNIPEPT_API 2>/dev/null
as_user "/opt/unipept-api/lib/deploy.sh check" >/tmp/p1.log 2>&1
check "exit non-zero"      "$([ $? -ne 0 ] && echo yes)" "yes"
check "says it is inactive" "$(grep -c 'unipept-api-ports is not active' /tmp/p1.log)" "1"
check "deploy refuses too"  "$(as_user "/opt/unipept-api/lib/deploy.sh deploy --from $d1/unipept-api-2.6.0-x86_64-linux-gnu-hybrid --timeout 10" >/dev/null 2>&1; [ $? -ne 0 ] && echo yes)" "yes"
systemctl start unipept-api-ports >/dev/null 2>&1
sleep 1
check "and passes once it is back" "$(as_user "/opt/unipept-api/lib/deploy.sh check" >/dev/null 2>&1; echo $?)" "0"

section "the rules are idempotent and survive a restart"
before=$(iptables -t nat -S UNIPEPT_API | grep -c REDIRECT)
systemctl restart unipept-api-ports >/dev/null 2>&1
systemctl restart unipept-api-ports >/dev/null 2>&1
check "still one redirect rule" "$(iptables -t nat -S UNIPEPT_API | grep -c REDIRECT)" "$before"
check "80 still reaches it"     "$(curl -s -o /dev/null -w '%{http_code}' --max-time 3 http://127.0.0.1:80/health)" "200"

section "the redirect survives being restarted faster than systemd's start limit"
# install.sh restarts this unit on every run, and the default limit refuses the sixth start within
# ten seconds. The refusal is silent in the unit's own output and leaves nothing answering on 80.
for _ in 1 2 3 4 5 6 7; do systemctl restart unipept-api-ports >/dev/null 2>&1; done
check "the unit is still active" "$(systemctl is-active unipept-api-ports)" "active"
check "80 still reaches it"      "$(curl -s -o /dev/null -w '%{http_code}' --max-time 3 http://127.0.0.1:80/health)" "200"

section "install.sh says what is still wrong"
# A first install, before anybody has edited the environment file: the check has to report the
# placeholder index rather than let the operator find out at the first deploy.
cp /opt/unipept-api/etc/unipept-api.env /tmp/env.good
sed -i 's#^INDEX_LOCATION=.*#INDEX_LOCATION=/mnt/nothing-here#' /opt/unipept-api/etc/unipept-api.env
$R/server/install.sh >/tmp/inst2.log 2>&1
check "still exits 0"        "$?" "0"
check "names the bad index"  "$([ "$(grep -c '/mnt/nothing-here' /tmp/inst2.log)" -ge 1 ] && echo yes)" "yes"
check "says what to do next" "$(grep -c 'what to fix before deploying' /tmp/inst2.log)" "1"
cp /tmp/env.good /opt/unipept-api/etc/unipept-api.env
systemctl restart unipept-api-ports >/dev/null 2>&1
$R/server/install.sh >/tmp/inst3.log 2>&1
check "and says so once fixed" "$(grep -c 'this host is ready' /tmp/inst3.log)" "1"

section "the redirect does not touch traffic leaving this host"
# The rule is jumped from OUTPUT as well as PREROUTING, so without --dst-type LOCAL it rewrites every
# outbound connection to port 80 anywhere: apt-get over http, or any plain-HTTP call this server
# makes, would be answered by the API instead of the host it asked for.
( socat TCP-LISTEN:8100,reuseaddr,fork SYSTEM:'printf "HTTP/1.1 200 OK\r\nContent-Length: 8\r\n\r\nELSEWHERE"' >/dev/null 2>&1 & )
sleep 1
# 127.0.0.2 is this host too, so it proves the LOCAL match still covers the loopback range.
check "a local address is redirected"   "$(curl -s --max-time 3 http://127.0.0.1:80/health)" "ok"
# A destination that is not this host must reach the destination, not the API.
ip addr add 10.99.99.99/32 dev lo 2>/dev/null
( socat TCP-LISTEN:80,reuseaddr,fork,bind=10.99.99.99 SYSTEM:'printf "HTTP/1.1 200 OK\r\nContent-Length: 8\r\n\r\nELSEWHERE"' >/dev/null 2>&1 & )
sleep 1
check "the rule names dst-type LOCAL" "$(iptables -t nat -S UNIPEPT_API | grep -c 'dst-type LOCAL')" "1"

section "re-running install.sh re-applies the rules"
# Type=oneshot with RemainAfterExit means `enable --now` starts nothing once the unit is active, so
# a PORT change needs an explicit restart or the redirect keeps pointing at the old port.
sed -i 's#^PORT=.*#PORT=8097#' /opt/unipept-api/etc/unipept-api.env
$R/server/install.sh >/tmp/reapply.log 2>&1
check "the redirect followed PORT" "$(iptables -t nat -S UNIPEPT_API | grep -c 'to-ports 8097')" "1"
sed -i 's#^PORT=.*#PORT=8099#' /opt/unipept-api/etc/unipept-api.env
$R/server/install.sh >/dev/null 2>&1
check "and followed it back"       "$(iptables -t nat -S UNIPEPT_API | grep -c 'to-ports 8099')" "1"

section "a bad --timeout is refused before anything is swapped"
running=$(/opt/unipept-api/bin/unipept-api --version)
d40=$(stage 8.0.0 yes)
as_user "/opt/unipept-api/lib/deploy.sh deploy --from $d40/unipept-api-8.0.0-x86_64-linux-gnu-hybrid --timeout 900s" >/tmp/t1.log 2>&1
check "exit non-zero"     "$([ $? -ne 0 ] && echo yes)" "yes"
check "says what it takes" "$(grep -c 'takes seconds' /tmp/t1.log)" "1"
check "binary untouched"   "$(/opt/unipept-api/bin/unipept-api --version)" "$running"

section "a release download is bounded by throughput, not left to hang"
# `--retry` acts on a failure that finished, so a transfer that connects and then goes quiet is not
# covered by it: without a throughput bound a stalled download waits forever, and on the server side
# that is a person's deploy sitting there with no output.
#
# This asserts the option set reaches the download. That a stall then aborts is curl's own behaviour,
# and exercising it here would cost the suite two minutes of real waiting: the bound is 30 seconds of
# silence and `--retry 3` spends it four times.
#
# The stand-in answers a release download with an empty file, so the deploy goes on to the second
# download, and then stops at the checksum. Anything else it fails, as an unreachable host would.
mkdir -p /tmp/curlbin
cat > /tmp/curlbin/curl <<'EOF'
#!/usr/bin/env bash
printf '%s\n' "$*" >> /tmp/curl-args.log
case "$*" in *releases/download*) ;; *) exit 1 ;; esac
while [ "$#" -gt 0 ]; do
  [ "$1" = -o ] && : > "$2"
  shift
done
exit 0
EOF
chmod +x /tmp/curlbin/curl
# Writable by the service user, which is who runs the deploy: root owns this file otherwise and the
# stand-in cannot append to it, so every assertion below would read an empty log and report that no
# download was attempted.
: > /tmp/curl-args.log
chmod 666 /tmp/curl-args.log
setpriv --reuid unipept --regid unipept --init-groups \
  env XDG_RUNTIME_DIR=/run/user/"$UID_N" HOME=/home/unipept PATH=/tmp/curlbin:/usr/bin:/bin \
  /opt/unipept-api/lib/deploy.sh deploy --version v9.9.9 --timeout 10 >/tmp/t7.log 2>&1
# Two downloads: the asset and its SHA256SUMS. `check`'s own health probe uses curl as well, so the
# lines are selected by the release URL rather than counted.
downloads=$(grep -c 'releases/download' /tmp/curl-args.log)
check "both files were fetched"   "$downloads" "2"
check "a connect timeout on each" "$(grep 'releases/download' /tmp/curl-args.log | grep -c -- '--connect-timeout 20')" "2"
check "a throughput floor"        "$(grep 'releases/download' /tmp/curl-args.log | grep -c -- '--speed-limit 1024')" "2"
check "and a window for it"       "$(grep 'releases/download' /tmp/curl-args.log | grep -c -- '--speed-time 30')" "2"
check "retries are still asked"   "$(grep 'releases/download' /tmp/curl-args.log | grep -c -- '--retry 3')" "2"
# The health probe must keep its own short bound rather than inherit the download's. Selected by its
# route, since check's search of the OpenSearch index shares that bound.
check "the probe is unchanged"    "$(grep -- '/health' /tmp/curl-args.log | grep -c -- '--max-time 5')" "1"

# A download that fails stops the deploy before the next one.
cat > /tmp/curlbin/curl <<'EOF'
#!/usr/bin/env bash
printf '%s\n' "$*" >> /tmp/curl-args.log
exit 22
EOF
: > /tmp/curl-args.log
setpriv --reuid unipept --regid unipept --init-groups \
  env XDG_RUNTIME_DIR=/run/user/"$UID_N" HOME=/home/unipept PATH=/tmp/curlbin:/usr/bin:/bin \
  /opt/unipept-api/lib/deploy.sh deploy --version v9.9.9 --timeout 10 >/tmp/t7b.log 2>&1
check "a failed download stops the deploy" "$?" "2"
check "before the next download"  "$(grep -c 'releases/download' /tmp/curl-args.log)" "1"
check "and says which command failed" "$(grep -c "failed with exit status 22" /tmp/t7b.log)" "1"
rm -rf /tmp/curlbin

section "READY_TIMEOUT belongs to the host"
# The fleet cannot share one deadline: the preloaded build on a slow disk needs far longer than the
# rest, and giving every host that number means a real failure anywhere takes as long to report.
as_user "/opt/unipept-api/lib/deploy.sh check" >/tmp/t2.log 2>&1
check "check reports a default"  "$(sed -n 's/^ready_timeout=//p' /tmp/t2.log)" "900"
printf 'READY_TIMEOUT=1800\n' >> /opt/unipept-api/etc/unipept-api.env
as_user "/opt/unipept-api/lib/deploy.sh check" >/tmp/t3.log 2>&1
check "and follows the file"     "$(sed -n 's/^ready_timeout=//p' /tmp/t3.log)" "1800"
# A value that is not seconds falls back rather than refusing, so `check` is what has to say so.
sed -i 's/^READY_TIMEOUT=.*/READY_TIMEOUT=twenty/' /opt/unipept-api/etc/unipept-api.env
as_user "/opt/unipept-api/lib/deploy.sh check" >/tmp/t4.log 2>&1
check "a bad value is a problem" "$([ $? -ne 0 ] && echo yes)" "yes"
check "and is named"             "$(grep -c "READY_TIMEOUT is 'twenty'" /tmp/t4.log)" "1"
check "the fallback still holds" "$(sed -n 's/^ready_timeout=//p' /tmp/t4.log)" "900"
sed -i '/^READY_TIMEOUT=/d' /opt/unipept-api/etc/unipept-api.env

section "a binary that exits at once is reported at once, not at the deadline"
# What a long READY_TIMEOUT costs if the wait only ever watches the port: the deadline that is right
# for a host loading an index is an hour of waiting for a binary that already gave up. The unit is
# the difference — Restart=on-failure replaces the process, so the pid it started on is gone.
d41=$(stage 8.1.0 yes)
as_user "/opt/unipept-api/lib/deploy.sh deploy --from $d41/unipept-api-8.1.0-x86_64-linux-gnu-hybrid --timeout 30" >/dev/null 2>&1
check "on a good binary first" "$(/opt/unipept-api/bin/unipept-api --version)" "unipept-api 8.1.0"

d42=$(mktemp -d); chmod 755 "$d42"
# shellcheck disable=SC2016  # $1 belongs to the generated script, not to this one.
printf '#!/bin/sh\nif [ "$1" = --version ]; then echo "unipept-api 8.2.0"; exit 0; fi\nexit 3\n' \
  > "$d42/unipept-api-8.2.0-x86_64-linux-gnu-hybrid"
chmod 755 "$d42/unipept-api-8.2.0-x86_64-linux-gnu-hybrid"
( cd "$d42" && sha256sum unipept-api-* > SHA256SUMS )

began=$SECONDS
as_user "/opt/unipept-api/lib/deploy.sh deploy --from $d42/unipept-api-8.2.0-x86_64-linux-gnu-hybrid --timeout 300" >/tmp/t5.log 2>&1
status=$?
elapsed=$((SECONDS - began))
check "exit non-zero"          "$([ "$status" -ne 0 ] && echo yes)" "yes"
check "says it is failing"     "$(grep -c 'failing, not loading' /tmp/t5.log)" "1"
check "well inside the 300s"   "$([ "$elapsed" -lt 120 ] && echo yes)" "yes"
check "rolled back"            "$([ "$(grep -c 'rolled back' /tmp/t5.log)" -ge 1 ] && echo yes)" "yes"
check "serving the old binary" "$(/opt/unipept-api/bin/unipept-api --version)" "unipept-api 8.1.0"

section "a binary that is still loading is given its whole deadline"
# The other half of the same watch, and the one that matters on a slow host: a service that is still
# reading its index keeps one pid and answers nothing for minutes. Only the deadline may end that
# wait, or the watch would cut short exactly the deploy it was added to protect.
d43=$(stage 8.3.0 no)
began=$SECONDS
as_user "/opt/unipept-api/lib/deploy.sh deploy --from $d43/unipept-api-8.3.0-x86_64-linux-gnu-hybrid --timeout 20" >/tmp/t6.log 2>&1
elapsed=$((SECONDS - began))
check "waited out the deadline" "$([ "$elapsed" -ge 20 ] && echo yes)" "yes"
check "and said so"             "$(grep -c 'did not answer /health within 20s' /tmp/t6.log)" "1"
check_absent "never called it failing" 'failing, not loading' /tmp/t6.log
check "rolled back"             "$([ "$(grep -c 'rolled back' /tmp/t6.log)" -ge 1 ] && echo yes)" "yes"
check "serving the old binary"  "$(/opt/unipept-api/bin/unipept-api --version)" "unipept-api 8.1.0"

section "install.sh is idempotent and keeps an edited env file"
$R/server/install.sh >/dev/null 2>&1; check "exit 0" "$?" "0"
check "PORT kept" "$(sed -n 's/^PORT=//p' /opt/unipept-api/etc/unipept-api.env)" "8099"

summary

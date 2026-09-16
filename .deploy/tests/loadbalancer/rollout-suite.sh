#!/usr/bin/env bash
# Drives rollout.sh against a real HAProxy and a fake ssh that stands in for the API servers.
# shellcheck disable=SC2181,SC2012  # SC2181: these cases check the exit status of a command whose
# output went to a log file, which is then grepped, so `if ! cmd` cannot do both. SC2012: `ls | wc -l`
# over a known-simple path is clearer here than `find`.
set -uo pipefail

mkdir -p /run/haproxy /work && cp -a /deploy/. /work/
R=/work/rollout.sh
export HAPROXY_SOCKET=/run/haproxy/haproxy.sock

printf 'HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok' > /response.http
for port in 9101 9102 9103; do
  # socat forks per connection, so two backends' checks and the polls never queue behind each
  # other. A single-connection nc drops checks and makes servers flap.
  # The response comes from a file: socat expands escapes in a SYSTEM argument before sh sees it.
  socat TCP-LISTEN:"$port",reuseaddr,fork SYSTEM:'cat /response.http' >/dev/null 2>&1 &
done
sleep 1
haproxy -f /etc/haproxy/haproxy.cfg -D
sleep 3

# The inventory points every server at localhost, on the ports the fake backends answer.
cat > /work/servers.conf <<EOF
rick   rick 9103 all_handlers,db_handlers rick
patty   patty 9101 all_handlers,db_handlers patty
selma   selma 9102 all_handlers,db_handlers selma
EOF

# A fake ssh: answers `status` the way deploy.sh does, and records what it was asked to do.
cat > /usr/local/bin/ssh <<'EOF'
#!/usr/bin/env bash
args=("$@"); cmd=""
for a in "${args[@]}"; do case $a in -o|BatchMode=yes) ;; *) cmd="$cmd $a" ;; esac; done
echo "SSH:$cmd" >> /tmp/ssh.log
case "$cmd" in
  *"deploy.sh check"*)
    [ -n "${FAKE_CHECK_FAILS:-}" ] && { echo "check: ${FAKE_CHECK_FAILS}" >&2; echo "problems=1"; exit 1; }
    printf 'variant=%s\nport=80\nindex_version=%s\nproblems=0\nwarnings=0\n' \
      "${FAKE_VARIANT:-hybrid}" "${FAKE_INDEX:-2026.09-test}" ;;
  *status*) printf 'version=%s\nprevious=2.5.3\nvariant=%s\nport=80\nactive=active\n' "${FAKE_VERSION:-2.6.0}" "${FAKE_VARIANT:-hybrid}" ;;
  *"deploy --from"*)
    [ -n "${FAKE_DEPLOY_BREAKS_HEALTH:-}" ] && touch /tmp/unhealthy
    [ -n "${FAKE_DEPLOY_FAILS:-}" ] && exit 1
    exit 0 ;;
  *rollback*)
    echo "ROLLBACK" >> /tmp/ssh.log
    [ -n "${FAKE_ROLLBACK_FAILS:-}" ] && exit 1
    rm -f /tmp/unhealthy   # putting the old binary back is what makes it serve again
    exit 0 ;;
  *) exit 0 ;;
esac
EOF
chmod +x /usr/local/bin/ssh
printf '#!/usr/bin/env bash\necho "SCP: $*" >> /tmp/scp.log\nexit 0\n' > /usr/local/bin/scp
chmod +x /usr/local/bin/scp

# A curl that fakes GitHub release downloads and passes everything else to the real one, so the
# health polls stay real. SHA256SUMS is written by the asset download, since rollout.sh asks for
# the sums first and verifies after.
cat > /usr/local/bin/curl <<'EOF'
#!/usr/bin/env bash
printf '%s\n' "$*" >> /tmp/curl-args.log
out=""; url=""; upload=""; args=("$@")
for ((i=0; i<${#args[@]}; i++)); do
  case ${args[i]} in
    -o) out=${args[i+1]} ;;
    --upload-file) upload=${args[i+1]} ;;
    https://github.com/*) url=${args[i]} ;;
  esac
done
if [ -n "$upload" ]; then cat "$upload" >> /tmp/mail.txt; exit 0; fi
# A server the deploy broke: healthy during preflight, not afterwards.
for a in "$@"; do
  case $a in
    *"/health/database"*)
      if [ -f /tmp/no-database ] && [[ $a == *"$(cat /tmp/no-database)"* ]]; then echo -n 503; exit 0; fi
      # "<host> <epoch>": down until that second, the way a restarted OpenSearch is. What tells a
      # server that is still coming back from one that is broken is only how long it takes.
      if [ -f /tmp/no-database-until ]; then
        read -r who until_when < /tmp/no-database-until
        if [[ $a == *"$who"* ]] && [ "$(date +%s)" -lt "$until_when" ]; then echo -n 503; exit 0; fi
      fi ;;
  esac
done
for a in "$@"; do case $a in *"/health"*) [ -f /tmp/unhealthy ] && { echo -n 503; exit 0; } ;; esac; done
if [ -n "$url" ] && [ -n "$out" ]; then
  directory=$(dirname "$out"); name=$(basename "$out")
  if [ "$name" = "SHA256SUMS" ]; then
    : > "$out"
  else
    printf 'fake binary for %s\n' "$name" > "$out"
    ( cd "$directory" && sha256sum "$name" >> SHA256SUMS )
  fi
  exit 0
fi
exec /usr/bin/curl "$@"
EOF
chmod +x /usr/local/bin/curl
: > /tmp/mail.txt
: > /tmp/curl-args.log

export PATH=/usr/local/bin:$PATH

cat > /work/rollout.conf <<EOF
HAPROXY_SOCKET=/run/haproxy/haproxy.sock
SSH_USER=unipept
NOTIFY_TO=unipept@example.invalid
LOCK_FILE=/tmp/unipept-rollout.lock
EOF

printf '127.0.0.1 patty selma rick\n' >> /etc/hosts

H=/work/loadbalancer/haproxy.sh

# Puts every server back in every backend and waits for HAProxy to agree, so each test starts from a
# fleet that is up. Without this, one test's leftover MAINT fails the next test's preflight.
reset_fleet() {
  rm -f /tmp/unhealthy
  for srv in patty selma rick; do $H ready "all_handlers,db_handlers/$srv" >/dev/null 2>&1; done
  for srv in patty selma rick; do $H wait-up "all_handlers,db_handlers/$srv" 30 >/dev/null 2>&1; done
  : > /tmp/ssh.log; : > /tmp/mail.txt
}

# Starts or stops the fake backend for one port, so a server can be made unhealthy on demand.
stop_backend() { pkill -f "TCP-LISTEN:$1" >/dev/null 2>&1; sleep 1; }
start_backend() { socat TCP-LISTEN:"$1",reuseaddr,fork SYSTEM:'cat /response.http' >/dev/null 2>&1 & sleep 1; }

# The container path; shellcheck is pointed at the checkout instead.
# shellcheck source-path=SCRIPTDIR source=../lib.sh
source /deploy/tests/lib.sh

echo "== 1. --dry-run reads every server and changes nothing =="
$R --version v2.6.0 --dry-run > /tmp/dry.txt 2>&1; check "exit 0" "$?" "0"
check "lists three servers" "$(grep -c 'server=' /tmp/dry.txt)" "3"
check "a primary comes first" "$(grep -oE '^(rick|patty|selma)' /tmp/dry.txt | head -1)" "patty"
check "the backup comes last" "$(grep -oE '^(rick|patty|selma)' /tmp/dry.txt | tail -1)" "rick"
check "reads the variant"   "$(grep -c 'variant=hybrid' /tmp/dry.txt)" "3"
check "haproxy untouched"   "$(/work/loadbalancer/haproxy.sh up-count all_handlers)" "3"

echo "== 2. --only selects one server =="
$R --version v2.6.0 --only patty --dry-run > /tmp/only.txt 2>&1
check "one line"  "$(grep -c 'server=' /tmp/only.txt)" "1"
check "it's patty" "$(grep -c 'patty' /tmp/only.txt)" "1"

echo "== 3. an unknown --only name is refused, not a silent no-op =="
$R --version v2.6.0 --only nosuch --dry-run > /tmp/no.txt 2>&1
check "exit non-zero" "$([ $? -ne 0 ] && echo yes)" "yes"
check "says so" "$(grep -c 'selects no server' /tmp/no.txt)" "1"

echo "== 4. no --version is a usage error =="
$R --dry-run > /tmp/usage.txt 2>&1; check "exit 2" "$?" "2"
$R --version > /tmp/usage2.txt 2>&1
check "valueless --version exits 2" "$?" "2"
check "and says usage" "$(grep -c 'usage:' /tmp/usage2.txt)" "1"

echo "== 4b. every server is reached, not just the first =="
# ssh reads stdin; without -n the first call would swallow the rest of the inventory.
: > /tmp/ssh.log
$R --version v2.6.0 --dry-run >/tmp/dry3.txt 2>&1
check "all three queried" "$(grep -c 'status' /tmp/ssh.log)" "3"

echo "== 4c. a malformed inventory line stops the run =="
cp /work/servers.conf /tmp/servers.keep
printf 'rick   rick 9103 all_handlers,db_handlers rick\nbroken 127.0.0.1\n' > /work/servers.conf
$R --version v2.6.0 >/tmp/bad.txt 2>&1
check "exit non-zero"  "$([ $? -ne 0 ] && echo yes)" "yes"
check "says too few"   "$(grep -c 'too few fields' /tmp/bad.txt)" "1"
check "nothing drained" "$(/work/loadbalancer/haproxy.sh state all_handlers/rick | cut -d' ' -f1)" "UP"
cp /tmp/servers.keep /work/servers.conf

echo "== 5. preflight refuses a fleet that is already down =="
/work/loadbalancer/haproxy.sh maint all_handlers/selma >/dev/null 2>&1
$R --version v2.6.0 > /tmp/pre.txt 2>&1
check "exit non-zero" "$([ $? -ne 0 ] && echo yes)" "yes"
check "names the problem" "$([ "$(grep -c 'preflight' /tmp/pre.txt)" -ge 2 ] && echo yes)" "yes"
check "nothing was drained" "$(/work/loadbalancer/haproxy.sh state all_handlers/patty)" "UP"
/work/loadbalancer/haproxy.sh ready all_handlers/selma >/dev/null 2>&1
sleep 5

echo "== 6. the downtime guard refuses to empty the backend =="
/work/loadbalancer/haproxy.sh maint all_handlers/selma >/dev/null 2>&1
/work/loadbalancer/haproxy.sh maint all_handlers/rick >/dev/null 2>&1
$R --version v2.6.0 --only patty > /tmp/guard.txt 2>&1
check "exit non-zero" "$([ $? -ne 0 ] && echo yes)" "yes"
check "says outage"    "$(grep -c 'is an outage' /tmp/guard.txt)" "1"
check "patty still UP" "$(/work/loadbalancer/haproxy.sh state all_handlers/patty | cut -d' ' -f1)" "UP"

echo "== 7. --allow-downtime overrides it, and the sequence is drain then maint then ready =="
: > /tmp/ssh.log
$R --version v2.6.0 --only patty --allow-downtime > /tmp/allow.txt 2>&1
check "exit 0" "$?" "0"
check "deploy was called"   "$(grep -c 'deploy --from' /tmp/ssh.log)" "1"
check "no sudo anywhere"    "$(grep -c 'sudo' /tmp/ssh.log)" "0"
check "status was read"     "$([ "$(grep -c 'status' /tmp/ssh.log)" -ge 2 ] && echo yes)" "yes"
check "binary was copied"   "$([ -s /tmp/scp.log ] && echo yes)" "yes"
check "patty back UP"       "$(/work/loadbalancer/haproxy.sh state all_handlers/patty | cut -d' ' -f1)" "UP"
check "patty back UP in db" "$(/work/loadbalancer/haproxy.sh state db_handlers/patty | cut -d' ' -f1)" "UP"
check "no rollback ran"     "$(grep -c '^ROLLBACK' /tmp/ssh.log)" "0"

echo "== 8a. a deploy that installed nothing returns the server to the pool and stops =="
reset_fleet
FAKE_DEPLOY_FAILS=1 FAKE_VERSION=2.5.3 $R --version v2.6.0 --only patty --allow-downtime >/tmp/r8a.txt 2>&1
check "exit non-zero"        "$([ $? -ne 0 ] && echo yes)" "yes"
check "says nothing installed" "$(grep -c 'nothing was installed' /tmp/r8a.txt)" "1"
check "no rollback attempted"  "$(grep -c '^ROLLBACK' /tmp/ssh.log)" "0"
check "back in the pool"       "$(printf '%s' "$($H state all_handlers/patty)" | cut -d' ' -f1)" "UP"
check "stopped the run"        "$(grep -c 'stopped at patty' /tmp/r8a.txt)" "1"
check "mailed a failed update" "$(grep -c 'was rolled back' /tmp/mail.txt)" "1"

echo "== 8b. installed but unhealthy: rolled back, then returned to the pool =="
reset_fleet
# The deploy lands and reports the new version, then the server stops answering: exactly the case the
# rollback exists for. Preflight saw it healthy, so the failure is discovered where it should be.
FAKE_DEPLOY_BREAKS_HEALTH=1 FAKE_VERSION=2.6.0 $R --version v2.6.0 --only patty --allow-downtime >/tmp/r8b.txt 2>&1
check "exit non-zero"        "$([ $? -ne 0 ] && echo yes)" "yes"
check "rolled back once"     "$(grep -c '^ROLLBACK' /tmp/ssh.log)" "1"
check "says rolling back"    "$(grep -c 'rolling it back' /tmp/r8b.txt)" "1"
check "serving again"        "$(grep -c 'rolled back and is serving again' /tmp/r8b.txt)" "1"
check "back in the pool"     "$(printf '%s' "$($H state all_handlers/patty)" | cut -d' ' -f1)" "UP"
check "back in the db pool"  "$(printf '%s' "$($H state db_handlers/patty)" | cut -d' ' -f1)" "UP"
check "stopped the run"      "$(grep -c 'stopped at patty' /tmp/r8b.txt)" "1"
check "mailed a failed update" "$(grep -c 'was rolled back' /tmp/mail.txt)" "1"

echo "== 8c. a rollback that also fails leaves the server out and mails urgently =="
reset_fleet
FAKE_DEPLOY_BREAKS_HEALTH=1 FAKE_VERSION=2.6.0 FAKE_ROLLBACK_FAILS=1 \
  $R --version v2.6.0 --only selma --allow-downtime >/tmp/r8c.txt 2>&1
check "exit non-zero"        "$([ $? -ne 0 ] && echo yes)" "yes"
check "could not roll back"  "$(grep -c 'could not be rolled back' /tmp/r8c.txt)" "1"
check "left out of the pool" "$($H state all_handlers/selma)" "MAINT"
rm -f /tmp/unhealthy
check "mailed urgently"      "$(grep -c 'needs attention' /tmp/mail.txt)" "1"
check "names the command"    "$(grep -c 'haproxy.sh ready' /tmp/mail.txt)" "1"

echo "== 8d. a deploy that worked while the connection died is not undone =="
reset_fleet
# The deploy call fails, but the server is serving the new version and is healthy: only ssh broke.
FAKE_DEPLOY_FAILS=1 FAKE_VERSION=2.6.0 $R --version v2.6.0 --only rick --allow-downtime >/tmp/r8d.txt 2>&1
check "exit 0"              "$?" "0"
check "says after all"      "$(grep -c 'after all' /tmp/r8d.txt)" "1"
check "no rollback"         "$(grep -c '^ROLLBACK' /tmp/ssh.log)" "0"
check "back in the pool"    "$(printf '%s' "$($H state all_handlers/rick)" | cut -d' ' -f1)" "UP"
check "no mail"             "$(grep -c . /tmp/mail.txt)" "0"

reset_fleet
echo "== 9. backups are updated last, whatever the inventory says =="
# rick is the backup in this config; put it first in the file and it must still go last.
cat > /work/servers.conf <<EOF
rick   rick 9103 all_handlers,db_handlers rick
patty   patty 9101 all_handlers,db_handlers patty
selma   selma 9102 all_handlers,db_handlers selma
EOF
check "is-backup: rick"  "$(/work/loadbalancer/haproxy.sh is-backup all_handlers/rick && echo yes)" "yes"
check "is-backup: patty" "$(/work/loadbalancer/haproxy.sh is-backup all_handlers/patty && echo yes || echo no)" "no"
$R --version v2.6.0 --dry-run >/tmp/order.txt 2>&1
check "rick is listed last" "$(grep -oE '^(rick|patty|selma)' /tmp/order.txt | tail -1)" "rick"

reset_fleet
echo "== 10. one rollout at a time =="
( flock -n 9 || exit 1; sleep 25 ) 9>/tmp/unipept-rollout.lock &
holder=$!
sleep 1
$R --version v2.6.0 --allow-downtime >/tmp/lock.txt 2>&1
check "exit non-zero"   "$([ $? -ne 0 ] && echo yes)" "yes"
check "names the lock"  "$(grep -c 'another rollout holds' /tmp/lock.txt)" "1"
check "nothing drained" "$(/work/loadbalancer/haproxy.sh state all_handlers/patty | cut -d' ' -f1)" "UP"
kill $holder 2>/dev/null; wait $holder 2>/dev/null

echo "== 11. a bad inventory is refused =="
printf 'patty   patty 9101 all_handlers patty\npatty   npatty 9102 all_handlers selma\n' > /work/servers.conf
$R --version v2.6.0 >/tmp/dup.txt 2>&1
check "duplicate name: non-zero" "$([ $? -ne 0 ] && echo yes)" "yes"
check "says names twice"         "$(grep -c 'twice' /tmp/dup.txt)" "1"
printf 'patty   patty 9101 all_handlers patty\nselma   nselma 9102 all_handlers patty\n' > /work/servers.conf
$R --version v2.6.0 >/tmp/dup2.txt 2>&1
check "duplicate server: non-zero" "$([ $? -ne 0 ] && echo yes)" "yes"
printf 'patty patty eighty all_handlers patty\n' > /work/servers.conf
$R --version v2.6.0 >/tmp/badport.txt 2>&1
check "bad port: non-zero" "$([ $? -ne 0 ] && echo yes)" "yes"
check "says not a number"  "$(grep -c 'not a number' /tmp/badport.txt)" "1"
cp /tmp/servers.keep /work/servers.conf 2>/dev/null || cat > /work/servers.conf <<EOF
patty   patty 9101 all_handlers,db_handlers patty
selma   selma 9102 all_handlers,db_handlers selma
rick   rick 9103 all_handlers,db_handlers rick
EOF

reset_fleet
echo "== 12. a check failure stops the run before anything is drained =="
FAKE_CHECK_FAILS="the index is missing" $R --version v2.6.0 >/tmp/pre2.txt 2>&1
check "exit non-zero"      "$([ $? -ne 0 ] && echo yes)" "yes"
check "says not ready"     "$(grep -c 'is not ready' /tmp/pre2.txt)" "3"
check "nothing was touched" "$(grep -c 'nothing was touched' /tmp/pre2.txt)" "1"
check "patty still UP"     "$(/work/loadbalancer/haproxy.sh state all_handlers/patty | cut -d' ' -f1)" "UP"

reset_fleet
echo "== 13. a fleet that disagrees on its index stops the run =="
cat > /usr/local/bin/ssh <<'EOF'
#!/usr/bin/env bash
args=("$@"); cmd=""
for a in "${args[@]}"; do case $a in -o|BatchMode=yes|ConnectTimeout=10|ServerAliveInterval=15|ServerAliveCountMax=4|-n) ;; *) cmd="$cmd $a" ;; esac; done
echo "SSH:$cmd" >> /tmp/ssh.log
# selma claims a different index than the others.
idx=2026.09-test; case "$cmd" in *@selma*) idx=2026.02-old ;; esac
case "$cmd" in
  *"deploy.sh check"*) printf 'variant=hybrid\nport=80\nindex_version=%s\nproblems=0\n' "$idx" ;;
  *status*) printf 'version=2.6.0\nprevious=2.5.3\nvariant=hybrid\nport=80\nactive=active\n' ;;
  *) exit 0 ;;
esac
EOF
chmod +x /usr/local/bin/ssh
$R --version v2.6.0 >/tmp/idx.txt 2>&1
check "exit non-zero"        "$([ $? -ne 0 ] && echo yes)" "yes"
check "says does not agree"  "$(grep -c 'does not agree on an index' /tmp/idx.txt)" "1"
check "nothing drained"      "$(/work/loadbalancer/haproxy.sh state all_handlers/patty | cut -d' ' -f1)" "UP"
$R --version v2.6.0 --allow-index-mismatch >/tmp/idx2.txt 2>&1
check "override proceeds"    "$(grep -c 'does not agree' /tmp/idx2.txt)" "1"
check "and reached the servers" "$([ "$(grep -c '=== ' /tmp/idx2.txt)" -ge 1 ] && echo yes)" "yes"

reset_fleet
echo "== 14. staging is cleared on every path, and the run is recorded =="
# A marker directory per host, created by the fake ssh's mkdir and removed by its rm.
cat > /usr/local/bin/ssh <<'EOF'
#!/usr/bin/env bash
args=("$@"); cmd=""
for a in "${args[@]}"; do case $a in -o|BatchMode=yes|ConnectTimeout=10|ServerAliveInterval=15|ServerAliveCountMax=4|-n) ;; *) cmd="$cmd $a" ;; esac; done
echo "SSH:$cmd" >> /tmp/ssh.log
host=$(printf '%s' "$cmd" | sed -n 's/.*@\([a-z]*\).*/\1/p')
case "$cmd" in
  *"mkdir -p"*) mkdir -p "/tmp/staged-${host}"; exit 0 ;;
  *"rm -rf"*)   rm -rf "/tmp/staged-${host}"; exit 0 ;;
  *"deploy.sh check"*)
    [ -n "${FAKE_CHECK_FAILS:-}" ] && { echo "check: ${FAKE_CHECK_FAILS}" >&2; echo "problems=1"; exit 1; }
    printf 'variant=hybrid\nport=80\nindex_version=2026.09-test\nproblems=0\n' ;;
  *status*) printf 'version=%s\nprevious=2.5.3\nvariant=hybrid\nport=80\nactive=active\n' "${FAKE_VERSION:-2.6.0}" ;;
  *) exit 0 ;;
esac
EOF
chmod +x /usr/local/bin/ssh
rm -rf /tmp/staged-* ; : > /tmp/logged.txt
printf '#!/usr/bin/env bash\necho "$*" >> /tmp/logged.txt\n' > /usr/local/bin/logger
chmod +x /usr/local/bin/logger

$R --version v2.6.0 --allow-downtime >/tmp/r14.txt 2>&1
check "success: staging gone"  "$(ls -d /tmp/staged-* 2>/dev/null | wc -l | tr -d ' ')" "0"
check "recorded every server"  "$(grep -c 'outcome=deployed' /tmp/logged.txt)" "3"
check "recorded the run"       "$(grep -c 'exit=0' /tmp/logged.txt)" "1"
check "names the operator"     "$(grep -c "by=$(id -un)" /tmp/logged.txt)" "4"

reset_fleet; rm -rf /tmp/staged-*; : > /tmp/logged.txt
FAKE_CHECK_FAILS=x $R --version v2.6.0 --allow-downtime >/tmp/r15.txt 2>&1
check "after a failure: staging gone" "$(ls -d /tmp/staged-* 2>/dev/null | wc -l | tr -d ' ')" "0"
check "recorded a non-zero exit"      "$(grep -c 'exit=1' /tmp/logged.txt)" "1"

reset_fleet; rm -rf /tmp/staged-*
$R --version v2.6.0 --allow-downtime >/dev/null 2>&1 &
runner=$!
sleep 2
kill -TERM $runner 2>/dev/null; wait $runner 2>/dev/null
sleep 1
check "after an interrupt: staging gone" "$(ls -d /tmp/staged-* 2>/dev/null | wc -l | tr -d ' ')" "0"

echo "== 15. rollout.sh status reads the fleet without changing it =="
reset_fleet
$R status >/tmp/r16.txt 2>&1
check "exit 0"            "$?" "0"
check "has a header"      "$(grep -c '^SERVER' /tmp/r16.txt)" "1"
check "lists all three"   "$(grep -cE '^(patty|selma|rick) ' /tmp/r16.txt)" "3"
check "shows haproxy"     "$(grep -c 'all_handlers=UP' /tmp/r16.txt)" "3"
check "changed nothing"   "$(printf '%s' "$($H state all_handlers/patty)" | cut -d' ' -f1)" "UP"

reset_fleet
section "16. a server is only returned to the pool when both routes answer"
# The deploy fails and leaves the database route down. The server is still on its old version, so
# there is nothing to undo — but db_handlers routes to it too, so putting it back on the strength of
# /health alone would return it for requests it cannot serve.
#
# The marker is set by the deploy, not before the run: set up front it would fail preflight instead,
# and the branch under test would never be reached.
cp /usr/local/bin/ssh /tmp/ssh.keep
cat > /usr/local/bin/ssh <<'EOF'
#!/usr/bin/env bash
args=("$@"); cmd=""
for a in "${args[@]}"; do case $a in -o|BatchMode=yes|ConnectTimeout=10|ServerAliveInterval=15|ServerAliveCountMax=4|-n) ;; *) cmd="$cmd $a" ;; esac; done
echo "SSH:$cmd" >> /tmp/ssh.log
case "$cmd" in
  *"deploy.sh rollback"*) echo "ROLLBACK" >> /tmp/ssh.log; exit 0 ;;
  *"deploy.sh check"*)    printf 'variant=hybrid\nport=80\nindex_version=2026.09-test\nproblems=0\n'; exit 0 ;;
  *"deploy --from"*)      echo patty > /tmp/no-database; exit 1 ;;
  *status*)            printf 'version=2.5.3\nprevious=2.5.3\nvariant=hybrid\nport=80\nactive=active\n'; exit 0 ;;
esac
exit 0
EOF
chmod +x /usr/local/bin/ssh
rm -f /tmp/no-database; : > /tmp/ssh.log
$R --version v2.6.0 --only patty --allow-downtime >/tmp/r20.txt 2>&1
check "exit non-zero"                 "$([ $? -ne 0 ] && echo yes)" "yes"
check "says nothing was installed"    "$(grep -c 'nothing was installed' /tmp/r20.txt)" "1"
check "not put back on /health alone" "$($H state all_handlers/patty)" "MAINT"
check "says which routes"             "$(grep -c 'both health routes' /tmp/r20.txt)" "1"
rm -f /tmp/no-database
$H ready all_handlers,db_handlers/patty >/dev/null 2>&1
sleep 5

reset_fleet
section "17. an unreachable server is not rolled back on a guess"
# The deploy fails and the host then cannot be asked what happened. Rolling back over the same dead
# connection would be guessing, and could undo a deploy that actually worked.
cat > /usr/local/bin/ssh <<'EOF'
#!/usr/bin/env bash
args=("$@"); cmd=""
for a in "${args[@]}"; do case $a in -o|BatchMode=yes|ConnectTimeout=10|ServerAliveInterval=15|ServerAliveCountMax=4|-n) ;; *) cmd="$cmd $a" ;; esac; done
echo "SSH:$cmd" >> /tmp/ssh.log
case "$cmd" in
  *"deploy.sh rollback"*) echo "ROLLBACK" >> /tmp/ssh.log; exit 0 ;;
  *"deploy.sh check"*)    printf 'variant=hybrid\nport=80\nindex_version=2026.09-test\nproblems=0\n'; exit 0 ;;
  *"deploy --from"*)      touch /tmp/gone; exit 1 ;;
  *status*)            [ -f /tmp/gone ] && exit 255; printf 'version=2.5.3\nprevious=2.5.3\nvariant=hybrid\nport=80\nactive=active\n'; exit 0 ;;
esac
exit 0
EOF
chmod +x /usr/local/bin/ssh
rm -f /tmp/gone; : > /tmp/ssh.log
$R --version v2.6.0 --only selma --allow-downtime >/tmp/r21.txt 2>&1
check "exit non-zero"        "$([ $? -ne 0 ] && echo yes)" "yes"
check "did not roll back"    "$(grep -c '^ROLLBACK' /tmp/ssh.log)" "0"
check "says it cannot ask"   "$(grep -c 'cannot be reached to ask' /tmp/r21.txt)" "1"
check "left out of the pool" "$($H state all_handlers/selma)" "MAINT"
cp /tmp/ssh.keep /usr/local/bin/ssh; rm -f /tmp/gone

reset_fleet
section "18. the release download is bounded by throughput"
# The same option set the server side uses, from lib.sh, because this downloads the same release the
# same way. `--retry` alone does not cover a transfer that connects and then goes quiet, and phase 0
# would wait on it for ever.
: > /tmp/curl-args.log
$R --version v2.6.0 --only patty --allow-downtime >/dev/null 2>&1
check "the release was fetched"    "$(grep -c 'releases/download' /tmp/curl-args.log)" "2"
check "a throughput floor on each" "$(grep 'releases/download' /tmp/curl-args.log | grep -c -- '--speed-limit 1024')" "2"
check "and a window for it"        "$(grep 'releases/download' /tmp/curl-args.log | grep -c -- '--speed-time 30')" "2"
# The health polls must keep their own short bound. A download's floor on one would mean a dead
# server took 30 seconds to report instead of 5.
check "the health polls still run"  "$([ "$(grep -c -- '--max-time 5' /tmp/curl-args.log)" -gt 0 ] && echo yes)" "yes"
check "and kept their own bound"    "$(grep -- '--max-time 5' /tmp/curl-args.log | grep -c -- '--speed-limit')" "0"

reset_fleet
section "19. a run interrupted during an install reports the server it left out"
# The one path that used to leave a server in maintenance and tell nobody: the traps hand back to the
# global handler once the server is in maint, so a signal during the install reached `finish` with
# nothing recorded. It has to be mailed and journalled like every other server left out of the pool.
cat > /usr/local/bin/ssh <<'EOF'
#!/usr/bin/env bash
args=("$@"); cmd=""
for a in "${args[@]}"; do case $a in -o|BatchMode=yes|ConnectTimeout=10|ServerAliveInterval=15|ServerAliveCountMax=4|-n) ;; *) cmd="$cmd $a" ;; esac; done
echo "SSH:$cmd" >> /tmp/ssh.log
case "$cmd" in
  *"deploy.sh check"*) printf 'variant=hybrid\nport=80\nindex_version=2026.09-test\nproblems=0\n'; exit 0 ;;
  # Still installing when the signal arrives, which is where a real deploy spends its minutes. The
  # duration is the marker the case kills it by, so it is distinctive rather than round.
  # exec, so the stand-in is the sleep rather than a shell waiting on one: a real ssh is a binary
  # that dies on TERM, and a shell would defer the signal until its own child returned.
  *"deploy --from"*)   exec sleep 971 ;;
  *status*)            printf 'version=2.5.3\nprevious=2.5.3\nvariant=hybrid\nport=80\nactive=active\n'; exit 0 ;;
esac
exit 0
EOF
chmod +x /usr/local/bin/ssh
: > /tmp/ssh.log; : > /tmp/mail.txt; : > /tmp/logged.txt
$R --version v2.6.0 --only patty --allow-downtime >/tmp/r22.txt 2>&1 &
runner=$!
# Signal it once it is inside the install, which is after the drain and the wait for an empty
# server. Bounded, so a run that never gets there fails the case instead of hanging the suite.
waited=0
until grep -q 'deploy --from' /tmp/ssh.log 2>/dev/null; do
  sleep 1; waited=$((waited + 1))
  [ "$waited" -lt 60 ] || break
done
check "reached the install"   "$(grep -c 'deploy --from' /tmp/ssh.log)" "1"
kill -TERM $runner 2>/dev/null
# The install is a foreground child, and bash runs a trap only once that returns, so the stand-in
# deploy has to end before `finish` does anything. Ending it here is what a dying ssh does to a real
# one; without it the case waits out the whole sleep for a signal it has already delivered.
pkill -f 'sleep 971' >/dev/null 2>&1
wait $runner 2>/dev/null
check "left in maintenance"   "$($H state all_handlers/patty)" "MAINT"
check "said so"               "$(grep -c 'left out of the pool by a run that did not finish' /tmp/r22.txt)" "1"
check "mailed about it"       "$(grep -c 'needs attention' /tmp/mail.txt)" "1"
check "named the server"      "$(grep -c 'patty' /tmp/mail.txt)" "1"
check "journalled it"         "$(grep -c 'server=patty.*outcome=needs-attention' /tmp/logged.txt)" "1"
check "did not roll back"     "$(grep -c '^ROLLBACK' /tmp/ssh.log)" "0"

reset_fleet
section "20. a server is given time to answer, not one sample"
# Both callers of `serving` reach it just after the server was restarted, and a process that has come
# back can miss a first connection without anything being wrong. The database route the more easily:
# it gives OpenSearch two seconds of its own. Sampled once, a server that is fine was left out of the
# pool and mailed about.
cat > /usr/local/bin/ssh <<'EOF'
#!/usr/bin/env bash
args=("$@"); cmd=""
for a in "${args[@]}"; do case $a in -o|BatchMode=yes|ConnectTimeout=10|ServerAliveInterval=15|ServerAliveCountMax=4|-n) ;; *) cmd="$cmd $a" ;; esac; done
echo "SSH:$cmd" >> /tmp/ssh.log
case "$cmd" in
  *"deploy.sh check"*) printf 'variant=hybrid\nport=80\nindex_version=2026.09-test\nproblems=0\n'; exit 0 ;;
  # Installs nothing and leaves the database route slow to come back, which is the state a restarted
  # OpenSearch is in. 15 seconds: beyond any single sample, well inside HEALTH_TIMEOUT.
  *"deploy --from"*)   printf 'selma %s\n' "$(( $(date +%s) + 15 ))" > /tmp/no-database-until; exit 1 ;;
  *status*)            printf 'version=2.5.3\nprevious=2.5.3\nvariant=hybrid\nport=80\nactive=active\n'; exit 0 ;;
esac
exit 0
EOF
chmod +x /usr/local/bin/ssh
rm -f /tmp/no-database-until; : > /tmp/ssh.log; : > /tmp/mail.txt
$R --version v2.6.0 --only selma --allow-downtime >/tmp/r23.txt 2>&1
check "stopped the run"        "$(grep -c 'stopped at selma' /tmp/r23.txt)" "1"
check "waited for the route"   "$(printf '%s' "$($H state all_handlers/selma)" | cut -d' ' -f1)" "UP"
check "and the db backend too" "$(printf '%s' "$($H state db_handlers/selma)" | cut -d' ' -f1)" "UP"
check "not called down"        "$(grep -c 'does not answer both health routes' /tmp/r23.txt)" "0"
check "no urgent mail"         "$(grep -c 'needs attention' /tmp/mail.txt)" "0"
rm -f /tmp/no-database-until

reset_fleet
section "21. each server is given the deadline it asks for"
# rick holds the preloaded build on a slow disk and reads its index before it answers, so it needs far
# longer than the rest. One deadline for the fleet is wrong either way: too short for rick, or every
# other server waiting rick's hour before a real failure is reported.
cat > /usr/local/bin/ssh <<'EOF'
#!/usr/bin/env bash
args=("$@"); cmd=""
for a in "${args[@]}"; do case $a in -o|BatchMode=yes|ConnectTimeout=10|ServerAliveInterval=15|ServerAliveCountMax=4|-n) ;; *) cmd="$cmd $a" ;; esac; done
echo "SSH:$cmd" >> /tmp/ssh.log
host=$(printf '%s' "$cmd" | sed -n 's/.*@\([a-z]*\).*/\1/p')
case "$cmd" in
  *"deploy.sh check"*)
    # rick names its own; patty answers like a server whose deploy.sh is too old to report one.
    printf 'variant=hybrid\nport=80\nindex_version=2026.09-test\nproblems=0\n'
    [ "$host" = rick ] && printf 'ready_timeout=4200\n'
    exit 0 ;;
  *status*) printf 'version=2.6.0\nprevious=2.5.3\nvariant=hybrid\nport=80\nactive=active\n'; exit 0 ;;
esac
exit 0
EOF
chmod +x /usr/local/bin/ssh
: > /tmp/ssh.log
$R --version v2.6.0 --only rick --allow-downtime >/tmp/r24.txt 2>&1
check "rick gets its own"   "$(grep -c 'deploy --from.*--timeout 4200' /tmp/ssh.log)" "1"
: > /tmp/ssh.log
$R --version v2.6.0 --only patty --allow-downtime >/tmp/r25.txt 2>&1
check "patty falls back"    "$(grep -c 'deploy --from.*--timeout 900' /tmp/ssh.log)" "1"
# A value that is not seconds must not reach `$((SECONDS + timeout))` on the server.
cat > /usr/local/bin/ssh <<'EOF'
#!/usr/bin/env bash
args=("$@"); cmd=""
for a in "${args[@]}"; do case $a in -o|BatchMode=yes|ConnectTimeout=10|ServerAliveInterval=15|ServerAliveCountMax=4|-n) ;; *) cmd="$cmd $a" ;; esac; done
echo "SSH:$cmd" >> /tmp/ssh.log
case "$cmd" in
  *"deploy.sh check"*) printf 'variant=hybrid\nport=80\nindex_version=2026.09-test\nready_timeout=soon\nproblems=0\n'; exit 0 ;;
  *status*) printf 'version=2.6.0\nprevious=2.5.3\nvariant=hybrid\nport=80\nactive=active\n'; exit 0 ;;
esac
exit 0
EOF
chmod +x /usr/local/bin/ssh
: > /tmp/ssh.log
$R --version v2.6.0 --only selma --allow-downtime >/tmp/r26.txt 2>&1
check "a bad value is not passed on" "$(grep -c 'deploy --from.*--timeout 900' /tmp/ssh.log)" "1"
cp /tmp/ssh.keep /usr/local/bin/ssh

reset_fleet
section "22. the fleet's own versions are reported before anything is installed"
# A fleet that disagrees is what a run which stopped part way leaves behind, and rolling out again
# is the cure — so this is said, never refused. Before this, the next rollout began with no mention
# that the fleet was split.
cat > /usr/local/bin/ssh <<'EOF'
#!/usr/bin/env bash
args=("$@"); cmd=""
for a in "${args[@]}"; do case $a in -o|BatchMode=yes|ConnectTimeout=10|ServerAliveInterval=15|ServerAliveCountMax=4|-n) ;; *) cmd="$cmd $a" ;; esac; done
echo "SSH:$cmd" >> /tmp/ssh.log
host=$(printf '%s' "$cmd" | sed -n 's/.*@\([a-z]*\).*/\1/p')
case "$cmd" in
  *"deploy.sh check"*) printf 'variant=hybrid\nport=80\nindex_version=2026.09-test\nproblems=0\n'; exit 0 ;;
  # patty is already on the new one; the other two are behind, which is a stopped run's fleet.
  *status*)
    if [ "$host" = patty ]; then v=2.6.0; else v=2.5.3; fi
    printf 'version=%s\nprevious=2.5.3\nvariant=hybrid\nport=80\nactive=active\n' "$v"; exit 0 ;;
esac
exit 0
EOF
chmod +x /usr/local/bin/ssh
$R --version v2.6.0 --dry-run >/dev/null 2>&1
: > /tmp/ssh.log
$R --version v2.6.0 --allow-downtime >/tmp/r27.txt 2>&1
check "says the fleet is split"  "$(grep -c 'the fleet is not on one version' /tmp/r27.txt)" "1"
check "names each version"       "$([ "$(grep -c 'selma=2.5.3' /tmp/r27.txt)" -ge 1 ] && echo yes)" "yes"
check "names the one ahead"      "$([ "$(grep -c 'already on 2.6.0: patty' /tmp/r27.txt)" -ge 1 ] && echo yes)" "yes"
check "and did not refuse"       "$(grep -c 'preflight passed' /tmp/r27.txt)" "1"
cp /tmp/ssh.keep /usr/local/bin/ssh

reset_fleet
section "23. a run that changes nothing is not recorded as a rollout"
# `trap finish EXIT` covers every invocation, so `status` used to journal `version= exit=0` and read
# back as a rollout of nothing.
: > /tmp/logged.txt
$R status >/dev/null 2>&1
check "status journalled nothing" "$(grep -c . /tmp/logged.txt)" "0"
: > /tmp/logged.txt
$R --version v2.6.0 --only patty --allow-downtime >/dev/null 2>&1
check "a rollout still is"        "$([ "$(grep -c 'version=v2.6.0' /tmp/logged.txt)" -ge 1 ] && echo yes)" "yes"

# The fake backends hold stdout open; without this a pipe on the outside never sees EOF.
pkill -f 'TCP-LISTEN' >/dev/null 2>&1
kill "$(jobs -p)" >/dev/null 2>&1
summary

#!/usr/bin/env bash
# install.sh + deploy.sh against a real systemd user manager. No sudo anywhere after install.
# shellcheck disable=SC2181,SC2012  # SC2181: these cases check the exit status of a command whose
# output went to a log file, which is then grepped, so `if ! cmd` cannot do both. SC2012: `ls | wc -l`
# over a known-simple path is clearer here than `find`.
set -uo pipefail

R=/deploy
# The container path; shellcheck is pointed at the checkout instead.
# shellcheck source-path=SCRIPTDIR source=../lib.sh
source /deploy/tests/lib.sh

echo "== install.sh (the one root step) =="
$R/server/install.sh >/tmp/install.log 2>&1; check "exit 0" "$?" "0"
check "linger on"      "$(loginctl show-user unipept -p Linger --value)" "yes"
check "bin dir owned"  "$(stat -c %U /opt/unipept-api/bin)" "unipept"
check "unit installed" "$([ -f /home/unipept/.config/systemd/user/unipept-api.service ] && echo yes)" "yes"
check "deploy.sh there" "$(stat -c '%U %a' /opt/unipept-api/lib/deploy.sh)" "unipept 755"

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
trap 'exit 0' TERM
while true; do printf 'HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok' | timeout 5 nc -l -p "\$PORT" -q0 >/dev/null 2>&1 || sleep 0.05; done
EOF
  sed -i "s/unipept-api \$1/unipept-api $1/" "$d/unipept-api-$1-x86_64-linux-gnu-hybrid"
  chmod 755 "$d/unipept-api-$1-x86_64-linux-gnu-hybrid"
  ( cd "$d" && sha256sum unipept-api-* > SHA256SUMS )
  echo "$d"
}

echo "== deploy as the unipept user, no sudo =="
d1=$(stage 2.6.0 yes)
as_user "/opt/unipept-api/lib/deploy.sh deploy --from $d1/unipept-api-2.6.0-x86_64-linux-gnu-hybrid --timeout 30" >/tmp/d1.log 2>&1
check "exit 0" "$?" "0"
check "unit active"    "$(as_user 'systemctl --user is-active unipept-api')" "active"
check "health answers" "$(curl -s -o /dev/null -w '%{http_code}' http://127.0.0.1:8099/health)" "200"
check "version live"   "$(/opt/unipept-api/bin/unipept-api --version)" "unipept-api 2.6.0"

echo "== the journal has the unit's output =="
check "journal readable as root" "$([ -n "$(journalctl -n 20 _SYSTEMD_USER_UNIT=unipept-api.service 2>/dev/null)" ] && echo yes)" "yes"

echo "== status, read the way the rollout reads it =="
as_user "/opt/unipept-api/lib/deploy.sh status" > /tmp/st.txt 2>/dev/null
check "version line" "$(sed -n 's/^version=//p' /tmp/st.txt)" "2.6.0"
check "variant line" "$(sed -n 's/^variant=//p' /tmp/st.txt)" "hybrid"
check "active line"  "$(sed -n 's/^active=//p' /tmp/st.txt)" "active"

echo "== upgrade keeps the old binary =="
d2=$(stage 2.7.0 yes)
as_user "/opt/unipept-api/lib/deploy.sh deploy --from $d2/unipept-api-2.7.0-x86_64-linux-gnu-hybrid --timeout 30" >/tmp/d2.log 2>&1
check "exit 0" "$?" "0"
check "new version" "$(/opt/unipept-api/bin/unipept-api --version)" "unipept-api 2.7.0"
check "previous kept" "$(/opt/unipept-api/bin/unipept-api.previous --version)" "unipept-api 2.6.0"

echo "== an unhealthy deploy rolls itself back =="
d3=$(stage 2.9.0 no)
as_user "/opt/unipept-api/lib/deploy.sh deploy --from $d3/unipept-api-2.9.0-x86_64-linux-gnu-hybrid --timeout 12" >/tmp/d3.log 2>&1
check "exit non-zero" "$([ $? -ne 0 ] && echo yes)" "yes"
check "back on the previous" "$(/opt/unipept-api/bin/unipept-api --version)" "unipept-api 2.7.0"
check "serving again" "$(curl -s -o /dev/null -w '%{http_code}' http://127.0.0.1:8099/health)" "200"

echo "== a corrupted binary is refused before anything is installed =="
d4=$(stage 3.0.0 yes)
echo tampered >> "$d4/unipept-api-3.0.0-x86_64-linux-gnu-hybrid"
as_user "/opt/unipept-api/lib/deploy.sh deploy --from $d4/unipept-api-3.0.0-x86_64-linux-gnu-hybrid --timeout 20" >/tmp/d4.log 2>&1
check "exit non-zero" "$([ $? -ne 0 ] && echo yes)" "yes"
check "says checksum"  "$([ "$(grep -c 'checksum' /tmp/d4.log)" -ge 1 ] && echo yes)" "yes"
check "binary untouched" "$(/opt/unipept-api/bin/unipept-api --version)" "unipept-api 2.7.0"

echo "== rollback with nothing to go back to =="
rm -f /opt/unipept-api/bin/unipept-api.previous
as_user "/opt/unipept-api/lib/deploy.sh rollback --timeout 5" >/tmp/d5.log 2>&1
check "exit non-zero" "$([ $? -ne 0 ] && echo yes)" "yes"
check "says no previous" "$(grep -c 'no previous binary' /tmp/d5.log)" "1"

echo "== deploy with neither --from nor --version =="
as_user "/opt/unipept-api/lib/deploy.sh deploy" >/tmp/d6.log 2>&1
check "exit 2" "$?" "2"

echo "== root is refused, to keep the user manager the right one =="
/opt/unipept-api/lib/deploy.sh status >/tmp/root.log 2>&1
check "exit non-zero" "$([ $? -ne 0 ] && echo yes)" "yes"
check "says run as unipept" "$(grep -c 'run this as unipept' /tmp/root.log)" "1"

echo "== the service user can run a remote command over ssh =="
check "login shell is not nologin" "$(getent passwd unipept | cut -d: -f7)" "/bin/bash"

echo "== the environment file is not world readable =="
check "mode 0600" "$(stat -c %a /opt/unipept-api/etc/unipept-api.env)" "600"

echo "== a flag without its value says usage =="
as_user "/opt/unipept-api/lib/deploy.sh deploy --version" >/tmp/f1.log 2>&1
check "exit 2"      "$?" "2"
check "printed usage" "$(grep -c 'usage:' /tmp/f1.log)" "1"

echo "== status answers for a binary too old for --version =="
printf '#!/bin/sh\nexit 2\n' > /opt/unipept-api/bin/unipept-api.old && chmod 755 /opt/unipept-api/bin/unipept-api.old
cp /opt/unipept-api/bin/unipept-api /tmp/keep-real
cp /opt/unipept-api/bin/unipept-api.old /opt/unipept-api/bin/unipept-api
as_user "/opt/unipept-api/lib/deploy.sh status" >/tmp/f2.log 2>&1
check "exit 0"          "$?" "0"
check "says unknown"    "$(sed -n 's/^version=//p' /tmp/f2.log)" "unknown"
check "still reports variant" "$(sed -n 's/^variant=//p' /tmp/f2.log)" "hybrid"
cp /tmp/keep-real /opt/unipept-api/bin/unipept-api

echo "== a restart that fails rolls back rather than aborting =="
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

echo "== a restart that returns non-zero rolls back, rather than aborting =="
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

echo "== check passes on a good host and reports the index version =="
as_user "/opt/unipept-api/lib/deploy.sh check" >/tmp/c0.log 2>&1
check "exit 0"            "$?" "0"
check "index_version"     "$(sed -n 's/^index_version=//p' /tmp/c0.log)" "2026.09-test"
check "no problems"       "$(sed -n 's/^problems=//p' /tmp/c0.log)" "0"

echo "== each failure on its own =="
# A missing index file.
mv /srv/index/mapping.bin /srv/mapping.bin.away
as_user "/opt/unipept-api/lib/deploy.sh check" >/tmp/c1.log 2>&1
check "missing file: non-zero" "$([ $? -ne 0 ] && echo yes)" "yes"
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

echo "== the memory arm judges each variant on what it holds resident =="
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

echo "== check --from validates the delivered binary =="
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

echo "== deploy refuses when check fails, before swapping anything =="
running_before=$(/opt/unipept-api/bin/unipept-api --version)
sed -i 's#^VARIANT=.*#VARIANT=nonsense#' /opt/unipept-api/etc/unipept-api.env
d22=$(stage 4.2.0 yes)
as_user "/opt/unipept-api/lib/deploy.sh deploy --from $d22/unipept-api-4.2.0-x86_64-linux-gnu-hybrid --timeout 20" >/tmp/c13.log 2>&1
check "exit non-zero"      "$([ $? -ne 0 ] && echo yes)" "yes"
check "says not ready"     "$(grep -c 'not ready' /tmp/c13.log)" "1"
check "binary untouched"   "$(/opt/unipept-api/bin/unipept-api --version)" "$running_before"
sed -i 's#^VARIANT=.*#VARIANT=hybrid#' /opt/unipept-api/etc/unipept-api.env

echo "== --no-rollback leaves the decision to the caller =="
d23=$(stage 4.3.0 no)
as_user "/opt/unipept-api/lib/deploy.sh deploy --from $d23/unipept-api-4.3.0-x86_64-linux-gnu-hybrid --timeout 10 --no-rollback" >/tmp/c14.log 2>&1
check "exit non-zero"        "$([ $? -ne 0 ] && echo yes)" "yes"
check "new binary still on"  "$(/opt/unipept-api/bin/unipept-api --version)" "unipept-api 4.3.0"
check "did not roll back"    "$(grep -c 'rolled back' /tmp/c14.log)" "0"
check "says caller decides"  "$(grep -c 'for the caller to decide' /tmp/c14.log)" "1"
check "no orphan .new"       "$([ -e /opt/unipept-api/bin/unipept-api.new ] && echo present || echo absent)" "absent"
# Put the host back for the cases after this.
as_user "/opt/unipept-api/lib/deploy.sh rollback --timeout 20" >/dev/null 2>&1

echo "== a failed rollback keeps .previous and the rejected binary =="
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

echo "== the swap never leaves the binary absent =="
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

echo "== an interrupt after the swap rolls back =="
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

echo "== HUP behaves like TERM, which is what a dying ssh sends =="
d31=$(stage 7.1.0 no)
as_user "/opt/unipept-api/lib/deploy.sh deploy --from $d31/unipept-api-7.1.0-x86_64-linux-gnu-hybrid --timeout 120" >/tmp/i2.log 2>&1 &
runner=$!
sleep 6
pkill -HUP -f 'deploy.sh deploy --from' >/dev/null 2>&1
wait $runner 2>/dev/null
check "caught HUP"          "$(grep -c 'caught HUP' /tmp/i2.log)" "1"
check "rolled back"         "$(grep -c 'rolling back' /tmp/i2.log)" "1"
check "serving again"       "$(curl -s -o /dev/null -w '%{http_code}' http://127.0.0.1:8099/health)" "200"

echo "== an interrupt before the swap changes nothing =="
running=$(/opt/unipept-api/bin/unipept-api --version)
as_user "/opt/unipept-api/lib/deploy.sh deploy --version v9.9.9 --timeout 30" >/tmp/i3.log 2>&1 &
runner=$!
sleep 1
pkill -TERM -f 'deploy.sh deploy --version' >/dev/null 2>&1
wait $runner 2>/dev/null
check "binary untouched"  "$(/opt/unipept-api/bin/unipept-api --version)" "$running"
check "did not roll back" "$(grep -c 'rolling back' /tmp/i3.log)" "0"
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

echo "== install.sh is idempotent and keeps an edited env file =="
$R/server/install.sh >/dev/null 2>&1; check "exit 0" "$?" "0"
check "PORT kept" "$(sed -n 's/^PORT=//p' /opt/unipept-api/etc/unipept-api.env)" "8099"

summary

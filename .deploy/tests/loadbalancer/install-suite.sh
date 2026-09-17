#!/usr/bin/env bash
# loadbalancer/install.sh against a real HAProxy: what it installs, what it audits, and what it
# refuses to touch.
# shellcheck disable=SC2181,SC2012  # SC2181: these cases check the exit status of a command whose
# output went to a log file, which is then grepped, so `if ! cmd` cannot do both. SC2012: `ls | wc -l`
# over a known-simple path is clearer here than `find`.
set -uo pipefail

# The container path; shellcheck is pointed at the checkout instead.
# shellcheck source-path=SCRIPTDIR source=../lib.sh
source /deploy/tests/lib.sh

printf '127.0.0.1 patty selma rick\n' >> /etc/hosts
useradd --create-home unipept 2>/dev/null
groupadd haproxy 2>/dev/null

mkdir -p /run/haproxy
printf 'HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok' > /response.http
for port in 9101 9102 9103; do
  socat TCP-LISTEN:"$port",reuseaddr,fork SYSTEM:'cat /response.http' >/dev/null 2>&1 &
done
sleep 1
haproxy -f /etc/haproxy/haproxy.cfg -D
sleep 3

# An ssh that answers the way a prepared server would, so the connectivity audit has something real
# to talk to.
cat > /usr/local/bin/ssh <<'EOF'
#!/usr/bin/env bash
# Steered by a file rather than the environment: the script reaches ssh through `sudo -u`, which
# resets the environment, and that is correct — it is the operator's keys being tested, not root's.
mode=$(cat /tmp/fake-ssh-mode 2>/dev/null || echo ok)
cmd="$*"
printf '%s\n' "$cmd" >> /tmp/audit-ssh.log
case "$cmd" in
  *"test -x"*)         [ "$mode" = no-deploy ] && exit 1; [ "$mode" = no-ssh ] && exit 255; exit 0 ;;
  *"deploy.sh check"*) [ "$mode" = check-fails ] && exit 1; [ "$mode" = no-ssh ] && exit 255; exit 0 ;;
  *)                   [ "$mode" = no-ssh ] && exit 255; exit 0 ;;
esac
EOF
chmod +x /usr/local/bin/ssh
export PATH=/usr/local/bin:$PATH

section "a first install"
# A first install on a host whose inventory still names the example servers: the audit will have
# something to say, so the exit status is not asserted here. What matters is what it installed.
/deploy/loadbalancer/install.sh >/tmp/i.log 2>&1 || true
check "installed the scripts"   "$([ -x /opt/unipept-rollout/rollout.sh ] && echo yes)" "yes"
check "installed haproxy.sh"    "$([ -x /opt/unipept-rollout/loadbalancer/haproxy.sh ] && echo yes)" "yes"
check "wrote rollout.conf"      "$([ -f /etc/unipept-rollout/rollout.conf ] && echo yes)" "yes"
check "wrote servers.conf"      "$([ -f /etc/unipept-rollout/servers.conf ] && echo yes)" "yes"
check "inventory owned by unipept" "$(stat -c %U /etc/unipept-rollout/servers.conf)" "unipept"
# Sourced by root in the audit, so a line in it is a command root runs. The operator has an ssh key
# on every API server; being able to write this file as well would make that account root here.
check "settings owned by root"     "$(stat -c %U /etc/unipept-rollout/rollout.conf)" "root"
check "added to the group"      "$(id -nG unipept | grep -c haproxy)" "1"

section "existing configuration is never overwritten"
echo "# edited by hand" >> /etc/unipept-rollout/servers.conf
/deploy/loadbalancer/install.sh >/tmp/i2.log 2>&1
check "kept the edit"  "$(grep -c 'edited by hand' /etc/unipept-rollout/servers.conf)" "1"
check "said it kept it" "$(grep -c 'keeping /etc/unipept-rollout/servers.conf' /tmp/i2.log)" "1"

section "a rollout.conf from before is taken back"
# A load balancer installed earlier has an operator-owned rollout.conf, and the audit sources it.
# Re-running the install is what corrects that, so the file has to keep its contents and change
# hands.
chown unipept:unipept /etc/unipept-rollout/rollout.conf
echo "# edited by hand" >> /etc/unipept-rollout/rollout.conf
/deploy/loadbalancer/install.sh >/tmp/i2b.log 2>&1
check "now owned by root" "$(stat -c %U /etc/unipept-rollout/rollout.conf)" "root"
check "kept the edit"     "$(grep -c 'edited by hand' /etc/unipept-rollout/rollout.conf)" "1"

section "the audit reads HAProxy, not just the file"
# The test config has both backends and all three servers.
check "no missing servers" "$(grep -c 'is in every backend it names' /tmp/i2.log)" "1"
check "checked all_handlers' uri" "$(grep -c 'all_handlers checks /health' /tmp/i2.log)" "1"
check "checked db_handlers' uri"  "$(grep -c 'db_handlers checks /health/database' /tmp/i2.log)" "1"

section "a missing backend is named, and a fragment written"
# An inventory naming a backend HAProxy is not running.
sed -i 's#all_handlers,db_handlers#all_handlers,absent_backend#' /etc/unipept-rollout/servers.conf
rm -f /tmp/unipept-haproxy-fragment.cfg
/deploy/loadbalancer/install.sh >/tmp/i3.log 2>&1
check "exit non-zero"      "$([ $? -ne 0 ] && echo yes)" "yes"
check "names what is missing" "$(grep -c 'absent_backend' /tmp/i3.log)" "1"
check "wrote the fragment"    "$([ -f /tmp/unipept-haproxy-fragment.cfg ] && echo yes)" "yes"
check "fragment has the backend" "$(grep -c '^backend db_handlers' /tmp/unipept-haproxy-fragment.cfg)" "1"
# notice mails every transition a rollout makes; alert keeps only the failures.
check "mails failures, not rollouts" "$(grep -c '^ *email-alert level alert$' /tmp/unipept-haproxy-fragment.cfg)" "1"
check_absent "not at notice level" 'email-alert level notice' /tmp/unipept-haproxy-fragment.cfg
check_absent "haproxy.cfg untouched" 'absent_backend' /etc/haproxy/haproxy.cfg
sed -i 's#all_handlers,absent_backend#all_handlers,db_handlers#' /etc/unipept-rollout/servers.conf

section "a wrong check uri is named"
cp /etc/haproxy/haproxy.cfg /tmp/haproxy.cfg.keep
sed -i 's#uri /health/database#uri /private_api/metadata.json#' /etc/haproxy/haproxy.cfg
/deploy/loadbalancer/install.sh >/tmp/i4.log 2>&1
check "exit non-zero"  "$([ $? -ne 0 ] && echo yes)" "yes"
check "names the uri"  "$(grep -c 'db_handlers checks /private_api/metadata.json' /tmp/i4.log)" "1"
cp /tmp/haproxy.cfg.keep /etc/haproxy/haproxy.cfg

section "the servers are reached the way a rollout reaches them"
echo no-ssh > /tmp/fake-ssh-mode
/deploy/loadbalancer/install.sh >/tmp/i5.log 2>&1
check "exit non-zero"     "$([ $? -ne 0 ] && echo yes)" "yes"
check "says it cannot ssh" "$([ "$(grep -c 'cannot ssh' /tmp/i5.log)" -ge 1 ] && echo yes)" "yes"
echo no-deploy > /tmp/fake-ssh-mode
/deploy/loadbalancer/install.sh >/tmp/i6.log 2>&1
check "says deploy.sh is absent" "$([ "$(grep -c 'has no /opt/unipept-api/lib/deploy.sh' /tmp/i6.log)" -ge 1 ] && echo yes)" "yes"
echo check-fails > /tmp/fake-ssh-mode
/deploy/loadbalancer/install.sh >/tmp/i7.log 2>&1
check "says the server is not ready" "$([ "$(grep -c 'is not ready' /tmp/i7.log)" -ge 1 ] && echo yes)" "yes"

section "a clean host reports ready"
echo ok > /tmp/fake-ssh-mode
: > /tmp/audit-ssh.log
/deploy/loadbalancer/install.sh >/tmp/i8.log 2>&1
check "exit 0"       "$?" "0"
check "says ready"   "$(grep -c 'this load balancer is ready' /tmp/i8.log)" "1"

section "the audit cannot wait for ever on a server that goes quiet"
# ConnectTimeout bounds the handshake only. A server that answers and then stops holds the
# connection open, and without keepalives this audit waits on it with nothing to end it. Every one
# of the three probes has to carry them, not just the first.
probes=$(grep -c 'ConnectTimeout=10' /tmp/audit-ssh.log)
check "every probe is bounded"  "$([ "$probes" -ge 3 ] && echo yes)" "yes"
check "keepalives on each"      "$(grep -c 'ServerAliveInterval=15' /tmp/audit-ssh.log)" "$probes"
check "and a count for them"    "$(grep -c 'ServerAliveCountMax=4' /tmp/audit-ssh.log)" "$probes"

section "rollout.conf is read the way rollout.sh reads it"
# Matching the lines with sed rather than sourcing made anything shell understands and a
# line-matcher does not part of the value. The example file comments half its settings, so an inline
# comment is the natural thing for an operator to add — and the audit then tried to reach
# `unipept  # the deploy account@patty` and called every server unreachable, while the rollout it
# audits read the same file and worked.
#
# The log belongs to the operator the audit runs as, so this reads the lines this run added rather
# than truncating it. Asked of the whole file, "reaches the plain target" would pass on the earlier
# runs' lines whatever this one did.
echo ok > /tmp/fake-ssh-mode
cp /etc/unipept-rollout/rollout.conf /tmp/rollout.conf.keep
sed -i 's/^SSH_USER=.*/SSH_USER=unipept  # the deploy account/' /etc/unipept-rollout/rollout.conf
before=$(wc -l < /tmp/audit-ssh.log)
/deploy/loadbalancer/install.sh >/tmp/i9.log 2>&1
check "exit 0"                   "$?" "0"
tail -n +$((before + 1)) /tmp/audit-ssh.log > /tmp/audit-ssh.new
check "reaches the plain target" "$([ "$(grep -c 'unipept@patty' /tmp/audit-ssh.new)" -ge 1 ] && echo yes)" "yes"
check_absent "the comment is not part of the user" 'deploy account@' /tmp/audit-ssh.new
cp /tmp/rollout.conf.keep /etc/unipept-rollout/rollout.conf

section "rollout.sh reads the installed configuration"
check "prefers /etc" "$(grep -c 'CONFIG_DIR=/etc/unipept-rollout' /opt/unipept-rollout/rollout.sh)" "1"

section "the installed rollout can find haproxy.sh"
# rollout.sh resolves it as loadbalancer/haproxy.sh relative to itself, so a flat install leaves the
# installed copy unable to reach HAProxy at all — and ordered_servers would swallow the failure and
# sort the backup as a primary.
check "installed in place"   "$([ -x /opt/unipept-rollout/loadbalancer/haproxy.sh ] && echo yes)" "yes"
echo ok > /tmp/fake-ssh-mode
cat > /etc/unipept-rollout/servers.conf <<EOF
patty  patty 9101 all_handlers,db_handlers patty
selma  selma 9102 all_handlers,db_handlers selma
rick   rick  9103 all_handlers,db_handlers rick
EOF
/opt/unipept-rollout/rollout.sh --version v2.6.0 --dry-run >/tmp/installed.txt 2>&1
check "the installed copy runs"  "$?" "0"
check "and reached HAProxy"      "$(grep -c 'all_handlers=UP' /tmp/installed.txt)" "3"
check "backup still sorted last" "$(grep -oE '^(patty|selma|rick)' /tmp/installed.txt | tail -1)" "rick"

pkill -f 'TCP-LISTEN' >/dev/null 2>&1
kill "$(jobs -p)" >/dev/null 2>&1
summary

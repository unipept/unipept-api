#!/usr/bin/env bash
# shellcheck disable=SC2181,SC2012  # SC2181: these cases check the exit status of a command whose
# output went to a log file, which is then grepped, so `if ! cmd` cannot do both. SC2012: `ls | wc -l`
# over a known-simple path is clearer here than `find`.
set -uo pipefail

H=/deploy/loadbalancer/haproxy.sh
export HAPROXY_SOCKET=/run/haproxy/haproxy.sock
mkdir -p /run/haproxy

# Three fake backends answering /health.
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

# The container path; shellcheck is pointed at the checkout instead.
# shellcheck source-path=SCRIPTDIR source=../lib.sh
source /deploy/tests/lib.sh

echo "== haproxy version =="; haproxy -v | head -1

echo "== 1. state and sessions are read by column name =="
check "patty is UP"        "$($H state all_handlers/patty)" "UP"
check "sessions is 0"      "$($H sessions all_handlers/patty)" "0"
check "up-count counts 3"  "$($H up-count all_handlers)" "3"

echo "== 2. an unknown server is refused, not silently wrong =="
$H state all_handlers/nosuch >/tmp/e1 2>&1; check "exit non-zero" "$([ $? -ne 0 ] && echo yes)" "yes"
check "says not in config" "$(grep -c 'is not in every one of' /tmp/e1)" "1"
$H state bogus >/tmp/e2 2>&1; check "bad target exit 1" "$([ $? -ne 0 ] && echo yes)" "yes"
check "says expected form" "$(grep -c 'expected <backend' /tmp/e2)" "1"

echo "== 3. drain, then maint, then ready =="
$H drain all_handlers/patty >/dev/null 2>&1
check "state is DRAIN"     "$($H state all_handlers/patty)" "DRAIN"
check "up-count drops to 2" "$($H up-count all_handlers)" "2"
$H wait-empty all_handlers/patty 30 >/dev/null 2>&1; check "wait-empty exit 0" "$?" "0"
$H maint all_handlers/patty >/dev/null 2>&1
check "state is MAINT"     "$($H state all_handlers/patty)" "MAINT"
$H ready all_handlers/patty >/dev/null 2>&1
$H wait-up all_handlers/patty 60 >/dev/null 2>&1; check "wait-up exit 0" "$?" "0"
# The status field carries the check counter in transition, so compare the first word.
back=$($H state all_handlers/patty); check "back to UP" "${back%% *}" "UP"
check "up-count back to 3" "$($H up-count all_handlers)" "3"

echo "== 3b. one call drains and restores every backend a server is in =="
$H drain all_handlers,db_handlers/patty >/dev/null 2>&1
check "drained in both"   "$($H states all_handlers,db_handlers/patty)" "all_handlers=DRAIN db_handlers=DRAIN "
$H wait-empty all_handlers,db_handlers/patty 30 >/dev/null 2>&1; check "one wait covers both" "$?" "0"
$H maint all_handlers,db_handlers/patty >/dev/null 2>&1
check "maint in both"     "$($H states all_handlers,db_handlers/patty)" "all_handlers=MAINT db_handlers=MAINT "
$H ready all_handlers,db_handlers/patty >/dev/null 2>&1
$H wait-up all_handlers,db_handlers/patty 60 >/dev/null 2>&1; check "one wait-up covers both" "$?" "0"
check "least-up across both" "$($H least-up all_handlers,db_handlers)" "3"

echo "== 3c. a backend that does not hold the server is refused =="
$H drain all_handlers,nosuch/patty >/tmp/e4 2>&1; check "exit non-zero" "$([ $? -ne 0 ] && echo yes)" "yes"
check "names the set" "$(grep -c 'is not in every one of' /tmp/e4)" "1"
check "patty untouched" "$($H state all_handlers/patty | cut -d' ' -f1)" "UP"
$H state all_handlers,db_handlers/patty >/tmp/e5 2>&1; check "state refuses a list" "$([ $? -ne 0 ] && echo yes)" "yes"

echo "== 3d. a transitional status does not shift the session count =="
# rick has `fall 10`, so HAProxy reports "UP 1/10" for it: the counter must not be read as scur.
rick_state=$($H state all_handlers/rick)
check "state keeps the counter"  "$(printf '%s' "$rick_state" | grep -cE '^UP')" "1"
check "sessions is a number"     "$(printf '%s' "$($H sessions all_handlers/rick)" | grep -cE '^[0-9]+$')" "1"
$H wait-empty all_handlers/rick 6 >/dev/null 2>&1
check "wait-empty sees it empty" "$?" "0"

echo "== 4. a backup server counts as capacity =="
$H maint all_handlers/patty >/dev/null 2>&1
$H maint all_handlers/selma >/dev/null 2>&1
check "only rick left"     "$($H up-count all_handlers)" "1"
$H ready all_handlers/patty >/dev/null 2>&1; $H ready all_handlers/selma >/dev/null 2>&1

echo "== 5. wait-empty gives up rather than hanging =="
$H drain all_handlers/selma >/dev/null 2>&1
start=$SECONDS
$H wait-empty all_handlers/selma 4 >/dev/null 2>&1
check "wait-empty on an idle server is instant" "$([ $((SECONDS-start)) -lt 4 ] && echo yes)" "yes"
$H ready all_handlers/selma >/dev/null 2>&1

echo "== 6. no socket is a clear error =="
HAPROXY_SOCKET=/run/haproxy/absent.sock $H state all_handlers/patty >/tmp/e3 2>&1
check "exit non-zero" "$([ $? -ne 0 ] && echo yes)" "yes"
check "names the socket" "$(grep -c 'no HAProxy socket' /tmp/e3)" "1"

# The fake backends hold stdout open; without this a pipe on the outside never sees EOF.
pkill -f 'TCP-LISTEN' >/dev/null 2>&1
kill "$(jobs -p)" >/dev/null 2>&1
summary

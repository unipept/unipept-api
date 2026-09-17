#!/usr/bin/env bash
# Drives the swap with a deliberate stop between its two steps, which is what being killed there
# amounts to. Run as the service user.
set -uo pipefail
B=/opt/unipept-api/bin/unipept-api
install -m 0755 /tmp/swapsrc "$B.new"
ln -f "$B" "$B.previous" 2>/dev/null || cp -f "$B" "$B.previous"
sleep 2
mv "$B.new" "$B"

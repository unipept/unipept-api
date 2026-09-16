#!/usr/bin/env bash
#
# Sends what arrives on port 80 to the port the API actually listens on. Run as root by
# unipept-api-ports.service, at boot and whenever the rules need re-applying.
#
# The API runs as a systemd *user* unit, and a user manager holds no capability to grant, so the
# service cannot bind 80 itself. The rewrite happens here instead, inside the host: the packet is
# addressed to port 80 on the wire, so HAProxy keeps its `server ...:80` lines and the campus
# firewall needs nothing.
#
# Everything lives in a chain of its own, so re-applying cannot duplicate a rule and nothing else on
# the host is disturbed.

set -euo pipefail

readonly CHAIN=UNIPEPT_API
readonly ENV_FILE=/opt/unipept-api/etc/unipept-api.env

port=$(sed -n 's/^PORT=//p' "$ENV_FILE" | tail -1)
case ${port:-} in
    '' | *[!0-9]*)
        echo "PORT in ${ENV_FILE} is '${port:-unset}', which is not a number" >&2
        exit 1
        ;;
esac

# Fresh, whether or not the chain already existed.
iptables -t nat -N "$CHAIN" 2>/dev/null || iptables -t nat -F "$CHAIN"
iptables -N "$CHAIN" 2>/dev/null || iptables -F "$CHAIN"

# Appended, never inserted. On these hosts INPUT begins with Tailscale's own chain, and putting
# ours ahead of its anti-spoofing rules to gain nothing is how someone loses their SSH session.
iptables -t nat -C PREROUTING -j "$CHAIN" >/dev/null 2>&1 || iptables -t nat -A PREROUTING -j "$CHAIN"
iptables -t nat -C OUTPUT -j "$CHAIN" >/dev/null 2>&1 || iptables -t nat -A OUTPUT -j "$CHAIN"
iptables -C INPUT -j "$CHAIN" >/dev/null 2>&1 || iptables -A INPUT -j "$CHAIN"

# PREROUTING catches what arrives from the load balancer. OUTPUT catches what this host sends to
# itself, which is what lets `deploy.sh check` prove the redirect works without being root.
#
# --dst-type LOCAL is what keeps OUTPUT to *this host*. Without it the same rule rewrites every
# outbound connection to port 80 anywhere, so `apt-get update` over http, or any other plain-HTTP
# call this server makes, is answered by the API instead of the host it asked for.
iptables -t nat -A "$CHAIN" -p tcp --dport 80 -m addrtype --dst-type LOCAL -j REDIRECT --to-port "$port"

# The host talking to its own service, which is how deploy.sh checks health and how anything else
# here would reach it. Without this the rule below refuses it: a loopback connection is not DNATed,
# so it looks exactly like someone dialling the service's own port from outside.
iptables -A "$CHAIN" -i lo -j ACCEPT

# Only connections the redirect created. Without this the service would also answer on its own port
# to anything that can route here, which is a path that does not exist today.
#
# It does not cover the tailnet: Tailscale's chain runs first and accepts what arrives on
# tailscale0, so that traffic never reaches here. That is left alone deliberately — the tailnet is
# authenticated, and reordering around Tailscale to close it would risk more than it gains.
iptables -A "$CHAIN" -p tcp --dport "$port" -m conntrack --ctstate DNAT -j ACCEPT
iptables -A "$CHAIN" -p tcp --dport "$port" -j REJECT

echo "port 80 now reaches ${port}"

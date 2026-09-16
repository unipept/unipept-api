# Deploy

Everything needed to run the API on a production host. The wiki pages link here rather than
holding copies.

## Layout

| Path | What it is |
| --- | --- |
| `lib.sh` | shared shell, sourced by the scripts |
| `rollout.sh` | updates every server one at a time. Run on the load balancer |
| `servers.example.conf` | the inventory. Copy to `servers.conf` on the load balancer |
| `rollout.conf.example` | load balancer settings. Copy to `rollout.conf` |
| `loadbalancer/haproxy.sh` | the HAProxy runtime API: drain, ready, wait |
| `server/unipept-api.service` | systemd **user** unit, installed at `~unipept/.config/systemd/user/` |
| `server/unipept-api.env.example` | per-host configuration, installed at `/opt/unipept-api/etc/unipept-api.env` |
| `server/install.sh` | prepares a host once. The only step that needs root |
| `server/deploy.sh` | installs or puts back a binary on one server |

## Deploying

One server, on that server, as the `unipept` user:

```bash
/opt/unipept-api/lib/deploy.sh check                 # is this host ready?
/opt/unipept-api/lib/deploy.sh deploy --version v2.6.0
/opt/unipept-api/lib/deploy.sh rollback
```

Every server, from the load balancer:

```bash
./rollout.sh --version v2.6.0 --dry-run
./rollout.sh --version v2.6.0
./rollout.sh status                                  # after a run that stopped part way
```

A rollout runs in four phases, and the order is the point:

1. **Claim** — one rollout at a time, and download the release once for the whole fleet.
2. **Check** — every server, before any of them is touched: HAProxy state, both health routes, and
   `deploy.sh check` with the real binary staged, so a build that cannot run on a host is found
   before another host is drained. Every problem in the fleet is reported together.
3. **Update** — one server at a time. It leaves the pool, takes the binary, and returns to the pool
   before the next one starts.
4. **Finish** — always: staging cleared on every server, one journal line per server, and an email if
   a server needs attention.

**At most one server is ever outside the pool.** A failure stops the run, so the servers after it are
never attempted. The capacity guard refuses to drain the last server that is UP — a backup counts as
capacity — and `--allow-downtime` is how an operator overrides that deliberately.

A server whose deploy failed is rolled back and, if it comes back healthy, returned to the pool: it is
serving a version that was known good, so holding it out would cost capacity for nothing. Only a
server that cannot be routed to is left out, and that is the case that sends mail.

## Reading what happened

```bash
journalctl -t unipept-rollout            # who deployed what, when
./rollout.sh status                       # the fleet as it is now
```

## One caution

`set server ... state` is a runtime change and this HAProxy has no `server-state-file`, so **reloading
or restarting HAProxy during a rollout returns a draining server to rotation** mid-restart. The lock
file is the signal that a run is in progress.

## A user unit, so a deploy needs no privilege## A user unit, so a deploy needs no privilege

The service runs as a systemd user unit owned by the `unipept` user, started at boot by lingering.
That user owns `/opt/unipept-api`, so it replaces the binary and restarts its own unit without root,
sudo or a polkit rule. Only the first install needs root.

A user manager holds no capability to grant, so the service cannot bind port 80 itself — measured on
systemd 255, `AmbientCapabilities=CAP_NET_BIND_SERVICE` fails with *"Failed to apply ambient
capabilities (before UID change): Operation not permitted"* and the unit exits 218.

So the service listens above 1024 and a netfilter rule sends what arrives on 80 to it.
`unipept-api-ports.service` holds that rule and install.sh puts it there. **Nothing on the network
changes**: the packet is addressed to port 80 on the wire and is rewritten inside the host, so
HAProxy keeps its `server ...:80` lines.

## The storage backend is a host property

The index has one implementation holding owned memory and one borrowing a memory mapping, and
which one a binary uses is decided when it is compiled. A release therefore publishes several
builds, and `VARIANT` in the environment file names the one this host takes. A restart cannot
correct it. See "Choosing a storage backend" in the top-level README.

## Logs

The unit sets no `StandardOutput` or `StandardError`, so the API's own log lines reach the journal.
As the service user:

```bash
journalctl --user -u unipept-api -f
```

As root, since a user unit's entries are not under `-u`:

```bash
journalctl -f _SYSTEMD_USER_UNIT=unipept-api.service
```

`RUST_LOG` in the environment file sets the filter.

One request is one line, except a health check that passes. A load balancer polling `/health` and
`/health/database` on every server every couple of seconds is tens of thousands of lines a day that
nobody reads, so a probe is logged only when it fails.

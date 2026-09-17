# Deploy

Everything needed to run the API on a production host. The wiki pages link here rather than
holding copies.

## Layout

| Path | What it is |
| --- | --- |
| `server/unipept-api.service` | systemd **user** unit, installed at `~unipept/.config/systemd/user/` |
| `server/unipept-api.env.example` | per-host configuration, installed at `/opt/unipept-api/etc/unipept-api.env` |

## A user unit, so a deploy needs no privilege

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

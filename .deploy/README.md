# Deploy

Everything needed to run the API on a production host. The wiki pages link here rather than
holding copies.

## Layout

| Path | What it is |
| --- | --- |
| `server/unipept-api.service` | systemd **user** unit, installed at `~unipept/.config/systemd/user/` |
| `server/unipept-api.env.example` | per-host configuration, installed at `/opt/unipept-api/etc/unipept-api.env` |

## A user unit, so a deploy needs no privilege

The service runs as a systemd user unit owned by the `unipept` user, started at boot by
lingering. That user owns `/opt/unipept-api`, so it replaces the binary and restarts its own unit
without root, sudo or a polkit rule. Only the first install needs root.

This is why the API listens above port 1024: binding a lower port needs a capability that an
unprivileged unit cannot hold. HAProxy's `server` lines must name the port in the environment file.

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

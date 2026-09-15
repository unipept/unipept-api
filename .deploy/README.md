# Deploy

Everything needed to run the API on a production host. The wiki pages link here rather than
holding copies.

## Layout

| Path | What it is |
| --- | --- |
| `server/unipept-api.service` | systemd unit, installed at `/etc/systemd/system/` |
| `server/unipept-api.env.example` | per-host configuration, installed at `/etc/unipept-api/unipept-api.env` |

## The storage backend is a host property

The index has one implementation holding owned memory and one borrowing a memory mapping, and
which one a binary uses is decided when it is compiled. A release therefore publishes several
builds, and `VARIANT` in the environment file names the one this host takes. A restart cannot
correct it. See "Choosing a storage backend" in the top-level README.

## Logs

The unit sets no `StandardOutput` or `StandardError`, so the API's own log lines reach journald:

```bash
journalctl -u unipept-api -f
```

`RUST_LOG` in the environment file sets the filter.

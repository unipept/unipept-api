# Deploy tests

The scripts in `.deploy/` have no other test coverage, and what they get wrong is the kind of thing
a mock gets wrong too. So these suites run against a real systemd and a real HAProxy, both at the
versions production runs.

```bash
.deploy/tests/run-tests.sh            # every suite
.deploy/tests/run-tests.sh server     # install.sh and deploy.sh
```

Docker is the only requirement. The server suite runs its container `--privileged` so systemd can
boot.

## What each suite covers

| Suite | Runs against | Covers |
| --- | --- | --- |
| `server/suite.sh` | systemd 255 on Ubuntu 24.04, with a user manager and lingering | the first install, a deploy with no privilege, `check` and each failure it reports, the memory arm per variant, the binary swap under signal, rollback, and the interrupt paths |

## Why containers rather than mocks

Every bug these suites caught was one a mock would have hidden:

* `systemctl --user` needs `XDG_RUNTIME_DIR`, which only exists when lingering is really enabled.
* `AmbientCapabilities=` makes a *user* unit fail to start, with exit 218. Nothing in the
  documentation says so.
* A hard link is refused when the caller does not own the file, because `fs.protected_hardlinks` is
  on by default.

## Writing a case

`lib.sh` holds the assertions:

```bash
check "what it should do"  "$(what it did)"  "what it should have been"
check_true "the file is there" test -f /some/path
section "a group of cases"
```

`check` never stops the suite, so one run reports everything that is wrong rather than the first
thing. `summary` at the end sets the exit status.

Two rules worth keeping:

1. **Make a new case fail first.** Every case here was run against the unfixed code and observed to
   fail. A case that has never failed is a case that may be asserting nothing — the swap test is the
   clearest example, and it drives the old two-move pair to prove the point.
2. **Reset what you changed.** A case that leaves the host altered makes the next one fail, and the
   failure then points at the wrong thing.

## Not in CI

These do not run in GitHub Actions yet. The server suite needs a privileged container with the host's
cgroups mounted, and whether that works on a hosted runner is untested — so it is a deliberate gap
rather than an oversight. `ci.yml` runs `shellcheck` over every script here, which catches the class
of mistake that does not need a container.

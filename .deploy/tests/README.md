# Deploy tests

The scripts in `.deploy/` have no other test coverage, and what they get wrong is the kind of thing
a mock gets wrong too. So these suites run against a real systemd and a real HAProxy, both at the
versions production runs.

```bash
.deploy/tests/run-tests.sh            # every suite, about seven minutes
.deploy/tests/run-tests.sh server     # install.sh and deploy.sh
.deploy/tests/run-tests.sh haproxy    # haproxy.sh
.deploy/tests/run-tests.sh rollout    # rollout.sh
```

Docker is the only requirement. The server suite runs its container `--privileged` so systemd can
boot; nothing else needs it.

## What each suite covers

| Suite | Runs against | Covers |
| --- | --- | --- |
| `server/suite.sh` | systemd 255 on Ubuntu 24.04, with a user manager and lingering | the first install, a deploy with no privilege, `check` and each failure it reports, the memory arm per variant, the binary swap under signal, rollback, the interrupt paths, and the per-host `READY_TIMEOUT` with the unit watch that makes a long one safe |
| `loadbalancer/haproxy-suite.sh` | HAProxy 2.8 | reading `show stat` by field name, multi-backend targets, the drain cycle, a transitional `UP 1/100` status, backups |
| `loadbalancer/rollout-suite.sh` | HAProxy 2.8 with two backends over three servers | the four phases, the lock, ordering, preflight, each failure-resolution path, cleanup, the record, an interrupt during an install, and the deadline each server asks for |

## Why containers rather than mocks

Every bug these suites caught was one a mock would have hidden:

* `systemctl --user` needs `XDG_RUNTIME_DIR`, which only exists when lingering is really enabled.
* `AmbientCapabilities=` makes a *user* unit fail to start, with exit 218. Nothing in the
  documentation says so.
* HAProxy's `status` field is not one word: it reads `UP 1/100` while a server is being checked back
  in, which shifted every field after it.
* A hard link is refused when the caller does not own the file, because `fs.protected_hardlinks` is
  on by default.
* `Type=exec` reports the exec, not readiness, so systemd calls a binary that exits at once started
  and a health wait watching only the port sits there until the deadline.

## Writing a case

`lib.sh` holds the assertions:

```bash
check "what it should do"  "$(what it did)"  "what it should have been"
check_true "the file is there" test -f /some/path
section "a group of cases"
```

`check` never stops the suite, so one run reports everything that is wrong rather than the first
thing. `summary` at the end sets the exit status and repeats every failure, because a run is long
enough that the first one has scrolled away by the time it ends.

**Head every group with `section`, not `echo`.** A failure prints the section it is in, which is
what makes `FAIL exit non-zero` identify one case rather than one of twelve. A heading printed with
`echo` records nothing, so its failures would name no case.

Two rules worth keeping:

1. **Make a new case fail first.** Every case here was run against the unfixed code and observed to
   fail. A case that has never failed is a case that may be asserting nothing — the swap test is the
   clearest example, and it drives the old two-move pair to prove the point.
2. **Reset what you changed.** `reset_fleet` in the rollout suite puts every server back in the pool
   before a case starts. Without it one case's leftovers fail the next, and the failure points at
   the wrong thing.

## Not in CI

These do not run in GitHub Actions yet. The server suite needs a privileged container with the
host's cgroups mounted, and whether that works on a hosted runner is untested — so it is a
deliberate gap rather than an oversight. `ci.yml` runs `shellcheck` over every script here, which
catches the class of mistake that does not need a container.
